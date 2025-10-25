//! Table-specific query builders that extend TypeSafeQueryBuilder with actual Diesel query building.
//!
//! This module provides implementations of TypeSafeQueryBuilder for specific tables,
//! enabling the functional query composition system to generate real Diesel SQL fragments.

use chrono::NaiveDateTime;
use diesel::pg::Pg;
use diesel::prelude::*;

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
            "id" => Self::apply_string_operator(query, "id", &predicate.operator, &predicate.value),
            "name" => {
                Self::apply_string_operator(query, "name", &predicate.operator, &predicate.value)
            }
            "db_url" => {
                Self::apply_string_operator(query, "db_url", &predicate.operator, &predicate.value)
            }
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
            .replace("%", "\\%") // Escape percent signs
            .replace("_", "\\_") // Escape underscores
    }

    /// Applies an operator to a string-like column (varchar or text) using Diesel's type-safe DSL.
    fn apply_string_operator(
        query: tenants::BoxedQuery<'static, Pg>,
        column_name: &str,
        operator: &Operator,
        value: &Option<String>,
    ) -> Result<tenants::BoxedQuery<'static, Pg>, String> {
        use diesel::expression_methods::ExpressionMethods;

        macro_rules! apply_string_op {
            ($column:expr) => {
                match operator {
                    Operator::Equals => {
                        let v = value
                            .as_ref()
                            .ok_or("Value required for string column predicate")?
                            .clone();
                        Ok(query.filter($column.eq(v)))
                    }
                    Operator::NotEquals => {
                        let v = value
                            .as_ref()
                            .ok_or("Value required for string column predicate")?
                            .clone();
                        Ok(query.filter($column.ne(v)))
                    }
                    Operator::Contains => {
                        let v = value
                            .as_ref()
                            .ok_or("Value required for string column predicate")?;
                        let pattern = format!("%{}%", Self::escape_like_pattern(v));
                        Ok(query.filter($column.like(pattern)))
                    }
                    Operator::NotContains => {
                        let v = value
                            .as_ref()
                            .ok_or("Value required for string column predicate")?;
                        let pattern = format!("%{}%", Self::escape_like_pattern(v));
                        Ok(query.filter($column.not_like(pattern)))
                    }
                    Operator::IsNull => Ok(query.filter($column.is_null())),
                    Operator::IsNotNull => Ok(query.filter($column.is_not_null())),
                    _ => Err(format!(
                        "Operator {:?} not supported for string columns",
                        operator
                    )),
                }
            };
        }

        match column_name {
            "id" => apply_string_op!(tenants::id),
            "name" => apply_string_op!(tenants::name),
            "db_url" => apply_string_op!(tenants::db_url),
            _ => Err(format!("Unknown field '{}' for tenants table", column_name)),
        }
    }

    /// Applies an operator to a timestamp column (created_at, updated_at) using Diesel's type-safe DSL.
    fn apply_timestamp_operator(
        query: tenants::BoxedQuery<'static, Pg>,
        column_name: &str,
        operator: &Operator,
        value: &Option<String>,
    ) -> Result<tenants::BoxedQuery<'static, Pg>, String> {
        use diesel::expression_methods::ExpressionMethods;

        macro_rules! apply_timestamp_op {
            ($column:expr) => {
                match operator {
                    Operator::Equals => {
                        let v = value
                            .as_ref()
                            .ok_or("Value required for timestamp predicate")?;
                        let timestamp = Self::parse_timestamp(v)?;
                        Ok(query.filter($column.eq(timestamp)))
                    }
                    Operator::NotEquals => {
                        let v = value
                            .as_ref()
                            .ok_or("Value required for timestamp predicate")?;
                        let timestamp = Self::parse_timestamp(v)?;
                        Ok(query.filter($column.ne(timestamp)))
                    }
                    Operator::GreaterThan => {
                        let v = value
                            .as_ref()
                            .ok_or("Value required for timestamp predicate")?;
                        let timestamp = Self::parse_timestamp(v)?;
                        Ok(query.filter($column.gt(timestamp)))
                    }
                    Operator::LessThan => {
                        let v = value
                            .as_ref()
                            .ok_or("Value required for timestamp predicate")?;
                        let timestamp = Self::parse_timestamp(v)?;
                        Ok(query.filter($column.lt(timestamp)))
                    }
                    Operator::GreaterThanEqual => {
                        let v = value
                            .as_ref()
                            .ok_or("Value required for timestamp predicate")?;
                        let timestamp = Self::parse_timestamp(v)?;
                        Ok(query.filter($column.ge(timestamp)))
                    }
                    Operator::LessThanEqual => {
                        let v = value
                            .as_ref()
                            .ok_or("Value required for timestamp predicate")?;
                        let timestamp = Self::parse_timestamp(v)?;
                        Ok(query.filter($column.le(timestamp)))
                    }
                    Operator::IsNull => Ok(query.filter($column.is_null())),
                    Operator::IsNotNull => Ok(query.filter($column.is_not_null())),
                    _ => Err(format!(
                        "Operator {:?} not supported for timestamp columns",
                        operator
                    )),
                }
            };
        }

        match column_name {
            "created_at" => apply_timestamp_op!(tenants::created_at),
            "updated_at" => apply_timestamp_op!(tenants::updated_at),
            _ => Err(format!("Unknown field '{}' for tenants table", column_name)),
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
        use diesel::expression_methods::ExpressionMethods;

        // Macro to reduce duplication in ordering logic
        macro_rules! apply_order {
            ($column:expr) => {
                if order_spec.ascending {
                    Ok(query.order($column.asc()))
                } else {
                    Ok(query.order($column.desc()))
                }
            };
        }

        match order_spec.column.as_str() {
            "id" => apply_order!(tenants::id),
            "name" => apply_order!(tenants::name),
            "db_url" => apply_order!(tenants::db_url),
            "created_at" => apply_order!(tenants::created_at),
            "updated_at" => apply_order!(tenants::updated_at),
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

    #[test]
    fn test_escape_like_pattern_with_percent() {
        let input = "50%";
        let escaped = TenantQueryBuilder::escape_like_pattern(input);
        assert_eq!(escaped, "50\\%");
    }

    #[test]
    fn test_escape_like_pattern_with_underscore() {
        let input = "test_name";
        let escaped = TenantQueryBuilder::escape_like_pattern(input);
        assert_eq!(escaped, "test\\_name");
    }

    #[test]
    fn test_escape_like_pattern_with_backslash() {
        let input = "path\\to\\file";
        let escaped = TenantQueryBuilder::escape_like_pattern(input);
        assert_eq!(escaped, "path\\\\to\\\\file");
    }

    #[test]
    fn test_escape_like_pattern_with_all_special_chars() {
        let input = "50%_path\\to";
        let escaped = TenantQueryBuilder::escape_like_pattern(input);
        // Backslashes are escaped first, then % and _
        // Input: "50%_path\to"
        // After escaping backslashes: "50%_path\\to"
        // After escaping %: "50\%_path\\to"
        // After escaping _: "50\%\_path\\to"
        assert_eq!(escaped, "50\\%\\_path\\\\to");
    }

    #[test]
    fn test_escape_like_pattern_no_double_escaping() {
        // Ensure we don't double-escape backslashes
        let input = "path\\to";
        let escaped = TenantQueryBuilder::escape_like_pattern(input);
        // Should be: "path\\to" (backslash becomes \\)
        assert_eq!(escaped, "path\\\\to");
        // Should NOT be: "path\\\\to" (double-escaped)
        assert_ne!(escaped, "path\\\\\\\\to");
    }

    #[test]
    fn test_like_pattern_formatting_for_contains() {
        // Test that Contains operator wraps pattern with % for substring matching
        let input = "test";
        let escaped = TenantQueryBuilder::escape_like_pattern(input);
        let pattern = format!("%{}%", escaped);
        assert_eq!(pattern, "%test%");
    }

    #[test]
    fn test_like_pattern_formatting_with_special_chars() {
        // Test pattern generation with special characters
        let input = "50%_path";
        let escaped = TenantQueryBuilder::escape_like_pattern(input);
        let pattern = format!("%{}%", escaped);
        // Escaped version is "50\%\_path"
        assert_eq!(pattern, "%50\\%\\_path%");
    }

    #[test]
    fn test_parse_timestamp_iso_format_with_microseconds() {
        let result = TenantQueryBuilder::parse_timestamp("2023-10-24T15:30:45.123456Z");
        assert!(result.is_ok());
        let ts = result.unwrap();
        assert_eq!(ts.format("%Y-%m-%d").to_string(), "2023-10-24");
    }

    #[test]
    fn test_parse_timestamp_iso_format_without_microseconds() {
        let result = TenantQueryBuilder::parse_timestamp("2023-10-24T15:30:45Z");
        assert!(result.is_ok());
        let ts = result.unwrap();
        assert_eq!(ts.format("%Y-%m-%d").to_string(), "2023-10-24");
    }

    #[test]
    fn test_parse_timestamp_invalid_format() {
        let result = TenantQueryBuilder::parse_timestamp("not-a-timestamp");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid timestamp format"));
    }

    #[test]
    fn test_escape_like_pattern_empty_string() {
        let input = "";
        let escaped = TenantQueryBuilder::escape_like_pattern(input);
        assert_eq!(escaped, "");
    }

    #[test]
    fn test_escape_like_pattern_only_special_chars() {
        let input = "%_%\\";
        let escaped = TenantQueryBuilder::escape_like_pattern(input);
        // Backslashes first: "%_%\\" → "%_%\\\\"
        // Then %: "%_%\\\\" → "\\%_%\\\\"
        // Then _: "\\%_%\\\\" → "\\%\\_\\\\"
        assert_eq!(escaped, "\\%\\_\\\\");
    }
}
