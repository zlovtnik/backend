use chrono::{NaiveDateTime, Utc};
use diesel::{prelude::*, Associations, Identifiable, Insertable, Queryable};
use uuid::Uuid;

use crate::{config::db::Connection, models::user::User, schema::refresh_tokens};

#[derive(Debug, Identifiable, Associations, Queryable)]
#[diesel(belongs_to(User))]
#[diesel(table_name = refresh_tokens)]
pub struct RefreshToken {
    pub id: i32,
    pub user_id: i32,
    pub token: String,
    pub expires_at: NaiveDateTime,
    pub created_at: Option<NaiveDateTime>,
    pub revoked: Option<bool>,
}

#[derive(Insertable)]
#[diesel(table_name = refresh_tokens)]
pub struct NewRefreshToken {
    pub user_id: i32,
    pub token: String,
    pub expires_at: NaiveDateTime,
}

impl RefreshToken {
    /// Generates, stores, and returns a new refresh token for the specified user.
    ///
    /// Creates a new UUID-based token, sets its expiry to 30 days from now, inserts a
    /// corresponding refresh token row into the database, and returns the token string.
    ///
    /// # Returns
    ///
    /// `Ok(String)` containing the generated refresh token on success, `Err(diesel::result::Error)` on failure.
    ///
    /// # Examples
    ///
    /// ```
    /// // `conn` should be a valid mutable database connection in real usage.
    /// let mut conn = /* obtain test connection */ unimplemented!();
    /// let token = RefreshToken::create(42, &mut conn).unwrap();
    /// assert!(!token.is_empty());
    /// ```
    pub fn create(
        user_id_val: i32,
        conn: &mut Connection,
    ) -> Result<String, diesel::result::Error> {
        let token_val = Uuid::new_v4().to_string();
        let expires_at_val = (Utc::now() + chrono::Duration::days(30)).naive_utc();

        let new_token = NewRefreshToken {
            user_id: user_id_val,
            token: token_val.clone(),
            expires_at: expires_at_val,
        };

        diesel::insert_into(refresh_tokens::table)
            .values(&new_token)
            .execute(conn)?;

        Ok(token_val)
    }

    pub fn find_by_token(token_val: &str, conn: &mut Connection) -> QueryResult<Self> {
        refresh_tokens::table
            .filter(refresh_tokens::token.eq(token_val))
            .filter(
                refresh_tokens::revoked
                    .is_null()
                    .or(refresh_tokens::revoked.eq(false)),
            )
            .filter(refresh_tokens::expires_at.gt(Utc::now().naive_utc()))
            .get_result(conn)
    }

    pub fn revoke(token_val: &str, conn: &mut Connection) -> QueryResult<usize> {
        diesel::update(refresh_tokens::table.filter(refresh_tokens::token.eq(token_val)))
            .set(refresh_tokens::revoked.eq(true))
            .execute(conn)
    }

    pub fn revoke_all_for_user(user_id_val: i32, conn: &mut Connection) -> QueryResult<usize> {
        diesel::update(refresh_tokens::table.filter(refresh_tokens::user_id.eq(user_id_val)))
            .set(refresh_tokens::revoked.eq(true))
            .execute(conn)
    }
}
