use chrono::NaiveDateTime;
use diesel::{prelude::*, result, AsChangeset, Identifiable, Insertable, Queryable};
use log;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    constants::{self, MESSAGE_OK},
    models::{filters::TenantFilter, response::Page},
    pagination::{PaginatedPage, Pagination as IteratorPagination},
    schema::tenants::{self, dsl::*},
};

use super::{functional_utils, Custom};

use crate::models::ValidationError;

// Re-export functional utilities for tenant operations

const MAX_PAGE_SIZE: i64 = 10_000;

#[derive(Clone, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = tenants)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub db_url: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

#[derive(Insertable, Serialize, Deserialize)]
#[diesel(table_name = tenants)]
pub struct TenantDTO {
    pub id: String,
    pub name: String,
    pub db_url: String,
}

#[derive(AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = tenants)]
pub struct UpdateTenant {
    pub name: Option<String>,
    pub db_url: Option<String>,
}

impl Tenant {
    /// Checks whether a string contains any non-whitespace characters.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::models::tenant::Tenant;
    ///
    /// let s = "  a  ".to_string();
    /// assert!(Tenant::is_not_blank(&s));
    ///
    /// let empty = "   ".to_string();
    /// assert!(!Tenant::is_not_blank(&empty));
    /// ```
    ///
    /// # Returns
    ///
    /// `true` if the string contains at least one non-whitespace character, `false` otherwise.
    fn is_not_blank(value: &String) -> bool {
        !value.trim().is_empty()
    }

    /// Checks whether every character in `value` is an alphanumeric character or `-` or `_`.
    ///
    /// Returns `true` if all characters satisfy `is_alphanumeric()` or are `'-'` or `'_'`, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::models::tenant::Tenant;
    ///
    /// assert!(Tenant::is_valid_identifier(&"tenant-123_α".to_string()));
    /// assert!(!Tenant::is_valid_identifier(&"bad id!".to_string()));
    /// ```
    fn is_valid_identifier(value: &String) -> bool {
        !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    }

    /// Parses a UTC timestamp string in the expected ISO-like format into a `NaiveDateTime`.
    ///
    /// Accepts timestamps formatted as `YYYY-MM-DDTHH:MM:SS` with an optional fractional seconds component and a trailing `Z` (example: `"2023-05-01T12:34:56.789Z"`).
    ///
    /// # Returns
    ///
    /// `Some(NaiveDateTime)` if the input matches the expected format, `None` otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::models::tenant::Tenant;
    ///
    /// let dt = Tenant::parse_timestamp("2023-05-01T12:34:56.789Z").unwrap();
    /// assert_eq!(dt.format("%Y-%m-%dT%H:%M:%S").to_string(), "2023-05-01T12:34:56");
    /// ```
    fn parse_timestamp(value: &str) -> Option<NaiveDateTime> {
        NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.fZ").ok()
    }

    /// Validates that the provided string is a well-formed URL suitable for a database connection.
    ///
    /// # Examples
    ///
    /// ```
    /// // Check a valid URL
    /// assert!(Tenant::validate_db_url("postgres://user:pass@localhost/db").is_ok());
    ///
    /// // Check an invalid URL
    /// assert!(Tenant::validate_db_url("not-a-valid-url").is_err());
    /// ```
    pub fn validate_db_url(url: &str) -> QueryResult<()> {
        // Validate URL format
        Url::parse(url).map_err(|_| {
            result::Error::DatabaseError(
                result::DatabaseErrorKind::Unknown,
                Box::new("Invalid database URL format".to_string()),
            )
        })?;

        Ok(())
    }

