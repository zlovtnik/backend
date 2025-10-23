pub mod token_utils;

use uuid::Uuid;

pub fn generate_tenant_id() -> String {
    Uuid::new_v4().to_string()
}
