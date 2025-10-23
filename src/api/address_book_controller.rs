use actix_web::http::StatusCode;
use actix_web::{web, HttpMessage, HttpRequest, HttpResponse, Responder, Result};
use serde_json::json;
use std::borrow::Cow;

use crate::{
    config::db::Pool,
    constants,
    error::ServiceError,
    functional::{
        pagination::Pagination,
        response_transformers::{ResponseTransformError, ResponseTransformer},
    },
    models::{
        filters::PersonFilter,
        person::{Person, PersonDTO},
    },
    services::{address_book_service, functional_service_base::FunctionalErrorHandling},
};

fn response_composition_error(err: ResponseTransformError) -> ServiceError {
    ServiceError::internal_server_error(constants::MESSAGE_INTERNAL_SERVER_ERROR)
        .with_tag("response")
        .with_detail(err.to_string())
}

fn respond_empty(req: &HttpRequest, status: StatusCode, message: &str) -> HttpResponse {
    ResponseTransformer::new(constants::EMPTY)
        .with_message(Cow::Owned(message.to_string()))
        .with_status(status)
        .respond_to(req)
}

fn respond_with_page(
    req: &HttpRequest,
    page: crate::models::response::Page<Person>,
) -> Result<HttpResponse, ServiceError> {
    let crate::models::response::Page {
        message,
        data,
        current_cursor,
        page_size,
        total_elements,
        next_cursor,
    } = page;

    let count = data.len();
    let metadata = json!({
        "current_cursor": current_cursor,
        "page_size": page_size,
        "total_elements": total_elements,
        "next_cursor": next_cursor,
        "count": count,
        "has_more": next_cursor.is_some(),
    });

    ResponseTransformer::new(data)
        .with_message(Cow::Owned(message))
        .try_with_metadata(metadata)
        .map(|transformer| transformer.respond_to(req))
        .map_err(response_composition_error)
}

/// Extract the database pool from the request extensions.
///
/// Returns the pool if present, otherwise returns a ServiceError indicating
/// that the tenant pool is missing from the request extensions.
fn extract_pool(req: &HttpRequest) -> Result<Pool, ServiceError> {
    req.extensions().get::<Pool>().cloned().ok_or_else(|| {
        ServiceError::internal_server_error("Pool not found")
            .with_detail("Missing tenant pool in request extensions")
            .with_tag("tenant")
    })
}
// GET api/address-book
/// Retrieve all people from the address book and return them in a standard JSON response.
///
/// # Returns
///
/// `HttpResponse` containing a JSON `ResponseBody` with the list of people and an OK message,
/// or a `ServiceError` if the database pool is missing or the service call fails.
///
/// # Examples
///
/// ```
/// use actix_web::HttpRequest;
/// // Construct a request with a Pool placed into its extensions before calling.
/// // Here we show the call shape; setting up a real Pool is omitted for brevity.
/// let req = HttpRequest::default();
/// let result = actix_rt::System::new().block_on(crate::api::address_book_controller::find_all(req));
/// // `result` will be `Ok(HttpResponse)` on success or `Err(ServiceError)` on failure.
/// ```
pub async fn find_all(
    query: web::Query<std::collections::HashMap<String, String>>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let pagination = Pagination::from_optional(
        query
            .get("cursor")
            .or_else(|| query.get("offset"))
            .and_then(|value| value.parse::<i64>().ok()),
        query
            .get("limit")
            .and_then(|value| value.parse::<i64>().ok())
            .map(|limit| limit.min(500)),
        50,
    );

    let pool = extract_pool(&req)?;

    // Use database-level pagination with Person::filter instead of loading all records
    let filter = PersonFilter {
        name: None,
        gender: None,
        age: None,
        phone: None,
        email: None,
        cursor: Some(pagination.cursor() as i32),
        page_size: Some(pagination.page_size() as i64),
        page_num: None,
        sort_by: None,
        sort_order: None,
    };

    address_book_service::filter(filter, &pool)
        .log_error("address_book_controller::find_all")
        .and_then(|page| respond_with_page(&req, page))
}

