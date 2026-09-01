use std::collections::HashMap;

use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, post, web::{self, Json}};

use crate::{ AppState, helper::token_fn::create_token, middleware::user::UserAuth, types::user::{ 
    GetUserBalanceResponse, SigninInput, SigninResponse, SignupInput, SignupResponse, User }
};

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

        let token = match create_token(user_index.clone()).await {
            Ok(t) => t,
            Err(_) => return HttpResponse::InternalServerError().json(SignupResponse {
                message: String::from("Something went wrong unable to create token!"),
                token: String::from("")
            }),
        };

        drop(users);
        drop(user_index);
        drop(usd_balance);
        drop(token_balance);
    
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

    let extension = req.extensions();
    let user = extension.get::<UserAuth>().unwrap();
    let user_id = user.id;
    let usd_balance_data = app_state.usd_balance.lock().unwrap();
    let token_balance_data = app_state.token_balance.lock().unwrap();

    let user_usd_balance = usd_balance_data.get(&user_id).unwrap_or(&0).clone();
    let user_asset_balance = token_balance_data.get(&user_id).unwrap_or(&HashMap::new()).clone();

    drop(usd_balance_data);
    drop(token_balance_data);

    HttpResponse::Ok().json(GetUserBalanceResponse{
        usd_balance: user_usd_balance,
        token_balance: user_asset_balance
    })
}