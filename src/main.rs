use actix_web::{App, HttpServer, middleware::from_fn, web};
use std::{collections::HashMap, sync::Mutex};

use crate::{routes::user::{login, signup, balance}, types::user::User, middleware::user::user_auth};

pub mod types;
pub mod routes;
pub mod middleware;
pub mod helper;

struct AppState {
    users: Mutex<Vec<User>>,
    user_index: Mutex<i32>,
    usd_balance: Mutex<HashMap<i32, i32>>,
    token_balance: Mutex<HashMap<i32, HashMap<String, u32>>>,
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
            .service(
                web::scope("/protected")
                .wrap(from_fn(user_auth))
                .service(balance)
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
