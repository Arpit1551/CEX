use actix_web::{ HttpRequest, HttpResponse, Responder, body, post, web::{self, Json}};
use futures::channel::oneshot;
use std::sync::mpsc;

use crate::{ 
    AppState, OrderBook::AddOrder, TokenTransactions::{self, DepositToken, GetAllTokens, GetTokenBalance}, UserBalanceTx::{self, GetBalance, Onramp}, helper::{token_fn::create_token, user_fn::get_user_id }, middleware::user, types::user::{ 
    DepositRequest, DepositResponse, GetUserBalanceResponse, OnRampRequest, OrderRequest, OrderResponse, SigninInput, SigninResponse, SignupInput, SignupResponse, User }
};

#[post("/signup")]
async fn signup(app_state: web::Data<AppState>, user_info: Json<SignupInput>) -> impl Responder {
    let mut users = app_state.users.lock().unwrap();
    let mut user_index = app_state.user_index.lock().unwrap();

    let user_found = users.iter().find(|u| u.username == user_info.username);

    if user_found.is_none() {
        
        app_state.usd_balance.send(Onramp(user_index.clone(), 0));
        *user_index = *user_index + 1;

        let new_user = User {
            id: user_index.clone(),
            username: user_info.username.clone(),
            password: user_info.password.clone(),
        };

        users.push(new_user);

        let token = match create_token(user_index.clone()).await {
            Ok(t) => t,
            Err(_) => return HttpResponse::InternalServerError().json(SignupResponse {
                message: String::from("Something went wrong unable to create token!"),
                token: String::from("")
            }),
        };

        drop(users);
        drop(user_index);

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

    let user_index = app_state.user_index.lock().unwrap();
    
    let token = match create_token(user_index.clone()).await {
        Ok(t) => t,
        Err(_) => return HttpResponse::Unauthorized().json(SigninResponse{
            message: String::from("Something went wrong, Unable to create token!"),
            token: String::from("")
        })
    };

    HttpResponse::Ok().json(SigninResponse { 
        message: String::from("Login successfull!"),
        token: token 
    })
}

#[post("/get_balance")]
async fn balance(app_state: web::Data<AppState>, req: HttpRequest) -> impl Responder {

    let user_id = get_user_id(req);
    let (user_balance_tx, user_balance_rx) = oneshot::channel();
    let (token_balance_tx, token_balance_rx) = oneshot::channel();

    app_state.usd_balance.send(GetBalance(user_id, user_balance_tx));
    app_state.token_balance.send(GetAllTokens(user_id, token_balance_tx));

    let user_usd_balance = user_balance_rx.await.unwrap();
    let user_asset_balance = token_balance_rx.await.unwrap();

    HttpResponse::Ok().json(GetUserBalanceResponse{
        usd_balance: user_usd_balance,
        token_balance: user_asset_balance
    })
}

#[post("/onramp")]
async fn onramp(app_state: web::Data<AppState>, req: HttpRequest, body: Json<OnRampRequest>)-> impl Responder {

    let user_id = get_user_id(req);
    app_state.usd_balance.send(UserBalanceTx::Onramp(user_id, body.qty));

    HttpResponse::Ok().body("Balance updated!")
}

#[post("/deposit/{asset_symbol}")]
async fn deposit(app_state: web::Data<AppState>, req: HttpRequest, symbol: web::Path<String>, body: Json<DepositRequest>) -> impl Responder {
    
    let user_id = get_user_id(req);
    let symbol = symbol.into_inner();
    app_state.token_balance.send(TokenTransactions::DepositToken(user_id, symbol, body.qty));

    HttpResponse::Ok().json(DepositResponse {
        msg: String::from("Deposit successfull!")
    })
}

#[post("/orders")]
async fn orders(app_state: web::Data<AppState>, body: web::Json<OrderRequest>, req: HttpRequest) -> impl Responder {

    let user_id = get_user_id(req);
    let (check_balance_tx, check_balance_rx) = oneshot::channel();
    let (order_book_response_tx, order_book_response_rx) = mpsc::channel();

    if body.header == "bid" {
        app_state.usd_balance.send(GetBalance(user_id, check_balance_tx));

        if check_balance_rx.await.unwrap() < body.price * body.qty {
            return HttpResponse::BadRequest().json(OrderResponse {
                msg: String::from("Insufficient fund!")
            })};

            app_state.order_book.send(
                crate::AddOrder(
                    user_id,
                    body.header, 
                    body.qty,
                    body.price, 
                    order_book_response_tx
                ));

            println!("{:?}", order_book_response_rx.recv().unwrap());
    }

    HttpResponse::Ok()
}

#[post("/cancle")]
async fn cancle(_app_state: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok()
}