// GET api/address-book/{id}
/// Retrieve a person by ID and return an HTTP 200 response containing the person as JSON.
///
/// On success returns an `HttpResponse::Ok` whose JSON body is a `ResponseBody` wrapping the found person.
/// On failure returns a `ServiceError` (for example when the DB pool is missing or the service reports an error).
///
/// # Examples
///
/// ```
/// # use actix_web::{test, web, HttpRequest};
/// # use crate::api::address_book_controller::find_by_id;
/// # async fn _example() {
/// let req = test::TestRequest::default().to_http_request();
/// let _ = find_by_id(web::Path::from(1), req).await;
/// # }
/// ```
pub async fn find_by_id(
    id: web::Path<i32>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let pool = extract_pool(&req)?;
    address_book_service::find_by_id(id.into_inner(), &pool)
        .log_error("address_book_controller::find_by_id")
        .map(|person| ResponseTransformer::new(person).respond_to(&req))
}

// GET api/address-book/filter
pub async fn filter(
    query: web::Query<PersonFilter>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    use log::{debug, error};

    let mut filter = query.into_inner();
    debug!("Filter endpoint called with filter: {:?}", filter);

    let pool = match extract_pool(&req) {
        Ok(pool) => {
            debug!("Successfully extracted pool from request");
            pool
        }
        Err(e) => {
            error!("Failed to extract pool from request: {}", e);
            return Err(e);
        }
    };

    let pagination = Pagination::from_optional(filter.cursor.map(i64::from), filter.page_size, 50);
    filter.cursor.get_or_insert(pagination.cursor() as i32);
    filter
        .page_size
        .get_or_insert(pagination.page_size() as i64);

    debug!("Calling address_book_service::filter");
    address_book_service::filter(filter, &pool)
        .log_error("address_book_controller::filter")
        .and_then(|page| {
            debug!(
                "Filter operation successful, returning {} results",
                page.data.len()
            );
            respond_with_page(&req, page)
        })
}

// POST api/address-book
pub async fn insert(
    new_person: web::Json<PersonDTO>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let pool = extract_pool(&req)?;
    address_book_service::insert(new_person.into_inner(), &pool)
        .log_error("address_book_controller::insert")
        .map(|_| respond_empty(&req, StatusCode::CREATED, constants::MESSAGE_OK))
}

// PUT api/address-book/{id}
/// Updates an existing person identified by `id` with the provided `updated_person` data.
///
/// On success returns an HTTP 200 response with a `ResponseBody` containing an OK message and an empty payload.
/// Returns a `ServiceError::InternalServerError` with message "Pool not found" if the database pool is missing from the request extensions.
/// Any service-layer error from `address_book_service::update` is propagated as the `Err` variant.
///
/// # Examples
///
/// ```no_run
/// use actix_web::{HttpRequest, web};
///
/// // Assume `PersonDTO` can be constructed like this in your codebase.
/// let id = web::Path::from(1);
/// let updated = web::Json(PersonDTO { /* fields */ });
/// let req = HttpRequest::default();
///
/// // Call from an async context
/// let result = update(id, updated, req).await;
/// ```
pub async fn update(
    id: web::Path<i32>,
    updated_person: web::Json<PersonDTO>,
    req: HttpRequest,
) -> Result<HttpResponse, ServiceError> {
    let pool = extract_pool(&req)?;
    address_book_service::update(id.into_inner(), updated_person.into_inner(), &pool)
        .log_error("address_book_controller::update")
        .map(|_| respond_empty(&req, StatusCode::OK, constants::MESSAGE_OK))
}

