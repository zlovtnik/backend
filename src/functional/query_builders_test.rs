#[cfg(test)]
mod tests {
    use super::*;
    use crate::functional::query_builder::{contains, not_contains};
    use diesel::query_builder::debug_query;
    
    #[test]
    fn test_contains_escaping() {
        // Test that special characters are properly escaped
        let filter = crate::functional::query_builder::QueryFilter::new()
            .with_predicate(contains(
                "tenants".to_string(),
                "name".to_string(),
                "test%value".to_string(),
                "name".to_string(),
            ));
        
        let builder = TenantQueryBuilder::new().filter(filter);
        // Build the query
        let query = builder.build_tenant_query().unwrap();
        // Get the SQL representation
        let sql = diesel::query_builder::debug_query(&query).to_string();
        // Assert that the percent sign is escaped (should be test\%value in the pattern)
        assert!(sql.contains("test\\\\%value"), "SQL should contain escaped percent sign: {}", sql);
    }
    
    #[test]
    fn test_not_contains_escaping() {
        // Test that special characters are properly escaped
        let filter = crate::functional::query_builder::QueryFilter::new()
            .with_predicate(not_contains(
                "tenants".to_string(),
                "name".to_string(),
                "test_value".to_string(),
                "name".to_string(),
            ));
        
        let builder = TenantQueryBuilder::new().filter(filter);
        // Build the query
        let query = builder.build_tenant_query().unwrap();
        // Get the SQL representation
        let sql = diesel::query_builder::debug_query(&query).to_string();
        // Assert that the underscore is escaped (should be test\_value in the pattern)
        assert!(sql.contains("test\\\\_value"), "SQL should contain escaped underscore: {}", sql);
    }
}
