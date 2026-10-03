use actix_web::{App, HttpServer, middleware::from_fn, web};
use jsonwebtoken::signature::rand_core::le;
use serde::de::value;
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    process,
    sync::{
        Mutex,
        mpsc::{self, Sender},
    },
    thread,
};

use crate::{
    OrderBook::AddOrder,
    TokenTransactions::{DepositToken, GetAllTokens, GetTokenBalance},
    UserBalanceTx::{GetBalance, LockBalance, Onramp, UnlockBalance},
    middleware::user::user_auth,
    routes::user::{balance, cancle, deposit, login, onramp, orders, signup},
    types::user::User,
};

pub mod helper;
pub mod middleware;
pub mod routes;
pub mod types;

enum UserBalanceTx {
    Onramp(i32, i32),
    GetBalance(i32, futures::channel::oneshot::Sender<i32>),
    LockBalance(i32, i32),
    UnlockBalance(i32, i32),
}

enum TokenTransactions {
    DepositToken(i32, String, i32),
    GetTokenBalance(i32, String, futures::channel::oneshot::Sender<i32>),
    GetAllTokens(i32, futures::channel::oneshot::Sender<HashMap<String, i32>>),
}

enum OrderBook {
    AddOrder(i32, String, i32, i32, Sender<Vec<Fill>>),
}

#[derive(Debug)]
enum FillsTypes {
    OrderBookEntry,
    OrderCompleted,
}

struct AppState {
    users: Mutex<Vec<User>>,
    user_index: Mutex<i32>,
    usd_balance: Sender<UserBalanceTx>,
    token_balance: Sender<TokenTransactions>,
    order_book: Sender<OrderBook>,
}

#[derive(Debug, Clone, Copy)]
struct OrderBookEntry {
    user_id: i32,
    price: i32,
    qty: i32,
    filled_qty: i32,
    order_id: i32,
}

#[derive(Debug)]
struct Fill {
    header: FillsTypes,
    user_id: Option<i32>,
    seller: Option<i32>,
    buyer: Option<i32>,
    qty: i32,
    price: i32,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let (user_balance_tx, user_balance_rx) = mpsc::channel();
    let (token_balance_tx, token_balance_rx) = mpsc::channel();
    let (order_book_tx, order_book_rv) = mpsc::channel();

    let app_state = web::Data::new(AppState {
        users: Mutex::new(vec![]),
        user_index: Mutex::new(0),
        usd_balance: user_balance_tx,
        token_balance: token_balance_tx,
        order_book: order_book_tx,
    });

