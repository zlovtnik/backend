use diesel::{prelude::*, AsChangeset, BoxableExpression, Insertable, Queryable};
use diesel_derive_enum::DbEnum;
use serde::{Deserialize, Serialize};

use crate::{
    config::db::Connection, constants::MESSAGE_OK, error::ServiceError,
    models::pagination::SortingAndPaging, schema::people,
};

use super::{filters::PersonFilter, pagination::HasId, response::Page};

pub mod validators;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, DbEnum)]
#[DbValueStyle = "snake_case"]
#[serde(rename_all = "snake_case")]
#[DieselType = "Gender"]
pub enum PersonGender {
    Male,
    Female,
    NonBinary,
    PreferNotToSay,
}

#[derive(Clone, Queryable, Serialize, Deserialize)]
pub struct Person {
    pub id: i32,
    pub name: String,
    pub gender: Option<PersonGender>,
    pub age: i32,
    pub address: String,
    pub phone: String,
    pub email: String,
}

#[derive(Insertable, AsChangeset, Serialize, Deserialize, Clone)]
#[diesel(table_name = people)]
pub struct PersonDTO {
    pub name: String,
    /// Gender field represented as an enum. Missing/null becomes None.
    #[serde(default)]
    pub gender: Option<PersonGender>,
    pub age: i32,
    pub address: String,
    pub phone: String,
    pub email: String,
}

impl PersonDTO {
    /// Check whether a string contains any non-whitespace characters.
    ///
    /// Returns `true` if the string contains any non-whitespace characters, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// assert!(crate::models::person::PersonDTO::is_not_blank(&"hello".to_string()));
    /// assert!(crate::models::person::PersonDTO::is_not_blank(&"  a  ".to_string()));
    /// assert!(!crate::models::person::PersonDTO::is_not_blank(&"   ".to_string()));
    /// assert!(!crate::models::person::PersonDTO::is_not_blank(&"".to_string()));
    /// ```
    #[allow(dead_code)]
    fn is_not_blank(value: &String) -> bool {
        !value.trim().is_empty()
    }

    /// Validate the DTO using the functional validator combinators.
    ///
    /// Returns `Ok(())` when all validation rules pass or `Err(ServiceError)`
    /// containing the first failing rule.
    pub fn validate(&self) -> Result<(), ServiceError> {
        validators::validate_person(self)
    }
}

impl HasId for Person {
    fn id(&self) -> i32 {
        self.id
    }
}

impl Person {
    pub fn find_all(conn: &mut Connection) -> QueryResult<Vec<Person>> {
        people::table.order(people::id.asc()).load::<Person>(conn)
    }

    pub fn find_by_id(i: i32, conn: &mut Connection) -> QueryResult<Person> {
        people::table.find(i).get_result::<Person>(conn)
    }

