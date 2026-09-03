use actix_web::{App, HttpServer, middleware::from_fn, web};
use std::{collections::HashMap, sync::{Mutex, mpsc::{self, Sender}}, thread, };

use crate::{
    TokenTransactions::{DepositToken, GetAllTokens, GetTokenBalance}, UserBalanceTx::{GetBalance, Onramp}, middleware::user::user_auth, routes::user::{
        balance, cancle, deposit, login, onramp, orders, signup
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

enum TokenTransactions {
    DepositToken(i32, String, i32),
    GetTokenBalance(i32, String, futures::channel::oneshot::Sender<i32>),
    GetAllTokens(i32, futures::channel::oneshot::Sender<HashMap<String, i32>>)
}
struct AppState {
    users: Mutex<Vec<User>>,
    user_index: Mutex<i32>,
    usd_balance: Sender<UserBalanceTx>,
    token_balance: Sender<TokenTransactions>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let (user_balance_tx, user_balance_rx) =  mpsc::channel();
    let (token_balance_tx, token_balance_rx) = mpsc::channel();

    let app_state = web::Data::new(AppState {
        users: Mutex::new(vec![]),
        user_index: Mutex::new(0),
        usd_balance: user_balance_tx,
        token_balance: token_balance_tx
    });

    thread::spawn( move ||  {
        let mut balances: HashMap<i32, i32> = HashMap::new();

        while let message = user_balance_rx.recv().unwrap() {

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

    thread::spawn(move || {
        let mut token_balances: HashMap<i32, HashMap<String, i32>> = HashMap::new();

        while let message = token_balance_rx.recv().unwrap() {
            match message {
                DepositToken(user_id, symbol, qty) => {

                    let mut existing_user_tokens = token_balances.entry(user_id).or_insert(HashMap::new());
                    let token_balance = existing_user_tokens.get(&symbol).unwrap_or(&0);

                    existing_user_tokens.insert(symbol, *token_balance + qty);
                }  
                GetTokenBalance(user_id, symbol, rx   ) => {

                    let user_token = token_balances.entry(user_id).or_insert(HashMap::new());
                    let user_token_balance = user_token.get(&symbol).unwrap_or(&0);

                    rx.send(*user_token_balance);
                }
                GetAllTokens(user_id, rv) => {
                    rv.send(token_balances.entry(user_id).or_insert(HashMap::new()).clone());
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
                .service(deposit)
                .service(orders)
                .service(cancle)
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