    thread::spawn(move || {
        let mut balances: HashMap<i32, i32> = HashMap::new();
        let mut locked_amount: HashMap<i32, i32> = HashMap::new();

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

                LockBalance(user_id, amount) => {
                    let user_locked_balance = locked_amount.get(&user_id).unwrap_or(&0);
                    let user_balance = balances.get(&user_id).unwrap_or(&0);

                    locked_amount.insert(user_id, *user_locked_balance + amount);
                    balances.insert(user_id, *user_balance - amount);
                }

                UnlockBalance(user_id, amount) => {
                    let user_locked_balance = locked_amount.get(&user_id).unwrap_or(&0);
                    let user_balance = balances.get(&user_id).unwrap_or(&0);

                    locked_amount.insert(user_id, *user_locked_balance - amount);
                    balances.insert(user_id, *user_balance + amount);
                }
            }
        }
    });

    thread::spawn(move || {
        let mut token_balances: HashMap<i32, HashMap<String, i32>> = HashMap::new();

        while let message = token_balance_rx.recv().unwrap() {
            match message {
                DepositToken(user_id, symbol, qty) => {
                    let mut existing_user_tokens =
                        token_balances.entry(user_id).or_insert(HashMap::new());
                    let token_balance = existing_user_tokens.get(&symbol).unwrap_or(&0);

                    existing_user_tokens.insert(symbol, *token_balance + qty);
                }
                GetTokenBalance(user_id, symbol, rx) => {
                    let user_token = token_balances.entry(user_id).or_insert(HashMap::new());
                    let user_token_balance = user_token.get(&symbol).unwrap_or(&0);

                    rx.send(*user_token_balance);
                }
                GetAllTokens(user_id, rv) => {
                    rv.send(
                        token_balances
                            .entry(user_id)
                            .or_insert(HashMap::new())
                            .clone(),
                    );
                }
            }
        }
    });

    thread::spawn(move || {
        let symbol = "SOL";
        let mut bids: BTreeMap<i32, VecDeque<OrderBookEntry>> = BTreeMap::new();
        let mut ask: BTreeMap<i32, VecDeque<OrderBookEntry>> = BTreeMap::new();

        while let message = order_book_rv.recv().unwrap() {
            match message {
                AddOrder(user_id, msg_type, qty, price, order_book_response_tx) => {
                    let mut fills: Vec<Fill> = vec![];
                    let mut unfilled_qty = qty;

                    if msg_type == "bid" {
                        for (&level_price, level) in ask.iter_mut() {
                            if level_price > price || unfilled_qty == 0 {
                                break;
                            }
                            while unfilled_qty > 0 {
                                let Some(maker) = level.front_mut() else {
                                    break;
                                };
                                let take = (maker.qty - maker.filled_qty).min(unfilled_qty);

                                if take > 0 {
                                    maker.filled_qty += take;
                                    unfilled_qty -= take;
                                    fills.push(Fill {
                                        header: FillsTypes::OrderCompleted,
                                        buyer: Some(user_id),
                                        seller: Some(maker.user_id),
                                        price: level_price,
                                        qty: take,
                                        user_id: None,
                                    });
                                }
                                if maker.filled_qty >= maker.qty {
                                    level.pop_front();
                                }
                            }
                        }
                        ask.retain(|_, level| !level.is_empty());

                        if unfilled_qty > 0 {
                            bids.entry(price).or_default().push_back(OrderBookEntry {
                                user_id,
                                price,
                                qty: unfilled_qty,
                                filled_qty: 0,
                                order_id: rand::random_range(0..=999),
                            });
                            fills.push(Fill {
                                header: FillsTypes::OrderBookEntry,
                                user_id: Some(user_id),
                                seller: None,
                                buyer: None,
                                qty: unfilled_qty,
                                price,
                            });
                        }
                    } else if msg_type == "ask" {
                        for (&level_price, level) in bids.iter_mut().rev() {
                            if level_price < price || unfilled_qty == 0 {
                                break;
                            }
                            while unfilled_qty > 0 {
                                let Some(maker) = level.front_mut() else {
                                    break;
                                };
                                let take = (maker.qty - maker.filled_qty).min(unfilled_qty);

                                if take > 0 {
                                    maker.filled_qty += take;
                                    unfilled_qty -= take;
                                    fills.push(Fill {
                                        header: FillsTypes::OrderCompleted,
                                        buyer: Some(maker.user_id),
                                        seller: Some(user_id),
                                        price: level_price,
                                        qty: take,
                                        user_id: None,
                                    });
                                }
                                if maker.filled_qty >= maker.qty {
                                    level.pop_front();
                                }
                            }
                        }
                        bids.retain(|_, level| !level.is_empty());

                        if unfilled_qty > 0 {
                            ask.entry(price).or_default().push_back(OrderBookEntry {
                                user_id,
                                price,
                                qty: unfilled_qty,
                                filled_qty: 0,
                                order_id: rand::random_range(0..=999),
                            });
                            fills.push(Fill {
                                header: FillsTypes::OrderBookEntry,
                                user_id: Some(user_id),
                                seller: None,
                                buyer: None,
                                qty: unfilled_qty,
                                price,
                            });
                        }
                    }

                    println!("Bids -> {:?}", bids);
                    println!("Asks -> {:?}", ask);
                    let _ = order_book_response_tx.send(fills);
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
                    .service(cancle),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
