use actix_web::{
    Error, HttpMessage, HttpResponse,
    body::{EitherBody, MessageBody, BoxBody},
    dev::{ServiceRequest, ServiceResponse},
    middleware::Next,
};

use crate::helper::token_fn::{verify_token};

pub struct UserAuth {
    pub id: i32,
}

pub async fn user_auth(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<EitherBody<impl MessageBody, BoxBody>>, Error> {
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|x| x.to_str().ok());

    match auth_header {
        None => Ok(
            req.into_response(
                HttpResponse::Unauthorized()
                .body("User Unauthorized!"))
                .map_into_right_body()
            ),

        Some(auth_token) => {
            let token = auth_token.replace("Bearer ", "");
            let data = verify_token(token.clone()).await;

            if data.valid {
                req.extensions_mut().insert(UserAuth { id: data.user_id });

                let response = next.call(req).await?;
                Ok(response.map_into_left_body())
            } else {
                Ok(
                    req.into_response(
                        HttpResponse::Unauthorized()
                        .body("Invalid token!")
                        .map_into_right_body()
                    ))
            }
        }
    }

}
