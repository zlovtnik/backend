//! Table-specific query builders that extend TypeSafeQueryBuilder with actual Diesel query building.
//!
//! This module provides implementations of TypeSafeQueryBuilder for specific tables,
//! enabling the functional query composition system to generate real Diesel SQL fragments.

use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::query_builder::QueryFragment;

use crate::functional::query_builder::{TypeSafeQueryBuilder, Operator};
use crate::schema::tenants;
use crate::schema::tenants::dsl::*;

/// Tenant-specific query builder that can generate actual Diesel queries for the tenants table.
///
/// This builder extends the generic TypeSafeQueryBuilder with the ability to map field names
/// to actual Diesel column references and build parameterized SQL queries.
pub type TenantQueryBuilder = TypeSafeQueryBuilder<tenants::table, String>;

impl TenantQueryBuilder {
    /// Builds a Diesel SQL fragment for the tenants table using the accumulated filters and predicates.
    ///
    /// This method iterates through all filters and predicates, mapping field names to actual
    /// Diesel column references and applying the appropriate query operations.
    ///
    /// # Returns
    ///
    /// A `Result` containing either a boxed Diesel `QueryFragment<Pg>` representing the complete
    /// parameterized query, or a `String` error message if query construction fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use crate::functional::query_builders::TenantQueryBuilder;
    /// use crate::functional::query_builder::{equals, contains};
    /// use diesel::prelude::*;
    ///
    /// // Build a query for tenants with specific criteria
    /// let query_builder = TenantQueryBuilder::new()
    ///     .filter(equals(tenants::id, "tenant123".to_string(), "id".to_string()))
    ///     .filter(contains(tenants::name, "acme".to_string(), "name".to_string()))
    ///     .limit(50);
    ///
    /// let query = query_builder.build().expect("Failed to build query");
    /// // query can now be executed against the database
    /// ```
    pub fn build(self) -> Result<Box<dyn QueryFragment<Pg> + Send>, String> {
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

        Ok(Box::new(query))
    }

    /// Applies a single predicate to the query by mapping field names to Diesel columns.
    fn apply_predicate(
        query: tenants::BoxedQuery<'static, Pg>,
        predicate: &crate::functional::query_builder::Predicate<String>,
    ) -> Result<tenants::BoxedQuery<'static, Pg>, String> {
        match predicate.column.column.as_str() {
            "id" => Self::apply_operator(query, tenants::id, &predicate.operator, &predicate.value),
            "name" => Self::apply_operator(query, tenants::name, &predicate.operator, &predicate.value),
            "db_url" => Self::apply_operator(query, tenants::db_url, &predicate.operator, &predicate.value),
            "created_at" => Self::apply_timestamp_operator(query, tenants::created_at, &predicate.operator, &predicate.value),
            "updated_at" => Self::apply_timestamp_operator(query, tenants::updated_at, &predicate.operator, &predicate.value),
            _ => Err(format!("Unknown field '{}' for tenants table", predicate.column.column)),
        }
    }

    /// Applies an operator to a string column (id, name, db_url).
    fn apply_operator(
        query: tenants::BoxedQuery<'static, Pg>,
        column: tenants::columns::Column,
        operator: &Operator,
        value: &Option<String>,
    ) -> Result<tenants::BoxedQuery<'static, Pg>, String> {
        let value = value.as_ref().ok_or("Value required for string column predicate")?;

        match operator {
            Operator::Equals => Ok(query.filter(column.eq(value))),
            Operator::NotEquals => Ok(query.filter(column.ne(value))),
            Operator::Contains => Ok(query.filter(column.like(format!("%{}%", value)))),
            Operator::NotContains => Ok(query.filter(column.not_like(format!("%{}%", value)))),
            Operator::IsNull => Ok(query.filter(column.is_null())),
            Operator::IsNotNull => Ok(query.filter(column.is_not_null())),
            _ => Err(format!("Operator {:?} not supported for string columns", operator)),
        }
    }

    /// Applies an operator to a timestamp column (created_at, updated_at).
    fn apply_timestamp_operator(
        query: tenants::BoxedQuery<'static, Pg>,
        column: tenants::columns::Column,
        operator: &Operator,
        value: &Option<String>,
    ) -> Result<tenants::BoxedQuery<'static, Pg>, String> {
        let value = value.as_ref().ok_or("Value required for timestamp predicate")?;

        // Parse the timestamp string
        let timestamp = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.fZ")
            .or_else(|_| chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%SZ"))
            .map_err(|_| format!("Invalid timestamp format: {}", value))?;

        match operator {
            Operator::Equals => Ok(query.filter(column.eq(timestamp))),
            Operator::NotEquals => Ok(query.filter(column.ne(timestamp))),
            Operator::GreaterThan => Ok(query.filter(column.gt(timestamp))),
            Operator::LessThan => Ok(query.filter(column.lt(timestamp))),
            Operator::GreaterThanEqual => Ok(query.filter(column.ge(timestamp))),
            Operator::LessThanEqual => Ok(query.filter(column.le(timestamp))),
            Operator::IsNull => Ok(query.filter(column.is_null())),
            Operator::IsNotNull => Ok(query.filter(column.is_not_null())),
            _ => Err(format!("Operator {:?} not supported for timestamp columns", operator)),
        }
    }

    /// Applies ordering to the query.
    fn apply_ordering(
        query: tenants::BoxedQuery<'static, Pg>,
        order_spec: &crate::functional::query_builder::OrderSpec,
    ) -> Result<tenants::BoxedQuery<'static, Pg>, String> {
        let column = match order_spec.column.as_str() {
            "id" => tenants::id,
            "name" => tenants::name,
            "db_url" => tenants::db_url,
            "created_at" => tenants::created_at,
            "updated_at" => tenants::updated_at,
            _ => return Err(format!("Unknown ordering column '{}'", order_spec.column)),
        };

        if order_spec.ascending {
            Ok(query.then_order_by(column.asc()))
        } else {
            Ok(query.then_order_by(column.desc()))
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
        let filter = crate::functional::query_builder::QueryFilter::new()
            .with_predicate(equals(
                crate::functional::query_builder::Column::new("tenants".to_string(), "name".to_string()),
                "test_tenant".to_string(),
                "name".to_string(),
            ));

        let builder = TenantQueryBuilder::new().filter(filter);
        assert_eq!(builder.filters().len(), 1);
    }
}
