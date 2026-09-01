use actix_web::{HttpRequest, HttpMessage};
use crate::{middleware::user::UserAuth};

pub fn get_user_id(req: HttpRequest) -> i32 {
    
    let extension = req.extensions();
    let user = extension.get::<UserAuth>().unwrap();
    let user_id = user.id;

    return user_id;
}