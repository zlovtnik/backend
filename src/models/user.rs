#![cfg_attr(test, allow(dead_code))]

use diesel::{Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};

use crate::schema::users;

// Include pure functional operations for User
pub mod operations;

#[derive(Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub password: String,
    pub login_session: String,
    pub active: bool,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = users)]
pub struct UserDTO {
    pub username: String,
    pub email: String,
    pub password: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UserUpdateDTO {
    pub username: String,
    pub email: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize)]
pub struct UserResponseDTO {
    pub id: i32,
    pub username: String,
    pub email: String,
    pub active: bool,
}

#[derive(Serialize, Deserialize)]
pub struct SignupDTO {
    pub username: String,
    pub email: String,
    pub password: String,
    pub tenant_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginDTO {
    pub username_or_email: String,
    pub password: String,
    pub tenant_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct LoginInfoDTO {
    pub username: String,
    pub login_session: String,
    pub tenant_id: String,
}
