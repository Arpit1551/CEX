use actix_web::{
    App, HttpResponse, HttpServer, Responder, post,
    web::{self, Json},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Mutex};

#[derive(Deserialize, Serialize)]
struct SignupInput {
    username: String,
    password: String,
}

#[derive(Deserialize, Serialize)]
struct SigninInput {
    username: String,
    password: String,
}

#[derive(Deserialize, Serialize)]
struct User {
    id: i32,
    username: String,
    password: String,
}

#[derive(Deserialize, Serialize)]
struct SignupResponse {
    message: String,
    token: String
}

#[derive(Deserialize, Serialize)]
struct SigninResponse {
    message: String,
    token: String
}

struct AppState {
    users: Mutex<Vec<User>>,
    user_index: Mutex<i32>,
    usd_balance: Mutex<HashMap<i32, i32>>,
    token_balance: Mutex<HashMap<i32, HashMap<String, u32>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32,
    pub exp: usize,
}

#[post("/signup")]
async fn signup(app_state: web::Data<AppState>, user_info: Json<SignupInput>) -> impl Responder {
    let mut users = app_state.users.lock().unwrap();
    let mut user_index = app_state.user_index.lock().unwrap();

    let user_found = users.iter().find(|u| u.username == user_info.username);

    if user_found.is_none() {
        let mut usd_balance = app_state.usd_balance.lock().unwrap();
        let mut token_balance = app_state.token_balance.lock().unwrap();
        *user_index = *user_index + 1;

        let new_user = User {
            id: user_index.clone(),
            username: user_info.username.clone(),
            password: user_info.password.clone(),
        };

        users.push(new_user);

        usd_balance.insert(user_index.clone(), 0);
        token_balance.insert(user_index.clone(), HashMap::new());

        let exp = Utc::now()
            .checked_add_signed(Duration::hours(24))
            .expect("valid timestamp")
            .timestamp() as usize;

        let claims = Claims {
            sub: user_index.clone(),
            exp,
        };

        drop(users);
        drop(user_index);
        drop(usd_balance);
        drop(token_balance);

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret("secret".as_ref()),
        )
        .unwrap();
    
        return HttpResponse::Ok().json(SignupResponse {
            message: String::from("Signup successfull!"),
            token: token
        });
    } else {
        return HttpResponse::Conflict().json(SignupResponse {
            message: String::from("User already exist!"),
            token: String::from("")
        });
    }
}

#[post("/login")]
async fn login(app_state: web::Data<AppState>, user_info: Json<SigninInput>) -> impl Responder {
    let users = app_state.users.lock().unwrap();

    let user_found = users
        .iter()
        .find(|u| u.username == user_info.username && u.password == user_info.password);

    if user_found.is_none() {
        return HttpResponse::Unauthorized().json(SigninResponse {
            message: String::from("Incorrect credentials"),
            token: String::from("")
        });
    };

    let exp = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let user_index = app_state.user_index.lock().unwrap();
    let claims = Claims {
        sub: user_index.clone(),
        exp,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret("secret".as_ref()),
    )
    .unwrap();

    HttpResponse::Ok().json(SigninResponse { 
        message: String::from("Login successfull!"),
        token: token 
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let app_state = web::Data::new(AppState {
        users: Mutex::new(vec![]),
        user_index: Mutex::new(0),
        usd_balance: Mutex::new(HashMap::new()),
        token_balance: Mutex::new(HashMap::new()),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .service(signup)
            .service(login)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
