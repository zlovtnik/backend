//! Table-specific query builders that extend TypeSafeQueryBuilder with actual Diesel query building.
//!
//! This module provides implementations of TypeSafeQueryBuilder for specific tables,
//! enabling the functional query composition system to generate real Diesel SQL fragments.

use chrono::NaiveDateTime;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::ExpressionMethods;

use crate::functional::query_builder::{Operator, TypeSafeQueryBuilder};
use crate::schema::tenants;

/// Tenant-specific query builder that can generate actual Diesel queries for the tenants table.
///
/// This builder extends the generic TypeSafeQueryBuilder with the ability to map field names
/// to actual Diesel column references and build parameterized SQL queries.
pub type TenantQueryBuilder = TypeSafeQueryBuilder<tenants::table, String>;

impl TenantQueryBuilder {
    /// Builds the final Diesel query from the accumulated filters and ordering.
    ///
    /// This method applies all the configured filters, ordering, and pagination
    /// to create a boxed Diesel query that can be executed against the database.
    ///
    /// # Returns
    ///
    /// A `Result` containing the boxed Diesel query on success, or an error message on failure.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::functional::query_builders::TenantQueryBuilder;
    /// use diesel::prelude::*;
    ///
    /// let builder = TenantQueryBuilder::new()
    ///     .with_filter("name", "John")
    ///     .order_by("created_at", true);
    ///
    /// let query = builder.build_tenant_query().expect("Failed to build query");
    /// let results: Vec<Tenant> = query.load(&conn).expect("Failed to execute query");
    /// ```
    pub fn build_tenant_query(self) -> Result<tenants::BoxedQuery<'static, Pg>, String> {
        // Start with the tenants table
        let mut query = tenants::table.into_boxed();

        // Apply all filters and their predicates
        for filter in self.filters {
            for predicate in filter.predicates() {
                query = Self::apply_predicate(query, predicate)?;
            }
        }

        // Apply ordering
        for order_spec in self.order_by {
            query = Self::apply_ordering(query, &order_spec)?;
        }

        // Apply limit and offset
        if let Some(limit_val) = self.limit {
            query = query.limit(limit_val);
        }
        if let Some(offset_val) = self.offset {
            query = query.offset(offset_val);
        }