// DELETE api/address-book/{id}
/// Deletes the person with the given ID from the address book.
///
/// On success returns an HTTP 200 response with a JSON `ResponseBody` containing an OK message and an empty payload.
/// If the database pool is missing or the service fails, a `ServiceError` is returned (missing pool yields an InternalServerError with message "Pool not found").
///
/// # Examples
///
/// ```no_run
/// # use actix_web::{web, HttpRequest};
/// # use crate::api::address_book_controller::delete;
/// # async fn run() -> Result<(), ()> {
/// let req = HttpRequest::default(); // example placeholder
/// let id = web::Path::from(1);
/// let _res = delete(id, req).await;
/// # Ok(())
/// # }
/// ```
pub async fn delete(id: web::Path<i32>, req: HttpRequest) -> Result<HttpResponse, ServiceError> {
    let pool = extract_pool(&req)?;
    address_book_service::delete(id.into_inner(), &pool)
        .log_error("address_book_controller::delete")
        .map(|_| respond_empty(&req, StatusCode::OK, constants::MESSAGE_OK))
}

#[cfg(test)]
mod tests {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use actix_cors::Cors;
    use actix_web::body::to_bytes;
    use actix_web::dev::Service;
    use actix_web::http::StatusCode;
    use actix_web::{http, test, ResponseError};
    use actix_web::{web, App};
    use diesel::r2d2::ConnectionManager;
    use diesel::{r2d2, PgConnection};
    use futures::FutureExt;
    use http::header;
    use serde_json::json;
    use testcontainers::clients;
    use testcontainers::images::postgres::Postgres;
    use testcontainers::Container;

    use crate::config;
    use crate::config::db::TenantPoolManager;
    use crate::models::person::{Person, PersonDTO};
    use crate::models::user::{LoginDTO, UserDTO};
    use crate::services::{account_service, address_book_service};

    pub type Connection = PgConnection;
    pub type Pool = r2d2::Pool<ConnectionManager<Connection>>;

