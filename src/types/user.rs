use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct SignupInput {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct SigninInput {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct SignupResponse {
    pub message: String,
    pub token: String
}

#[derive(Deserialize, Serialize)]
pub struct SigninResponse {
    pub  message: String,
    pub token: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: i32,
    pub exp: usize,
}