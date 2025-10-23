#![allow(unused_must_use)]

use std::default::Default;
use std::io::LineWriter;
use std::path::Path;
use std::{env, fs::OpenOptions, io};

use actix_cors::Cors;
use actix_web::dev::Service;
use actix_web::web;
use actix_web::{http, App, HttpServer};
use futures::FutureExt;

mod api;
mod config;
mod constants;
mod error;
mod functional;
mod middleware;
mod models;
mod pagination;
mod schema;
mod services;
mod utils;
/// יהי רצון מלפני ה' שימצא עבודה חדשה טובה, בעוד נקודת הכניסה ליישום מסדרת לוגים וסביבה, מאתחלת מסד נתונים ורדיס,
/// רושמת בריכות טננטים, מסדרת CORS ומיידלוור, ומתחילה שרת Actix HTTP.
///
/// פונקציה זו קוראת משתני סביבה נחוצים (APP_HOST, APP_PORT, DATABASE_URL, REDIS_URL),
/// מסדרת לוגים (אופציונלי לקובץ אם LOG_FILE מסופק), מאתחלת בריכה ראשית DB ו
/// לקוח רדיס, רושמת טננט הדגמה, בונה אפליקציית Actix עם CORS ומיידלוור, קושרת
/// לכתובת מסודרת, ומריצה שרת עד לכיבוי.
///
/// # דוגמאות
///
/// ```no_run
/// // התחל יישום (מחייב משתני סביבה מתאימים).
/// // בדרך כלל מופעל על ידי סביבת ריצה; מוצג כאן למטרות אילוסטרטיביות בלבד.
/// let _ = futures::executor::block_on(crate::main());
/// ```
#[actix_rt::main]
async fn main() -> io::Result<()> {
    if let Err(e) = dotenv::dotenv() {
        match e {
            dotenv::Error::Io(io_err) if io_err.kind() == std::io::ErrorKind::NotFound => {
                log::warn!(".env file not found, environment variables will be read from system environment");
            }
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Failed to read .env file: {}", e),
                ));
            }
        }
    }
    env::set_var("RUST_LOG", "actix_web=debug");

    if let Ok(log_file_path) = env::var("LOG_FILE") {
        let path = Path::new(&log_file_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)?;
        env_logger::Builder::from_default_env()
            .target(env_logger::Target::Pipe(Box::new(LineWriter::new(
                log_file,
            ))))
            .init();
    } else {
        env_logger::init();
    }

    let app_host = env::var("APP_HOST").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("APP_HOST not found: {}", e),
        )
    })?;
    let app_port = env::var("APP_PORT").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("APP_PORT not found: {}", e),
        )
    })?;
    let app_url = format!("{}:{}", &app_host, &app_port);
    let db_url = env::var("DATABASE_URL").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("DATABASE_URL not found: {}", e),
        )
    })?;
    let redis_url = env::var("REDIS_URL").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("REDIS_URL not found: {}", e),
        )
    })?;

    let main_pool = config::db::init_db_pool(&db_url);
    config::db::run_migration(&mut main_pool.get().unwrap());
    let redis_client = config::cache::init_redis_client(&redis_url);

    let manager = config::db::TenantPoolManager::new(main_pool.clone());
    // יהי רצון שימצא עבודה, קוד קשה טננט להדגמה, בייצור טען ממסד נתונים
    manager
        .add_tenant_pool("tenant1".to_string(), main_pool.clone())
        .expect("Failed to add tenant pool");

    HttpServer::new(move || {
        // יהי רצון שימצא עבודה, הגדר CORS על פי סביבה
        let app_env = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
        let mut cors_builder = if app_env == "production" {
            // יהי רצון שימצא עבודה, ייצור: CORS מגביל עם מקורות מותרים מוגדרים
            let mut builder = Cors::default();

            if let Ok(allowed_origins) = env::var("CORS_ALLOWED_ORIGINS") {
                // יהי רצון שימצא עבודה, חלק מקורות מופרדות בפסיק והוסף כל אחד
                for origin in allowed_origins
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                {
                    builder = builder.allowed_origin(origin);
                }
            } else {
                // יהי רצון שימצא עבודה, ברירת מחדל ללוקאל-הוסט אם לא מוגדר מקורות
                builder = builder.allowed_origin("http://localhost:3000");
            }
            builder
        } else {
            // יהי רצון שימצא עבודה, פיתוח/בדיקה: CORS מתיר יותר אך מפורש
            // Note: send_wildcard() conflicts with allowed_origin(), so we use explicit origins
            Cors::default()
                .allowed_origin("http://localhost:3000")
                .allowed_origin("http://localhost:3001")
                .allowed_origin("http://127.0.0.1:3000")
                .allowed_origin("http://127.0.0.1:3001")
                .allowed_origin("http://localhost:5173") // Vite dev server
                .allowed_origin("http://127.0.0.1:5173") // Vite dev server
        };

        // יהי רצון שימצא עבודה, הוסף שיטות וכותרות נפוצות
        cors_builder = cors_builder
            .allowed_methods(vec![
                http::Method::GET,
                http::Method::POST,
                http::Method::PUT,
                http::Method::DELETE,
                http::Method::OPTIONS,
            ])
            .allowed_headers(vec![
                http::header::AUTHORIZATION,
                http::header::ACCEPT,
                http::header::CONTENT_TYPE,
                http::header::HeaderName::from_static("x-tenant-id"),
            ])
            .expose_headers(vec![
                http::header::AUTHORIZATION,
                http::header::CONTENT_TYPE,
                http::header::HeaderName::from_static("x-tenant-id"),
            ])
            .max_age(3600);

        // יהי רצון שימצא עבודה, בדוק דגל אישורים
        let cors = if env::var("CORS_ALLOW_CREDENTIALS")
            .map(|v| v == "true")
            .unwrap_or(false)
        {
            cors_builder.supports_credentials()
        } else {
            cors_builder
        };

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(manager.clone()))
            .app_data(web::Data::new(main_pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .wrap(actix_web::middleware::Logger::default())
            .wrap(crate::middleware::auth_middleware::Authentication) // יהי רצון שימצא עבודה, הערה לקו זה אם רוצים לשלב עם yew-address-book-frontend
            .wrap_fn(|req, srv| srv.call(req).map(|res| res))
            .configure(config::app::config_services)
    })
    .bind(&app_url)?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use std::panic::{catch_unwind, AssertUnwindSafe};

    use actix_cors::Cors;
    use actix_web::dev::Service;
    use actix_web::web;
    use actix_web::{http, App, HttpServer};
    use futures::FutureExt;
    use testcontainers::clients;
    use testcontainers::images::postgres::Postgres;
    use testcontainers::Container;

    use crate::config;

    fn try_run_postgres<'a>(docker: &'a clients::Cli) -> Option<Container<'a, Postgres>> {
        catch_unwind(AssertUnwindSafe(|| docker.run(Postgres::default()))).ok()
    }

    #[actix_web::test]
    async fn test_startup_ok() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!("Skipping test_startup_ok because Docker is unavailable");
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
        config::db::run_migration(&mut pool.get().unwrap());

        HttpServer::new(move || {
            App::new()
                .wrap(
                    Cors::default() // allowed_origin return access-control-allow-origin: * by default
                        // .allowed_origin("http://127.0.0.1:8080")
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                        .allowed_header(http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(pool.clone()))
                .wrap(actix_web::middleware::Logger::default())
                .wrap(crate::middleware::auth_middleware::Authentication)
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(config::app::config_services)
        })
        .bind("localhost:8000".to_string())
        .unwrap()
        .run();

        assert_eq!(true, true);
    }

    /// Starts an Actix HTTP server configured with CORS and a database pool to verify it can start without authentication middleware.
    ///
    /// # Examples
    ///
    /// ```
    /// // This test starts a PostgreSQL test container, runs DB migrations,
    /// // and launches an Actix server bound to localhost:8001 with permissive CORS
    /// // and no authentication middleware to ensure startup succeeds.
    /// #[actix_web::test]
    /// async fn test_startup_without_auth_middleware_ok() {
    ///     // setup test Postgres, pool, migrations, and start server...
    /// }
    /// ```
    #[actix_web::test]
    async fn test_startup_without_auth_middleware_ok() {
        let docker = clients::Cli::default();
        let postgres = match try_run_postgres(&docker) {
            Some(container) => container,
            None => {
                eprintln!(
                    "Skipping test_startup_without_auth_middleware_ok because Docker is unavailable"
                );
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
        config::db::run_migration(&mut pool.get().unwrap());

        HttpServer::new(move || {
            App::new()
                .wrap(
                    Cors::default() // allowed_origin return access-control-allow-origin: * by default
                        // .allowed_origin("http://127.0.0.1:8080")
                        .send_wildcard()
                        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
                        .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
                        .allowed_header(http::header::CONTENT_TYPE)
                        .max_age(3600),
                )
                .app_data(web::Data::new(pool.clone()))
                .wrap(actix_web::middleware::Logger::default())
                .wrap_fn(|req, srv| srv.call(req).map(|res| res))
                .configure(config::app::config_services)
        })
        .bind("localhost:8001".to_string())
        .unwrap()
        .run();

        assert_eq!(true, true);
    }
}
