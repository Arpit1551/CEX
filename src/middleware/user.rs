use actix_web::{
    Error,
    body::MessageBody,
    dev::{ServiceRequest, ServiceResponse},
    middleware:: Next,
    HttpResponse
};

use crate::{helper::token_fn::{verify_token, VerifyTokenResposne}};


pub struct UserAuth {
    pub id: i32,
}

pub async fn user_auth(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|x| x.to_str().ok());

    match auth_header {
        Some(auth_token) => {

            let token = auth_token.replace("Bearer ", "");
            let data = verify_token(token.clone()).await;


            }
        },
        None => {
             Ok(req.into_response(HttpResponse::Unauthorized().body("User Unauthorized!")))
        }
    }

}
