//! Dummy schema for functional_lib

diesel::table! {
    tenants (id) {
        #[max_length = 36]
        id -> Varchar,
        name -> Varchar,
        db_url -> Text,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}