    fn try_run_postgres<'a>(docker: &'a clients::Cli) -> Option<Container<'a, Postgres>> {
        catch_unwind(AssertUnwindSafe(|| docker.run(Postgres::default()))).ok()
    }

    fn ensure_migrations(pool: &Pool, test_name: &str) -> bool {
        match pool.get() {
            Ok(mut conn) => match config::db::run_migration(&mut conn) {
                Ok(_) => true,
                Err(e) => {
                    eprintln!("Skipping {test_name} because migration failed: {e}");
                    false
                }
            },
            Err(e) => {
                eprintln!("Skipping {test_name} because DB pool unavailable: {e}");
                false
            }
        }
    }

    /// Signs up a test admin user and performs a login, returning the authentication token on success.
    ///
    /// # Returns
    /// `Ok(token)` containing the JWT when signup and login succeed, `Err(String)` with a debug-formatted error message otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use actix_web::web;
    /// # use crate::tests::Pool;
    /// # async fn example(pool: &Pool) {
    /// let token = signup_and_login(pool).await.unwrap();
    /// println!("{}", token);
    /// # }
    /// ```
    async fn signup_and_login(pool: &Pool) -> Result<String, String> {
        let user_dto = UserDTO {
            email: "admin@example.com".to_string(),
            username: "admin".to_string(),
            password: "TestPass123".to_string(),
            active: true,
        };

        match account_service::signup(user_dto, pool) {
            Ok(_) => {
                let login_dto = LoginDTO {
                    username_or_email: "admin".to_string(),
                    password: "TestPass123".to_string(),
                    tenant_id: "tenant1".to_string(),
                };
                match account_service::login(login_dto, pool) {
                    Ok(token_res) => Ok(token_res.access_token),
                    Err(err) => Err(format!("{:?}", err.error_response())),
                }
            }
            Err(err) => Err(format!("{:?}", err.error_response())),
        }
    }

    async fn insert_mock_data(n: i32, pool: &Pool) -> Result<(), String> {
        for x in 1..=n {
            if let Err(err) = address_book_service::insert(
                PersonDTO {
                    email: format!("user{}@example.com", x),
                    name: format!("user{}", x),
                    gender: x % 2 == 0,
                    age: x * 10,
                    address: "US".to_string(),
                    phone: format!("012345678{}", x),
                },
                pool,
            ) {
                return Err(format!("{:?}", err.error_response()));
            }
        }

        Ok(())
    }

    async fn get_people_in_db(pool: &Pool) -> Result<Vec<Person>, String> {
        match address_book_service::find_all(pool) {
            Ok(data) => Ok(data),
            Err(err) => Err(format!("{:?}", err.error_response())),
        }
    }

    #[actix_web::test]
    async fn test_mock_work() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!("Skipping test_mock_work because Docker is unavailable");
                return;
            }
        };
        let pool = config::db::init_db_pool(
            format!(
                "postgres://postgres:postgres@127.0.0.1:{}/postgres",
                postgres.get_host_port_ipv4(5432)
            )
            .as_str(),
        );
        if !ensure_migrations(&pool, "test_mock_work") {
            return;
        }

        let manager = TenantPoolManager::new(pool.clone());

        let _app = test::init_service(
            App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(manager))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(crate::config::app::config_services),
        )
        .await;

        insert_mock_data(1, &pool).await.unwrap();
        assert_eq!(get_people_in_db(&pool).await.unwrap().len(), 1);
    }

    #[actix_web::test]
    async fn test_insert_ok() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!("Skipping test_insert_ok because Docker is unavailable");
                return;
            }
        };
        let pool = config::db::init_db_pool(
            format!(
                "postgres://postgres:postgres@127.0.0.1:{}/postgres",
                postgres.get_host_port_ipv4(5432)
            )
            .as_str(),
        );
        if !ensure_migrations(&pool, "test_insert_ok") {
            return;
        }

        let manager = TenantPoolManager::new(pool.clone());
        manager
            .add_tenant_pool("tenant1".to_string(), pool.clone())
            .unwrap();

        let app = test::init_service(
            App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(manager))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(crate::config::app::config_services),
        )
        .await;

        let payload = json!({
            "name": "test",
            "gender": true,
            "age": 20_i32,
            "address": "US",
            "phone": "0123456789",
            "email": "test@example.com"
        });

        match signup_and_login(&pool).await {
            Ok(token_res) => {
                let resp = test::TestRequest::post()
                    .uri("/api/address-book")
                    .insert_header(header::ContentType::json())
                    .insert_header((header::AUTHORIZATION, format!("bearer {}", token_res)))
                    .set_payload(payload.to_string())
                    .send_request(&app)
                    .await;

                assert_eq!(resp.status(), StatusCode::CREATED);
                assert_eq!(get_people_in_db(&pool).await.unwrap().len(), 1);
            }
            Err(err) => {
                unreachable!("{}", err);
            }
        };
    }

    /// Verifies that POSTing invalid person payloads returns HTTP 400 and that no records are inserted.
    ///
    /// Sends three invalid requests (missing required email, empty body, and unrelated fields) to the
    /// address-book insertion endpoint, asserts each response is `400 Bad Request`, and asserts the
    /// database remains empty afterwards.
    ///
    /// # Examples
    ///
    /// ```
    /// // Runs the integration test; the test itself sets up a temporary DB and Actix app.
    /// #[actix_rt::test]
    /// async fn run_test_insert_failed() {
    ///     crate::api::address_book_controller::test_insert_failed().await;
    /// }
    /// ```
    #[actix_web::test]
    async fn test_insert_failed() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!("Skipping test_insert_failed because Docker is unavailable");
                return;
            }
        };
        let pool = config::db::init_db_pool(
            format!(
                "postgres://postgres:postgres@127.0.0.1:{}/postgres",
                postgres.get_host_port_ipv4(5432)
            )
            .as_str(),
        );
        if !ensure_migrations(&pool, "test_insert_failed") {
            return;
        }

        let manager = TenantPoolManager::new(pool.clone());
        manager
            .add_tenant_pool("tenant1".to_string(), pool.clone())
            .unwrap();

        let app = test::init_service(
            App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(manager))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(crate::config::app::config_services),
        )
        .await;

        let req_missing_email = json!({
            "name": "test",
            "gender": true,
            "age": 20_i32,
            "address": "US",
            "phone": "0123456789"
        });

        let req_empty = json!({});

        let req_trash_value = json!({
            "this": "is",
            "not": ["a"],
            "valid": false,
            "request": 2022
        });

        match signup_and_login(&pool).await {
            Ok(token_res) => {
                let resp_missing_email = test::TestRequest::post()
                    .uri("/api/address-book")
                    .insert_header(header::ContentType::json())
                    .insert_header((header::AUTHORIZATION, format!("bearer {}", token_res)))
                    .set_payload(req_missing_email.to_string())
                    .send_request(&app)
                    .await;

                let resp_empty = test::TestRequest::post()
                    .uri("/api/address-book")
                    .insert_header(header::ContentType::json())
                    .insert_header((header::AUTHORIZATION, format!("bearer {}", token_res)))
                    .set_payload(req_empty.to_string())
                    .send_request(&app)
                    .await;

                let resp_trash_value = test::TestRequest::post()
                    .uri("/api/address-book")
                    .insert_header(header::ContentType::json())
                    .insert_header((header::AUTHORIZATION, format!("bearer {}", token_res)))
                    .set_payload(req_trash_value.to_string())
                    .send_request(&app)
                    .await;

                assert_eq!(resp_missing_email.status(), StatusCode::BAD_REQUEST);
                assert_eq!(resp_empty.status(), StatusCode::BAD_REQUEST);
                assert_eq!(resp_trash_value.status(), StatusCode::BAD_REQUEST);
                assert_eq!(get_people_in_db(&pool).await.unwrap().len(), 0);
            }
            Err(err) => {
                unreachable!("{}", err);
            }
        };
    }

    #[actix_web::test]
    async fn test_update_ok() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!("Skipping test_update_ok because Docker is unavailable");
                return;
            }
        };
        let pool = config::db::init_db_pool(
            format!(
                "postgres://postgres:postgres@127.0.0.1:{}/postgres",
                postgres.get_host_port_ipv4(5432)
            )
            .as_str(),
        );
        config::db::run_migration(&mut pool.get().unwrap())
            .expect("DB migration failed in test setup");

        let manager = TenantPoolManager::new(pool.clone());
        manager
            .add_tenant_pool("tenant1".to_string(), pool.clone())
            .unwrap();

        let app = test::init_service(
            App::new()
                .wrap(
                    Cors::default()
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_header(header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(pool.clone()))
                .app_data(web::Data::new(manager))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(crate::config::app::config_services),
        )
        .await;

        insert_mock_data(1, &pool)
            .await
            .expect("Failed to insert mock data in test setup");

        let update_request = json!({
            "email": "email1@example.com",
            "name": "Nguyen Van Teo",
            "gender": false,
            "age": 10_i32,
            "address": "US",
            "phone": "0123456781"
        });

        match signup_and_login(&pool).await {
            Ok(token_res) => {
                let resp = test::TestRequest::put()
                    .uri("/api/address-book/1")
                    .insert_header(header::ContentType::json())
                    .insert_header((header::AUTHORIZATION, format!("bearer {}", token_res)))
                    .set_payload(update_request.to_string())
                    .send_request(&app)
                    .await;

                assert_eq!(
                    resp.status(),
                    StatusCode::OK,
                    "Err: {:?}",
                    to_bytes(resp.into_body()).await.unwrap()
                );

                let data_in_db = get_people_in_db(&pool).await.unwrap();
                assert_eq!(data_in_db.len(), 1);
                assert_eq!(data_in_db[0].name, "Nguyen Van Teo");
            }
            Err(err) => {
                unreachable!("{}", err);
            }
        };
    }
}
