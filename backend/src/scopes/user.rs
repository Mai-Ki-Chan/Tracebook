use crate::extractors::auth_token::AuthToken;
use actix_web::{web, HttpResponse, Scope};
use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, errors::Error, Algorithm, DecodingKey, EncodingKey, Header, TokenData,
    Validation,
};
use rand::random;
use serde::{Deserialize, Serialize};
use surrealdb::{Datastore, Session};

pub fn user_scope() -> Scope {
    web::scope("/user")
        .route("/encode-token", web::post().to(encode_token))
        .route("decode-token", web::post().to(decode_token))
        .route("protected", web::get().to(protected))
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub id: u128,
    pub exp: usize,
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
struct Response {
    message: String,
}

#[derive(Serialize, Deserialize)]
struct EncodeResponse {
    message: String,
    token: String,
    info: String,
}

#[derive(Serialize, Deserialize)]
struct DecodeResponse {
    message: String,
    id: u128,
    username: String,
    password: String,
}

#[derive(Serialize, Deserialize)]
struct Info {
    password: String,
    username: String,
}

async fn encode_token(body: web::Json<Info>, secret: web::Data<String>) -> HttpResponse {
    let id = random::<u128>();
    let exp: usize = (Utc::now() + Duration::days(365)).timestamp() as usize;
    println!("{} {}", body.username, body.password);
    let db = DB::use_db("memory", ("ses", "db"));
    let create_user = DB::create_user(db.await, id, body.username.clone(), body.password.clone())
        .await
        .unwrap();
    let claim: Claims = Claims {
        id,
        exp,
        username: body.username.clone(),
        password: body.password.clone(),
    };
    let token: String = encode(
        &Header::default(),
        &claim,
        &EncodingKey::from_secret(secret.as_str().as_ref()),
    )
    .unwrap();
    let select = DB::select_user(DB::use_db("memory", ("ses", "db")).await)
        .await
        .unwrap();
    println!("{select}");
    HttpResponse::Ok().json(EncodeResponse {
        message: String::from("success"),
        token,
        info: create_user,
    })
}

#[derive(Serialize, Deserialize)]
struct DecodeBody {
    token: String,
}

async fn decode_token(body: web::Json<DecodeBody>, secret: web::Data<String>) -> HttpResponse {
    let decoded: Result<TokenData<Claims>, Error> = decode::<Claims>(
        &body.token,
        &DecodingKey::from_secret(secret.as_str().as_ref()),
        &Validation::new(Algorithm::HS256),
    );
    match decoded {
        Ok(token) => HttpResponse::Ok().json(DecodeResponse {
            message: "Authed".to_string(),
            id: token.claims.id,
            username: token.claims.username,
            password: token.claims.password,
        }),
        Err(e) => HttpResponse::BadRequest().json(Response {
            message: e.to_string(),
        }),
    }
}

async fn protected(aut_token: AuthToken) -> HttpResponse {
    println!("{}", aut_token.id);
    HttpResponse::Ok().json(Response {
        message: String::from("protected"),
    })
}

struct DB {
    db: (Datastore, Session),
}

impl DB {
    async fn use_db(ds: &str, ses: (&str, &str)) -> Self {
        Self {
            db: (
                Datastore::new(ds).await.unwrap(),
                Session::for_db(ses.0, ses.1),
            ),
        }
    }

    async fn create_user(
        self,
        id: u128,
        username: String,
        password: String,
    ) -> Result<String, String> {
        let (ds, ses) = &self.db;
        let sql_cmd = format!(
            "CREATE user:{} SET username = {}, password = {}",
            id, username, password
        );
        let exec = ds.execute(&sql_cmd, ses, None, false).await?;
        Ok(format!("{exec:?}"))
    }

    async fn select_user(self) -> Result<String, String> {
        let (ds, ses) = &self.db;
        let sql_cmd = "SELECT * FROM user;";
        let exec = ds.execute(&sql_cmd, ses, None, false).await?;
        Ok(format!("{exec:?}"))
    }
}