    /// Get a paginated Page of people matching the provided filter criteria.
    ///
    /// Applies the following optional filters from `PersonFilter`:
    /// - `age`: exact match.
    /// - `email`, `name`, `phone`: partial match using SQL `LIKE` with surrounding `%` wildcards (case-sensitive).
    /// - `gender`: accepts `"male"`, `"female"`, `"non_binary"`, `"prefer_not_to_say"` (case-insensitive).
    ///
    /// Pagination uses `filter.cursor` as the page cursor (defaults to `0`) and `filter.page_size` as items per page (defaults to `crate::constants::DEFAULT_PER_PAGE`).
    ///
    /// # Examples
    ///
    /// ```
    /// // Construct a filter to find people with "example" in their email and use a DB connection.
    /// let filter = PersonFilter {
    ///     email: Some("example".into()),
    ///     age: None,
    ///     gender: None,
    ///     name: None,
    ///     phone: None,
    ///     cursor: None,
    ///     page_size: None,
    /// };
    ///
    /// let mut conn: Connection = /* obtain connection */;
    ///
    /// let page = Person::filter(filter, &mut conn).expect("query failed");
    /// assert!(page.items.len() <= crate::constants::DEFAULT_PER_PAGE);
    /// ```
    pub fn filter(
        filter: PersonFilter,
        conn: &mut Connection,
    ) -> Result<Page<Person>, ServiceError> {
        // Use functional query building with iterator-based predicate composition
        let mut query = people::table.into_boxed();

        // Build query using functional composition with fold
        let mut predicate_results: Vec<
            Result<
                Box<
                    dyn BoxableExpression<
                        people::table,
                        diesel::pg::Pg,
                        SqlType = diesel::sql_types::Nullable<diesel::sql_types::Bool>,
                    >,
                >,
                ServiceError,
            >,
        > = Vec::new();

        if let Some(age) = filter.age {
            predicate_results.push(Ok(Box::new(people::age.eq(age).nullable())
                as Box<
                    dyn BoxableExpression<
                        people::table,
                        diesel::pg::Pg,
                        SqlType = diesel::sql_types::Nullable<diesel::sql_types::Bool>,
                    >,
                >));
        }

        if let Some(email) = filter.email.as_ref() {
            let escaped_email = email
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            predicate_results.push(Ok(Box::new(
                people::email.like(format!("%{}%", escaped_email)).nullable(),
            )
                as Box<
                    dyn BoxableExpression<
                        people::table,
                        diesel::pg::Pg,
                        SqlType = diesel::sql_types::Nullable<diesel::sql_types::Bool>,
                    >,
                >));
        }

        if let Some(name) = filter.name.as_ref() {
            predicate_results.push(Ok(
                Box::new(people::name.like(format!("%{}%", name)).nullable())
                    as Box<
                        dyn BoxableExpression<
                            people::table,
                            diesel::pg::Pg,
                            SqlType = diesel::sql_types::Nullable<diesel::sql_types::Bool>,
                        >,
                    >,
            ));
        }

        if let Some(phone) = filter.phone.as_ref() {
            predicate_results.push(Ok(Box::new(
                people::phone.like(format!("%{}%", phone)).nullable(),
            )
                as Box<
                    dyn BoxableExpression<
                        people::table,
                        diesel::pg::Pg,
                        SqlType = diesel::sql_types::Nullable<diesel::sql_types::Bool>,
                    >,
                >));
        }

        if let Some(gender) = filter.gender.as_ref() {
            let normalized_gender = gender.trim().to_lowercase().replace('-', "_");
            let expr = match normalized_gender.as_str() {
                "male" => Some(people::gender.eq(Some(PersonGender::Male))),
                "female" => Some(people::gender.eq(Some(PersonGender::Female))),
                "non_binary" | "nonbinary" => {
                    Some(people::gender.eq(Some(PersonGender::NonBinary)))
                }
                "prefer_not_to_say" | "prefernottosay" => {
                    Some(people::gender.eq(Some(PersonGender::PreferNotToSay)))
                }
                _ => {
                    return Err(ServiceError::bad_request(format!(
                        "Invalid gender filter value: {gender}. Supported values: male, female, non_binary, prefer_not_to_say"
                    ))
                    .with_tag("invalid_gender"))
                }
            };

            if let Some(expr) = expr {
                predicate_results.push(Ok(Box::new(expr)
                    as Box<
                        dyn BoxableExpression<
                            people::table,
                            diesel::pg::Pg,
                            SqlType = diesel::sql_types::Nullable<diesel::sql_types::Bool>,
                        >,
                    >));
            }
        }

        let predicates: Vec<
            Box<
                dyn BoxableExpression<
                    people::table,
                    diesel::pg::Pg,
                    SqlType = diesel::sql_types::Nullable<diesel::sql_types::Bool>,
                >,
            >,
        > = predicate_results
            .into_iter()
            .collect::<Result<_, ServiceError>>()?;

        query = predicates
            .into_iter()
            .fold(query, |q, predicate| q.filter(predicate));

        let cursor = filter.cursor.unwrap_or(0);
        let page_size = filter
            .page_size
            .unwrap_or(crate::constants::DEFAULT_PER_PAGE);

        // Handle sorting through pagination - don't add ORDER BY to the base query
        // The pagination system will handle ordering by the cursor column
        let records = query
            .paginate(cursor)
            .per_page(page_size)
            .load_items::<Person>(conn)
            .map_err(|e| {
                ServiceError::internal_server_error(format!("Failed to list people: {e}"))
            })?;
        Ok(Page::new(
            MESSAGE_OK,
            records.data,
            cursor,
            page_size,
            records.total_elements,
            records.next_cursor,
            records.previous_cursor,
        ))
    }

    /// Insert a new person record into the `people` table.
    ///
    /// Inserts the provided `PersonDTO` and returns the number of rows inserted.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::models::{PersonDTO, insert};
    /// // `conn` is a mutable database connection available in your test/setup.
    /// let new_person = PersonDTO {
    ///     name: "Alice".into(),
    ///     gender: Some(Gender::Female),
    ///     age: 30,
    ///     address: "123 Main St".into(),
    ///     phone: "555-1234".into(),
    ///     email: "alice@example.com".into(),
    /// };
    /// let rows_inserted = insert(new_person, &mut conn).unwrap();
    /// assert_eq!(rows_inserted, 1);
    /// ```
    pub fn insert(new_person: PersonDTO, conn: &mut Connection) -> Result<usize, ServiceError> {
        // Validate using functional validation patterns
        new_person.validate()?;

        // Insert using functional composition
        diesel::insert_into(people::table)
            .values(&new_person)
            .execute(conn)
            .map_err(|e| {
                ServiceError::internal_server_error(format!("Failed to insert person: {}", e))
            })
    }

    /// Updates the person record with the specified id using values from `updated_person`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::{PersonDTO, update, Connection};
    /// let mut conn: Connection = /* obtain connection */;
    /// let dto = PersonDTO {
    ///     name: "Alice".into(),
    ///     gender: Some(Gender::Female),
    ///     age: 30,
    ///     address: "123 Main St".into(),
    ///     phone: "555-0100".into(),
    ///     email: "alice@example.com".into(),
    /// };
    /// let rows = update(1, dto, &mut conn).expect("update failed");
    /// assert_eq!(rows, 1);
    /// ```
    ///
    /// # Returns
    ///
    /// Number of rows updated on success.
    pub fn update(i: i32, updated_person: PersonDTO, conn: &mut Connection) -> QueryResult<usize> {
        diesel::update(people::table.find(i))
            .set(&updated_person)
            .execute(conn)
    }

    /// Deletes the person with the given id from the people table.
    ///
    /// # Returns
    ///
    /// `usize` number of rows deleted.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::config::db::Connection;
    /// use crate::models::person::Person;
    ///
    /// // `conn` must be a mutable database connection.
    /// let mut conn: Connection = /* obtain connection */;
    /// let deleted = Person::delete(1, &mut conn).unwrap();
    /// assert_eq!(deleted, 1);
    /// ```
    pub fn delete(i: i32, conn: &mut Connection) -> QueryResult<usize> {
        diesel::delete(people::table.find(i)).execute(conn)
    }
}
