use chrono::{NaiveDateTime, Utc};
use diesel::{prelude::*, Associations, Identifiable, Insertable, Queryable};

use crate::{
    config::db::Connection,
    models::user::{operations as user_ops, User},
    schema::login_history::{self, dsl::*},
};

#[derive(Identifiable, Associations, Queryable)]
#[diesel(belongs_to(User))]
#[diesel(table_name = login_history)]
pub struct LoginHistory {
    pub id: i32,
    pub user_id: i32,
    pub login_timestamp: NaiveDateTime,
}
#[derive(Insertable)]
#[diesel(table_name = login_history)]
pub struct LoginHistoryInsertableDTO {
    pub user_id: i32,
    pub login_timestamp: NaiveDateTime,
}

impl LoginHistory {
    /// Constructs an insertable login-history record for the given username if the user exists.
    ///
    /// Looks up the user by username and, on success, returns a `LoginHistoryInsertableDTO` populated
    /// with the user's `id` and the current UTC timestamp. Returns `None` if no matching user is found.
    ///
    /// # Returns
    ///
    /// `Some(LoginHistoryInsertableDTO)` with `user_id` and `login_timestamp` when the user exists, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// // assume `conn` is a valid &mut Connection and a user "alice" exists
    /// let dto = create("alice", &mut conn);
    /// assert!(dto.is_some());
    /// if let Some(record) = dto {
    ///     assert!(record.user_id > 0);
    /// }
    /// ```
    pub fn create(un: &str, conn: &mut Connection) -> Option<LoginHistoryInsertableDTO> {
        user_ops::find_user_by_username(un, conn).ok().map(|user| {
            let now = Utc::now();
            LoginHistoryInsertableDTO {
                user_id: user.id,
                login_timestamp: now.naive_utc(),
            }
        })
    }

    /// Inserts a login history record into the database and returns how many rows were affected.
    ///
    /// Returns the number of rows inserted on success, or a Diesel error on failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::models::login_history::{LoginHistoryInsertableDTO, save_login_history};
    /// use crate::config::db::Connection;
    ///
    /// let mut conn: Connection = /* obtain connection */;
    /// let record = LoginHistoryInsertableDTO {
    ///     user_id: 1,
    ///     login_timestamp: chrono::Utc::now().naive_utc(),
    /// };
    ///
    /// let affected = save_login_history(record, &mut conn).unwrap();
    /// assert_eq!(affected, 1);
    /// ```
    pub fn save_login_history(
        insert_record: LoginHistoryInsertableDTO,
        conn: &mut Connection,
    ) -> QueryResult<usize> {
        diesel::insert_into(login_history)
            .values(&insert_record)
            .execute(conn)
    }
}
