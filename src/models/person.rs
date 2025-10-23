use diesel::{prelude::*, AsChangeset, BoxableExpression, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::{
    config::db::Connection, constants::MESSAGE_OK, error::ServiceError,
    models::pagination::SortingAndPaging, schema::people,
};

use super::{
    filters::PersonFilter, functional_utils, pagination::HasId, response::Page, Custom, Email,
    Length, Phone, Range,
};

use crate::functional::{validation_engine::ValidationOutcome, validation_rules::ValidationRule};

// Re-export functional utilities for person operations

#[derive(Clone, Queryable, Serialize, Deserialize)]
pub struct Person {
    pub id: i32,
    pub name: String,
    pub gender: bool,
    pub age: i32,
    pub address: String,
    pub phone: String,
    pub email: String,
}

#[derive(Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = people)]
pub struct PersonDTO {
    pub name: String,
    pub gender: bool,
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
    fn is_not_blank(value: &String) -> bool {
        !value.trim().is_empty()
    }

    /// Validate the DTO's fields and collect any validation error messages.
    ///
    /// Performs the following checks:
    /// - name: required (non-blank) and maximum length 100.
    /// - email: required, valid email format, and maximum length 255.
    /// - phone: required, length between 10 and 20, and valid phone format.
    /// - address: required and maximum length 500.
    /// - age: must be between 0 and 150.
    ///
    /// # Returns
    ///
    /// `Ok(())` if all validations pass, `Err(Vec<String>)` containing one or more human-readable error messages otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let dto = PersonDTO {
    ///     name: "".into(),
    ///     gender: true,
    ///     age: 25,
    ///     address: "123 Main".into(),
    ///     phone: "1234567890".into(),
    ///     email: "test@example.com".into(),
    /// };
    ///
    /// let res = dto.validate();
    /// assert!(res.is_err());
    /// let errors = res.unwrap_err();
    /// assert!(errors.iter().any(|e| e.contains("name")));
    /// ```
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let string_engine = functional_utils::validation_engine::<String>();
        let range_engine = functional_utils::validation_engine::<i32>();

        let string_validations = [
            string_engine.validate_field(
                &self.name,
                "name",
                vec![Custom::new(
                    Self::is_not_blank,
                    "REQUIRED",
                    "{} is required",
                )],
            ),
            string_engine.validate_field(
                &self.name,
                "name",
                vec![Length {
                    min: None,
                    max: Some(100),
                }],
            ),
            string_engine.validate_field(
                &self.email,
                "email",
                vec![Custom::new(
                    Self::is_not_blank,
                    "REQUIRED",
                    "{} is required",
                )],
            ),
            string_engine.validate_field(&self.email, "email", vec![Email]),
            string_engine.validate_field(
                &self.email,
                "email",
                vec![Length {
                    min: None,
                    max: Some(255),
                }],
            ),
            string_engine.validate_field(
                &self.phone,
                "phone",
                vec![Custom::new(
                    Self::is_not_blank,
                    "REQUIRED",
                    "{} is required",
                )],
            ),
            string_engine.validate_field(
                &self.phone,
                "phone",
                vec![Length {
                    min: Some(10),
                    max: Some(20),
                }],
            ),
            string_engine.validate_field(&self.phone, "phone", vec![Phone]),
            {
                let rules: Vec<Box<dyn ValidationRule<String>>> = vec![
                    Box::new(Custom::new(
                        Self::is_not_blank,
                        "REQUIRED",
                        "{} is required",
                    )),
                    Box::new(Length {
                        min: None,
                        max: Some(500),
                    }),
                ];
                let mut errors = Vec::new();
                for rule in &rules {
                    if let Err(e) = rule.validate(&self.address, "address") {
                        errors.push(e);
                    }
                }
                if errors.is_empty() {
                    ValidationOutcome::success(&self.address)
                } else {
                    ValidationOutcome::failure(errors)
                }
            },
        ];

        let age_validations = [range_engine.validate_field(
            &self.age,
            "age",
            vec![Range {
                min: Some(0),
                max: Some(150),
            }],
        )];

        let mut errors: Vec<String> = string_validations
            .into_iter()
            .flat_map(|outcome| functional_utils::to_error_messages(outcome.errors))
            .collect();

        errors.extend(
            age_validations
                .into_iter()
                .flat_map(|outcome| functional_utils::to_error_messages(outcome.errors)),
        );

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
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
    /// - `gender`: accepts `"male"` or `"female"` (case-insensitive) and maps to the stored boolean.
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
    pub fn filter(filter: PersonFilter, conn: &mut Connection) -> QueryResult<Page<Person>> {
        // Use functional query building with iterator-based predicate composition
        let mut query = people::table.into_boxed();

        // Build query using functional composition with fold
        let predicates: Vec<
            Box<
                dyn BoxableExpression<
                    people::table,
                    diesel::pg::Pg,
                    SqlType = diesel::sql_types::Bool,
                >,
            >,
        > = vec![
            filter.age.map(|age| people::age.eq(age)).map(|expr| {
                Box::new(expr)
                    as Box<
                        dyn BoxableExpression<
                            people::table,
                            diesel::pg::Pg,
                            SqlType = diesel::sql_types::Bool,
                        >,
                    >
            }),
            filter
                .email
                .as_ref()
                .map(|email| people::email.like(format!("%{}%", email)))
                .map(|expr| {
                    Box::new(expr)
                        as Box<
                            dyn BoxableExpression<
                                people::table,
                                diesel::pg::Pg,
                                SqlType = diesel::sql_types::Bool,
                            >,
                        >
                }),
            filter
                .name
                .as_ref()
                .map(|name| people::name.like(format!("%{}%", name)))
                .map(|expr| {
                    Box::new(expr)
                        as Box<
                            dyn BoxableExpression<
                                people::table,
                                diesel::pg::Pg,
                                SqlType = diesel::sql_types::Bool,
                            >,
                        >
                }),
            filter
                .phone
                .as_ref()
                .map(|phone| people::phone.like(format!("%{}%", phone)))
                .map(|expr| {
                    Box::new(expr)
                        as Box<
                            dyn BoxableExpression<
                                people::table,
                                diesel::pg::Pg,
                                SqlType = diesel::sql_types::Bool,
                            >,
                        >
                }),
            filter
                .gender
                .as_ref()
                .and_then(|gender| match gender.to_lowercase().as_str() {
                    "male" => Some(people::gender.eq(true)),
                    "female" => Some(people::gender.eq(false)),
                    _ => None,
                })
                .map(|expr| {
                    Box::new(expr)
                        as Box<
                            dyn BoxableExpression<
                                people::table,
                                diesel::pg::Pg,
                                SqlType = diesel::sql_types::Bool,
                            >,
                        >
                }),
        ]
        .into_iter()
        .flatten()
        .collect();

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
            .load_items::<Person>(conn)?;
        Ok(Page::new(
            MESSAGE_OK,
            records.data,
            cursor,
            page_size,
            records.total_elements,
            records.next_cursor,
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
    ///     gender: true,
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
        new_person
            .validate()
            .map_err(|errors| ServiceError::bad_request(errors.join("; ")))?;

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
    ///     gender: true,
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
