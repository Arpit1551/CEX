use actix_web::{App, HttpServer, middleware::from_fn, web};
use std::{collections::HashMap, sync::{Mutex, mpsc::{self, Sender}}, thread, };

use crate::{
    UserBalanceTx::{GetBalance, Onramp}, middleware::user::user_auth, routes::user::{
        balance, cancle, login, onramp, orders, signup
    }, types::user::User
};

pub mod types;
pub mod routes;
pub mod middleware;
pub mod helper;

enum UserBalanceTx {
    Onramp(i32, i32),
    GetBalance(i32, futures::channel::oneshot::Sender<i32>)
}
struct AppState {
    users: Mutex<Vec<User>>,
    user_index: Mutex<i32>,
    usd_balance: Sender<UserBalanceTx>,
    token_balance: Mutex<HashMap<i32, HashMap<String, u32>>>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let (tx, rx) =  mpsc::channel();
    let app_state = web::Data::new(AppState {
        users: Mutex::new(vec![]),
        user_index: Mutex::new(0),
        usd_balance: tx,
        token_balance: Mutex::new(HashMap::new()),
    });

    thread::spawn( move ||  {
        let mut balances: HashMap<i32, i32> = HashMap::new();

        while let message = rx.recv().unwrap() {

            match message {
                Onramp(user_id, qty) => {
                    let existing_balance = balances.get(&user_id).unwrap_or(&0);
                    balances.insert(user_id, qty + existing_balance);
                }
                GetBalance(user_id, tx) => {
                    let user_balance = *balances.get(&user_id).unwrap_or(&0);
                    tx.send(user_balance);
                }
            }
        }
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
                .service(onramp)
                .service(orders)
                .service(cancle)
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
