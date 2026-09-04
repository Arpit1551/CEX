use actix_web::{App, HttpServer, middleware::from_fn, web};
use std::{collections::{BTreeMap, HashMap, VecDeque}, process, sync::{Mutex, mpsc::{self, Sender}}, thread, };
use rand::Rng;  

use crate::{
    OrderBook::AddOrder, TokenTransactions::{DepositToken, GetAllTokens, GetTokenBalance}, UserBalanceTx::{GetBalance, Onramp}, middleware::user::user_auth, routes::user::{
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

enum OrderBook {
    CreateOrder(String),
    AddOrder(i32, String, i32, i32)
}

enum FillsTypes {
    OrderBookEntry,
    OrderCompleted
}

struct AppState {
    users: Mutex<Vec<User>>,
    user_index: Mutex<i32>,
    usd_balance: Sender<UserBalanceTx>,
    token_balance: Sender<TokenTransactions>,
    order_book: Sender<OrderBook>
}

struct OrderBookEntry {
    user_id: i32,
    price: i32,
    qty: i32,
    filled_qty: i32,
    order_id: i32
}

struct Fills {
    header: FillsTypes,
    user_id: Option<i32>,
    seller: Option<i32>,
    buyer: Option<i32>,
    qty: i32,
    price: i32
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    let (user_balance_tx, user_balance_rx) =  mpsc::channel();
    let (token_balance_tx, token_balance_rx) = mpsc::channel();
    let (order_book_tx, order_book_rv) = mpsc::channel();

    let app_state = web::Data::new(AppState {
        users: Mutex::new(vec![]),
        user_index: Mutex::new(0),
        usd_balance: user_balance_tx,
        token_balance: token_balance_tx,
        order_book: order_book_tx
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

    thread::spawn( move || {
        let symbol = "SOL";
        let bids: BTreeMap<String, VecDeque<OrderBookEntry>> = BTreeMap::new();
        let ask: BTreeMap<String, VecDeque<OrderBookEntry>> = BTreeMap::new();

        while let message = order_book_rv.recv().unwrap() {
            match message {
                AddOrder(user_id, msg_type, qty, price ) => {

                    let fills:Vec<Fills> = vec![];

                    if msg_type == "bid" {
                        let mut unfilled_qty = qty;
                        let (ask_price, ask_price_orders) = ask.first_key_value().unwrap();

                        if ask_price.parse::<i32>().unwrap() <= price {
                            for order in ask_price_orders{
                                let left_qty = order.qty - order.filled_qty;

                                if left_qty >= unfilled_qty {
                                    order.filled_qty += unfilled_qty;

                                    let fill = Fills{
                                        header: FillsTypes::OrderCompleted,
                                        user_id: None,
                                        seller: Some(order.user_id),
                                        buyer: Some(user_id),
                                        price,
                                        qty
                                    };
                                    fills.push(fill);

                                    unfilled_qty = 0;
                                    break;
                                } else {
                                    let fill = Fills {
                                        header: FillsTypes::OrderCompleted,
                                        user_id: None,
                                        seller: Some(order.user_id),
                                        buyer: Some(user_id),
                                        price,
                                        qty: left_qty
                                    };
                                    fills.push(fill);

                                    unfilled_qty - left_qty;
                                    ask_price_orders.pop_front();
                                }
                            }

                            if unfilled_qty == 0 {
                                break;
                            }
                            
                        } else {
                            if unfilled_qty != 0 {
                                let order_book_entry = OrderBookEntry{
                                    user_id,
                                    price,
                                    qty,
                                    filled_qty: qty - unfilled_qty,
                                    order_id: rand::random_range(0..=999)
                                };
                                let price_vec = bids.entry(price.to_string()).or_insert(VecDeque::new());
                                price_vec.push_back(order_book_entry);

                                let fill = Fills {
                                    header: FillsTypes::OrderBookEntry,
                                    user_id: Some(user_id),
                                    seller: None,
                                    buyer: None,
                                    price,
                                    qty: unfilled_qty
                                };
                                fills.push(fill);
                            }
                        }
                            
                        }
                    }
                }
            }
        });

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