    /// Inserts a new tenant record after validating the DTO and its database URL.
    ///
    /// Returns the inserted `Tenant`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::models::{Tenant, TenantDTO};
    ///
    /// let dto = TenantDTO {
    ///     id: "tenant_1".into(),
    ///     name: "Tenant One".into(),
    ///     db_url: "postgres://user:pass@localhost/tenant_db".into(),
    /// };
    ///
    /// let mut conn = crate::config::db::establish_connection();
    /// let tenant = Tenant::create(dto, &mut conn).unwrap();
    /// assert_eq!(tenant.id, "tenant_1");
    /// ```
    pub fn create(dto: TenantDTO, conn: &mut crate::config::db::Connection) -> QueryResult<Tenant> {
        vec![
            Self::validate_tenant_dto(&dto),
            Self::validate_db_url(&dto.db_url),
        ]
        .into_iter()
        .collect::<QueryResult<Vec<_>>>()?;
        diesel::insert_into(tenants).values(&dto).get_result(conn)
    }

    /// Updates the tenant identified by `id_` with the provided fields.
    ///
    /// If `dto.db_url` is `Some`, the URL is validated before applying the update.
    /// `dto`'s `None` fields are left unchanged.
    ///
    /// # Examples
    ///
    /// ```
    /// let dto = UpdateTenant { name: Some("New Name".into()), db_url: None };
    /// let updated = Tenant::update("tenant-123", dto, &mut conn).unwrap();
    /// assert_eq!(updated.id, "tenant-123");
    /// ```
    ///
    /// Returns the updated `Tenant` on success.
    pub fn update(
        id_: &str,
        dto: UpdateTenant,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<Tenant> {
        if let Some(ref url) = dto.db_url {
            Self::validate_db_url(url)?;
        }
        diesel::update(tenants.find(id_)).set(&dto).get_result(conn)
    }

    pub fn delete(id_: &str, conn: &mut crate::config::db::Connection) -> QueryResult<usize> {
        diesel::delete(tenants.find(id_)).execute(conn)
    }

    pub fn find_by_id(t_id: &str, conn: &mut crate::config::db::Connection) -> QueryResult<Tenant> {
        tenants.filter(id.eq(t_id)).get_result::<Tenant>(conn)
    }

    /// Returns the total count of all tenants in the database.
    ///
    /// This method efficiently counts tenants without loading the actual records into memory.
    ///
    /// # Examples
    ///
    /// ```
    /// let total = Tenant::count_all(&mut conn).unwrap();
    /// assert!(total >= 0);
    /// ```
    pub fn count_all(conn: &mut crate::config::db::Connection) -> QueryResult<i64> {
        tenants.count().get_result::<i64>(conn)
    }

    /// Loads tenants up to an enforced maximum; errors if the total tenant count exceeds MAX_PAGE_SIZE.
    ///
    /// This function first counts tenants and returns an error if the total exceeds the hard limit
    /// of MAX_PAGE_SIZE. If the total is within the limit, it loads and returns up to MAX_PAGE_SIZE tenant records.
    ///
    /// # Errors
    ///
    /// Returns a `DatabaseError` when the total tenant count is greater than MAX_PAGE_SIZE and suggests
    /// using paginated methods instead.
    ///
    /// # Examples
    ///
    /// ```
    /// let all = Tenant::list_all(&mut conn).unwrap();
    /// assert!(all.len() <= 10_000);
    /// ```
    pub fn list_all(conn: &mut crate::config::db::Connection) -> QueryResult<Vec<Tenant>> {
        // Check total count first
        let total_count: i64 = tenants.count().get_result(conn)?;

        if total_count > MAX_PAGE_SIZE {
            return Err(diesel::result::Error::DatabaseError(
                diesel::result::DatabaseErrorKind::Unknown,
                Box::new(format!(
                    "Tenant count ({}) exceeds maximum limit ({}). Use paginated methods instead.",
                    total_count, MAX_PAGE_SIZE
                )),
            ));
        }

        tenants.limit(MAX_PAGE_SIZE).load::<Tenant>(conn)
    }

    /// Loads tenant records with an optional limit; defaults to 1,000 and is capped at MAX_PAGE_SIZE.
    ///
    /// The `limit` value, if provided, will be clamped to a maximum of MAX_PAGE_SIZE. If `None` is
    /// supplied, a default limit of 1,000 is used.
    ///
    /// # Examples
    ///
    /// ```
    /// // `conn` should be a mutable database connection from your application's DB layer.
    /// let tenants = crate::models::tenant::Tenant::list_all_with_limit(Some(50), &mut conn)?;
    /// assert!(tenants.len() <= 50);
    /// ```
    pub fn list_all_with_limit(
        limit: Option<i64>,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<Vec<Tenant>> {
        let limit = limit.unwrap_or(1000).max(0).min(MAX_PAGE_SIZE);
        tenants.limit(limit).load::<Tenant>(conn)
    }

    /// Fetches a page of tenants and the total tenant count.
    ///
    /// The `offset` and `limit` parameters control the page window applied at the database level:
    /// `offset` is the number of records to skip and `limit` is the maximum number of records to return.
    ///
    /// # Returns
    ///
    /// A tuple where the first element is a `Vec<Tenant>` for the requested page and the second element is the total count of tenants.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // assume `conn` is a mutable database connection: &mut crate::config::db::Connection
    /// let (page, total) = Tenant::list_paginated(0, 10, &mut conn).expect("query failed");
    /// assert!(total >= page.len() as i64);
    /// ```
    pub fn list_paginated(
        offset: i64,
        limit: i64,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<(Vec<Tenant>, i64)> {
        let total = tenants.count().get_result::<i64>(conn)?;
        let results = tenants.offset(offset).limit(limit).load::<Tenant>(conn)?;
        Ok((results, total))
    }

    /// Retrieves the tenant that exactly matches the provided name.
    ///
    /// # Returns
    ///
    /// `Tenant` matching the provided name.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut conn = crate::config::db::establish_connection();
    /// let tenant = Tenant::find_by_name("acme", &mut conn).unwrap();
    /// assert_eq!(tenant.name, "acme");
    /// ```
    pub fn find_by_name(
        name_: &str,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<Tenant> {
        tenants.filter(name.eq(name_)).first::<Tenant>(conn)
    }

    /// Validates a TenantDTO's fields against application rules.
    ///
    /// - `id` must not be empty and must contain only alphanumeric characters, dashes, or underscores.
    /// - `name` must not be empty.
    ///
    /// # Returns
    ///
    /// `Ok(())` if validation passes, `Err(diesel::result::Error)` with a joined error message otherwise.
    ///
    /// # Examples
    ///
    /// ```
    /// let dto = TenantDTO {
    ///     id: "tenant_1".into(),
    ///     name: "Tenant One".into(),
    ///     db_url: "postgres://user:pass@localhost/db".into(),
    /// };
    /// assert!(Tenant::validate_tenant_dto(&dto).is_ok());
    /// ```
    pub fn validate_tenant_dto(dto: &TenantDTO) -> QueryResult<()> {
        let string_engine = functional_utils::validation_engine::<String>();

        let validations = [
            string_engine.validate_field(
                &dto.id,
                "id",
                vec![
                    Custom::new(
                        Self::is_not_blank as fn(&String) -> bool,
                        "REQUIRED",
                        "{} cannot be empty",
                    ),
                    Custom::new(
                        Self::is_valid_identifier as fn(&String) -> bool,
                        "INVALID_ID",
                        "{} must contain only alphanumeric characters, dashes, and underscores",
                    ),
                ],
            ),
            string_engine.validate_field(
                &dto.name,
                "name",
                vec![Custom::new(
                    Self::is_not_blank as fn(&String) -> bool,
                    "REQUIRED",
                    "{} cannot be empty",
                )],
            ),
        ];

        let errors: Vec<ValidationError> = validations
            .into_iter()
            .flat_map(|outcome| outcome.errors)
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            Err(result::Error::DatabaseError(
                result::DatabaseErrorKind::Unknown,
                Box::new(
                    errors
                        .into_iter()
                        .map(|e| e.message)
                        .collect::<Vec<_>>()
                        .join("; "),
                ),
            ))
        }
    }

    /// Insert multiple tenants in a single database transaction after validating each DTO.
    ///
    /// Each `TenantDTO`'s `id`, `name`, and `db_url` are validated before insertion; if any validation
    /// or the insert operation fails, the transaction is rolled back.
    ///
    /// # Returns
    ///
    /// The number of rows inserted.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::models::tenant::{Tenant, TenantDTO};
    ///
    /// let dtos = vec![TenantDTO {
    ///     id: "t1".into(),
    ///     name: "Tenant 1".into(),
    ///     db_url: "postgres://user:pass@localhost/db".into(),
    /// }];
    ///
    /// let mut conn = crate::config::db::establish_connection();
    /// let inserted = Tenant::batch_create(dtos, &mut conn).unwrap();
    /// assert_eq!(inserted, 1);
    /// ```
    pub fn batch_create(
        dtos: Vec<TenantDTO>,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<usize> {
        conn.transaction(|tx_conn| {
            dtos.iter().try_for_each(|dto| {
                Self::validate_tenant_dto(dto)?;
                Self::validate_db_url(&dto.db_url)
            })?;
            diesel::insert_into(tenants).values(&dtos).execute(tx_conn)
        })
    }

    /// Filter tenants by field-based conditions using iterator-backed pagination.
    ///
    /// Supported fields and operators:
    /// - `id`, `name`, `db_url`: `contains`, `equals`.
    ///   - `contains` is implemented with SQL `LIKE`; `%` and `_` in the value act as wildcards.
    /// - `created_at`, `updated_at`: `gt`, `gte`, `lt`, `lte`, `equals`. Date/time values must use ISO-like format `YYYY-MM-DDTHH:MM:SS.sssZ` (for example `2023-12-25T10:00:00.000Z`).
    ///
    /// Unknown fields or unsupported operators are ignored. Pagination parameters are normalized via the iterator-based pagination helper; an extra record is fetched per page to detect whether more data exists without materializing the full result set.
    ///
    /// Returns a `DatabaseError` when a date value cannot be parsed or when pagination parameters overflow supported bounds.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use crate::models::tenant::{Tenant, TenantFilter, FieldFilter};
    ///
    /// // let mut conn = /* obtain a mutable DB connection */;
    /// // let filter = TenantFilter {
    /// //     filters: vec![FieldFilter { field: "name".into(), operator: "contains".into(), value: "acme".into() }],
    /// //     page_size: Some(100),
    /// //     cursor: Some(0),
    /// // };
    /// // let tenant_page = Tenant::filter(filter, &mut conn).expect("query failed");
    /// // assert!(tenant_page.data.len() <= 100);
    /// ```
    pub fn filter(
        filter: TenantFilter,
        conn: &mut crate::config::db::Connection,
    ) -> QueryResult<Page<Tenant>> {
        // Validate cursor is non-negative
        if let Some(cursor) = filter.cursor {
            if cursor < 0 {
                return Err(result::Error::DatabaseError(
                    result::DatabaseErrorKind::Unknown,
                    Box::new("Cursor must be non-negative".to_string()),
                ));
            }
        }
        let query = filter.filters.iter().try_fold(
            tenants::table.into_boxed(),
            |acc, field_filter| -> QueryResult<_> {
                let mut acc = acc;
                match field_filter.field.as_str() {
                    "id" => match field_filter.operator.as_str() {
                        "contains" => {
                            acc = acc.filter(id.like(format!("%{}%", field_filter.value)))
                        }
                        "equals" => acc = acc.filter(id.eq(&field_filter.value)),
                        _ => {}
                    },
                    "name" => match field_filter.operator.as_str() {
                        "contains" => {
                            acc = acc.filter(name.like(format!("%{}%", field_filter.value)))
                        }
                        "equals" => acc = acc.filter(name.eq(&field_filter.value)),
                        _ => {}
                    },
                    "db_url" => match field_filter.operator.as_str() {
                        "contains" => {
                            acc = acc.filter(db_url.like(format!("%{}%", field_filter.value)))
                        }
                        "equals" => acc = acc.filter(db_url.eq(&field_filter.value)),
                        _ => {}
                    },
                    "created_at" | "updated_at" => {
                        let timestamp =
                            Self::parse_timestamp(&field_filter.value).ok_or_else(|| {
                                log::warn!("Invalid timestamp value: '{}'", field_filter.value);
                                result::Error::DatabaseError(
                                    result::DatabaseErrorKind::Unknown,
                                    Box::new(format!(
                                        "Invalid timestamp '{}' for field '{}'",
                                        field_filter.value, field_filter.field
                                    )),
                                )
                            })?;

                        acc = match (field_filter.field.as_str(), field_filter.operator.as_str()) {
                            ("created_at", "gt") => acc.filter(created_at.gt(timestamp)),
                            ("created_at", "gte") => acc.filter(created_at.ge(timestamp)),
                            ("created_at", "lt") => acc.filter(created_at.lt(timestamp)),
                            ("created_at", "lte") => acc.filter(created_at.le(timestamp)),
                            ("created_at", "equals") => acc.filter(created_at.eq(timestamp)),
                            ("updated_at", "gt") => acc.filter(updated_at.gt(timestamp)),
                            ("updated_at", "gte") => acc.filter(updated_at.ge(timestamp)),
                            ("updated_at", "lt") => acc.filter(updated_at.lt(timestamp)),
                            ("updated_at", "lte") => acc.filter(updated_at.le(timestamp)),
                            ("updated_at", "equals") => acc.filter(updated_at.eq(timestamp)),
                            _ => acc,
                        };
                    }
                    _ => {}
                }
                Ok(acc)
            },
        )?;

        // Normalize pagination using iterator-based helper (defaulting to constants::DEFAULT_PER_PAGE)
        let default_page_size = constants::DEFAULT_PER_PAGE as usize;
        let mut pagination = IteratorPagination::from_optional(
            filter.cursor.map(|value| value as i64),
            filter.page_size,
            default_page_size,
        );

        let mut page_size_i64 = i64::try_from(pagination.page_size()).map_err(|_| {
            result::Error::DatabaseError(
                result::DatabaseErrorKind::Unknown,
                Box::new("Page size is too large".to_string()),
            )
        })?;

        if page_size_i64 > MAX_PAGE_SIZE {
            page_size_i64 = MAX_PAGE_SIZE;
            pagination = IteratorPagination::new(pagination.cursor(), MAX_PAGE_SIZE as usize);
        }

        let cursor_i64 = i64::try_from(pagination.cursor()).map_err(|_| {
            result::Error::DatabaseError(
                result::DatabaseErrorKind::Unknown,
                Box::new("Cursor is too large".to_string()),
            )
        })?;

        let limit = page_size_i64.checked_add(1).ok_or_else(|| {
            result::Error::DatabaseError(
                result::DatabaseErrorKind::Unknown,
                Box::new("Limit calculation would overflow".to_string()),
            )
        })?;

        let offset = cursor_i64.checked_mul(page_size_i64).ok_or_else(|| {
            result::Error::DatabaseError(
                result::DatabaseErrorKind::Unknown,
                Box::new("Offset calculation would overflow".to_string()),
            )
        })?;

        let mut query = query;
        query = query.limit(limit).offset(offset);

        let mut results = query.load::<Tenant>(conn)?;

        let mut has_more = false;
        if results.len() as i64 > page_size_i64 {
            has_more = true;
            results.truncate(page_size_i64 as usize);
        }

        let paginated = PaginatedPage::from_items(results, pagination, has_more, None);

        let to_i32 = |value: usize| -> i32 {
            match i32::try_from(value) {
                Ok(v) => v,
                Err(_) => {
                    log::warn!(
                        "Cursor value {} exceeds i32::MAX, truncating to i32::MAX",
                        value
                    );
                    i32::MAX
                }
            }
        };

        let page = Page::new(
            MESSAGE_OK,
            paginated.items,
            to_i32(paginated.summary.current_cursor),
            paginated.summary.page_size as i64,
            paginated
                .summary
                .total_elements
                .map(|total| total.min(i64::MAX as usize) as i64),
            paginated.summary.next_cursor.map(|cursor| to_i32(cursor)),
        );

        Ok(page)
    }
}
