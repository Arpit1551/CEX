use crate::{types::user::Claims};
use actix_web::Error;
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, DecodingKey, Validation, decode, encode};

pub struct VerifyTokenResposne {
    pub valid: bool,
    pub user_id: i32
}

pub async fn create_token(user_id: i32) -> Result<String, Error> {
    let exp = Utc::now()
        .checked_add_signed(Duration::hours(24))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.clone(),
        exp,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret("secret".as_ref()),
    )
    .unwrap();

    return Ok(token);
}

pub async fn verify_token(token: String) -> VerifyTokenResposne {
        let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret("secret".as_bytes()),
        &Validation::default(),
    );

    match token_data {
        Ok(data) => VerifyTokenResposne {
            valid: true,
            user_id: data.claims.sub,
        },
        Err(_) => VerifyTokenResposne {
            valid: false,
            user_id: 0,
        },
    }
}
