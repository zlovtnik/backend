use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct PersonFilter {
    pub name: Option<String>,
    pub gender: Option<String>,
    pub age: Option<i32>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub cursor: Option<i32>,
    pub page_size: Option<i64>,
    #[serde(default)]
    pub page_num: Option<i32>,
    #[serde(default)]
    pub sort_by: Option<String>,
    #[serde(default)]
    pub sort_order: Option<String>,
}

#[derive(Deserialize)]
pub struct TenantFilter {
    #[serde(default)]
    pub filters: Vec<FieldFilter>,
    pub cursor: Option<i32>,
    pub page_size: Option<i64>,
}

#[derive(Deserialize)]
pub struct FieldFilter {
    pub field: String,    // "name", "id", "db_url", "created_at", "updated_at"
    pub operator: String, // "contains", "equals", "gt", "lt", "gte", "lte"
    pub value: String,
}