        Ok(query)
    }

    /// Applies a single predicate to the query by mapping field names to Diesel columns.
    fn apply_predicate(
        query: tenants::BoxedQuery<'static, Pg>,
        predicate: &crate::functional::query_builder::Predicate<String>,
    ) -> Result<tenants::BoxedQuery<'static, Pg>, String> {
        match predicate.column.column.as_str() {
            "id" => Self::apply_string_operator(
                query,
                "id",
                &predicate.operator,
                &predicate.value,
            ),
            "name" => Self::apply_string_operator(
                query,
                "name",
                &predicate.operator,
                &predicate.value,
            ),
            "db_url" => Self::apply_string_operator(
                query,
                "db_url",
                &predicate.operator,
                &predicate.value,
            ),
            "created_at" => Self::apply_timestamp_operator(
                query,
                "created_at",
                &predicate.operator,
                &predicate.value,
            ),
            "updated_at" => Self::apply_timestamp_operator(
                query,
                "updated_at",
                &predicate.operator,
                &predicate.value,
            ),
            _ => Err(format!(
                "Unknown field '{}' for tenants table",
                predicate.column.column
            )),
        }
    }

    /// Escapes a value for use in LIKE patterns by properly handling SQL wildcards.
    ///
    /// This method escapes backslashes first, then percent signs (%) and underscores (_)
    /// to prevent them from being interpreted as wildcards in LIKE queries.
    fn escape_like_pattern(value: &str) -> String {
        value
            .replace("\\", "\\\\") // Escape backslashes first
            .replace("%", "\\%")   // Escape percent signs
            .replace("_", "\\_")   // Escape underscores
    }

    /// Applies an operator to a string-like column (varchar or text).
    fn apply_string_operator(
        query: tenants::BoxedQuery<'static, Pg>,
        column_name: &str,
        operator: &Operator,
        value: &Option<String>,
    ) -> Result<tenants::BoxedQuery<'static, Pg>, String> {
        use diesel::dsl::sql;

        match operator {
            Operator::Equals => {
                let value = value
                    .as_ref()
                    .ok_or("Value required for string column predicate")?;
                let sql_str = format!("{} = ", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str).bind::<diesel::sql_types::Text, String>(value.clone())))
            }
            Operator::NotEquals => {
                let value = value
                    .as_ref()
                    .ok_or("Value required for string column predicate")?;
                let sql_str = format!("{} != ", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str).bind::<diesel::sql_types::Text, String>(value.clone())))
            }
            Operator::Contains => {
                let value = value
                    .as_ref()
                    .ok_or("Value required for string column predicate")?;
                let pattern = format!("%{}%", Self::escape_like_pattern(value));
                let sql_str = format!("{} LIKE ", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str).bind::<diesel::sql_types::Text, String>(pattern)))
            }
            Operator::NotContains => {
                let value = value
                    .as_ref()
                    .ok_or("Value required for string column predicate")?;
                let pattern = format!("%{}%", Self::escape_like_pattern(value));
                let sql_str = format!("{} NOT LIKE ", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str).bind::<diesel::sql_types::Text, String>(pattern)))
            }
            Operator::IsNull => {
                let sql_str = format!("{} IS NULL", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str)))
            }
            Operator::IsNotNull => {
                let sql_str = format!("{} IS NOT NULL", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str)))
            }
            _ => Err(format!(
                "Operator {:?} not supported for string columns",
                operator
            )),
        }
    }

    /// Applies an operator to a timestamp column (created_at, updated_at).
    fn apply_timestamp_operator(
        query: tenants::BoxedQuery<'static, Pg>,
        column_name: &str,
        operator: &Operator,
        value: &Option<String>,
    ) -> Result<tenants::BoxedQuery<'static, Pg>, String> {
        use diesel::dsl::sql;

        match operator {
            Operator::Equals => {
                let value = value
                    .as_ref()
                    .ok_or("Value required for timestamp predicate")?;
                let timestamp = Self::parse_timestamp(value)?;
                let sql_str = format!("{} = ", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str).bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, Option<NaiveDateTime>>(Some(timestamp))))
            }
            Operator::NotEquals => {
                let value = value
                    .as_ref()
                    .ok_or("Value required for timestamp predicate")?;
                let timestamp = Self::parse_timestamp(value)?;
                let sql_str = format!("{} != ", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str).bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, Option<NaiveDateTime>>(Some(timestamp))))
            }
            Operator::GreaterThan => {
                let value = value
                    .as_ref()
                    .ok_or("Value required for timestamp predicate")?;
                let timestamp = Self::parse_timestamp(value)?;
                let sql_str = format!("{} > ", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str).bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, Option<NaiveDateTime>>(Some(timestamp))))
            }
            Operator::LessThan => {
                let value = value
                    .as_ref()
                    .ok_or("Value required for timestamp predicate")?;
                let timestamp = Self::parse_timestamp(value)?;
                let sql_str = format!("{} < ", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str).bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, Option<NaiveDateTime>>(Some(timestamp))))
            }
            Operator::GreaterThanEqual => {
                let value = value
                    .as_ref()
                    .ok_or("Value required for timestamp predicate")?;
                let timestamp = Self::parse_timestamp(value)?;
                let sql_str = format!("{} >= ", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str).bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, Option<NaiveDateTime>>(Some(timestamp))))
            }
            Operator::LessThanEqual => {
                let value = value
                    .as_ref()
                    .ok_or("Value required for timestamp predicate")?;
                let timestamp = Self::parse_timestamp(value)?;
                let sql_str = format!("{} <= ", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str).bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, Option<NaiveDateTime>>(Some(timestamp))))
            }
            Operator::IsNull => {
                let sql_str = format!("{} IS NULL", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str)))
            }
            Operator::IsNotNull => {
                let sql_str = format!("{} IS NOT NULL", column_name);
                Ok(query.filter(sql::<diesel::sql_types::Bool>(&sql_str)))
            }
            _ => Err(format!(
                "Operator {:?} not supported for timestamp columns",
                operator
            )),
        }
    }

    /// Parses a timestamp string in ISO format
    fn parse_timestamp(value: &str) -> Result<NaiveDateTime, String> {
        NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.fZ")
            .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%SZ"))
            .map_err(|_| format!("Invalid timestamp format: {}", value))
    }

    /// Applies ordering to the query.
    fn apply_ordering(
        query: tenants::BoxedQuery<'static, Pg>,
        order_spec: &crate::functional::query_builder::OrderSpec,
    ) -> Result<tenants::BoxedQuery<'static, Pg>, String> {
        match order_spec.column.as_str() {
            "id" => if order_spec.ascending {
                Ok(query.order(tenants::id.asc()))
            } else {
                Ok(query.order(tenants::id.desc()))
            },
            "name" => if order_spec.ascending {
                Ok(query.order(tenants::name.asc()))
            } else {
                Ok(query.order(tenants::name.desc()))
            },
            "db_url" => if order_spec.ascending {
                Ok(query.order(tenants::db_url.asc()))
            } else {
                Ok(query.order(tenants::db_url.desc()))
            },
            "created_at" => if order_spec.ascending {
                Ok(query.order(tenants::created_at.asc()))
            } else {
                Ok(query.order(tenants::created_at.desc()))
            },
            "updated_at" => if order_spec.ascending {
                Ok(query.order(tenants::updated_at.asc()))
            } else {
                Ok(query.order(tenants::updated_at.desc()))
            },
            _ => Err(format!("Unknown ordering column '{}'", order_spec.column)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::functional::query_builder::equals;

    #[test]
    fn test_tenant_query_builder_creation() {
        let builder = TenantQueryBuilder::new();
        assert!(builder.filters().is_empty());
        assert!(builder.order_by_specs().is_empty());
    }

    #[test]
    fn test_tenant_query_builder_with_filters() {
        let filter = crate::functional::query_builder::QueryFilter::new().with_predicate(equals(
            crate::functional::query_builder::Column::new(
                "tenants".to_string(),
                "name".to_string(),
            ),
            "test_tenant".to_string(),
            "name".to_string(),
        ));

        let builder = TenantQueryBuilder::new().filter(filter);
        assert_eq!(builder.filters().len(), 1);
    }
}
