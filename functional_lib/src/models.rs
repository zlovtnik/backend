//! Dummy models for functional_lib to compile

pub mod tenant {
    use chrono::NaiveDateTime;

    #[derive(Debug, Clone)]
    pub struct Tenant {
        pub id: String,
        pub name: String,
        pub db_url: String,
        pub created_at: Option<NaiveDateTime>,
        pub updated_at: Option<NaiveDateTime>,
    }
}

pub mod person {
    #[derive(Debug, Clone)]
    pub struct PersonDTO {
        pub name: String,
        pub age: i32,
        pub email: String,
        pub address: String,
        pub phone: String,
        pub gender: bool,
    }
}

pub mod response {
    use serde::Serialize;

    #[derive(Debug, Clone, Serialize)]
    pub struct ResponseBody {
        pub message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub data: Option<serde_json::Value>,
    }

    impl ResponseBody {
        pub fn new(message: &str) -> Self {
            Self {
                message: message.to_string(),
                data: None,
            }
        }

        pub fn with_data(message: &str, data: serde_json::Value) -> Self {
            Self {
                message: message.to_string(),
                data: Some(data),
            }
        }
    }
}

pub mod filters {
    #[derive(Debug, Clone)]
    pub struct FieldFilter {
        pub field: String,
        pub operator: String,
        pub value: String,
    }
}