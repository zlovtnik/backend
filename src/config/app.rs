use crate::api::*;
use crate::config::functional_config::RouteBuilder;
use actix_web::web;

/// Macro to generate CRUD route configuration functions.
///
/// This macro eliminates boilerplate by generating the standard pattern for CRUD endpoints.
/// It creates two route resources: one for the collection (list/create) and one for individual items (get/update/delete).
///
/// The generated function follows this pattern:
/// - GET `/` → list handler
/// - POST `/` → create handler
/// - GET `/{id}` → get by id handler
/// - PUT `/{id}` → update handler
/// - DELETE `/{id}` → delete handler
///
/// # Example
///
/// ```ignore
/// configure_crud_routes!(
///     configure_nfag_routes,
///     nfag_controller,
///     find_all,
///     create,
///     find_by_id,
///     update,
///     delete
/// );
/// ```
macro_rules! configure_crud_routes {
    (
        $fn_name:ident,
        $controller:ident,
        $list:ident,
        $create:ident,
        $get:ident,
        $update:ident,
        $delete:ident,
        $scope:expr,
        $doc_prefix:expr
    ) => {
        #[doc = concat!(
            "Register ", $doc_prefix, " HTTP routes using functional composition patterns.\n\n",
            "Uses RouteBuilder to compose endpoints functionally:\n",
            "- GET `/` → `", stringify!($controller), "::", stringify!($list), "`\n",
            "- POST `/` → `", stringify!($controller), "::", stringify!($create), "`\n",
            "- GET `/{id}` → `", stringify!($controller), "::", stringify!($get), "`\n",
            "- PUT `/{id}` → `", stringify!($controller), "::", stringify!($update), "`\n",
            "- DELETE `/{id}` → `", stringify!($controller), "::", stringify!($delete), "`\n\n",
            "# Examples\n\n",
            "```\n",
            "use actix_web::web;\n\n",
            "// Mount the ", $doc_prefix, " routes under `/", $scope, "`\n",
            "let scope = web::scope(\"/", $scope, "\").configure(", stringify!($fn_name), ");\n",
            "```"
        )]
        fn $fn_name(cfg: &mut web::ServiceConfig) {
            RouteBuilder::new()
                .add_route(|cfg| {
                    cfg.service(
                        web::resource("")
                            .route(web::get().to($controller::$list))
                            .route(web::post().to($controller::$create)),
                    );
                })
                .add_route(|cfg| {
                    cfg.service(
                        web::resource("/{id}")
                            .route(web::get().to($controller::$get))
                            .route(web::put().to($controller::$update))
                            .route(web::delete().to($controller::$delete)),
                    );
                })
                .build(cfg);
        }
    };
}

/// Configure application HTTP routes using functional composition patterns.
///
/// This function uses the RouteBuilder pattern to compose route configurations
/// in a functional, composable manner. It uses functional composition for
/// logging and error handling.
///
/// # Examples
///
/// ```
/// use actix_web::App;
///
/// // Mounts the routes onto an Actix application builder using functional composition.
/// let app = App::new().configure(config_services);
/// ```
pub fn config_services(cfg: &mut web::ServiceConfig) {
    // Build routes using functional composition
    let route_builder: RouteBuilder = RouteBuilder::new()
        .add_route(|cfg| {
            cfg.service(health_controller::health);
        })
        .add_route(|cfg| {
            cfg.service(web::resource("/api-doc").route(web::get().to(openapi::api_doc_redirect)));
            cfg.service(
                web::resource("/api-doc/openapi.json").route(web::get().to(openapi::openapi_json)),
            );
            cfg.service(utoipa_swagger_ui::SwaggerUi::new("/swagger-ui/{_:.*}").url(
                "/api-doc/openapi.json",
                <openapi::ApiDoc as utoipa::OpenApi>::openapi(),
            ));
        })
        .add_route(|cfg| {
            cfg.service(web::scope("/api").configure(configure_api_routes));
        });

    // Build routes directly
    route_builder.build(cfg);
}

/// Register API endpoints and nested scopes under `/api` using functional composition.
///
/// Uses the RouteBuilder pattern to compose routes functionally, making the configuration
/// more composable and testable. Each scope is added as a separate route transformation.
///
/// # Examples
///
/// ```
/// use actix_web::{App, web};
///
/// let app = App::new().service(web::scope("/api").configure(configure_api_routes));
/// ```
fn configure_api_routes(cfg: &mut web::ServiceConfig) {
    RouteBuilder::new()
        // Standalone routes in /api
        .add_route(|cfg| {
            cfg.service(ping_controller::ping);
        })
        .add_route(|cfg| {
            cfg.service(health_controller::health_detailed);
        })
        .add_route(|cfg| {
            cfg.service(health_controller::performance_metrics);
        })
        .add_route(|cfg| {
            cfg.service(health_controller::logs);
        })
        .add_route(|cfg| {
            cfg.service(web::resource("/ws/logs").route(web::get().to(ws_controller::ws_logs)));
        })
        // Scoped routes
        .add_route(|cfg| {
            cfg.service(web::scope("/auth").configure(configure_auth_routes));
        })
        .add_route(|cfg| {
            cfg.service(web::scope("/address-book").configure(configure_address_book_routes));
        })
        .add_route(|cfg| {
            cfg.service(web::scope("/admin").configure(configure_admin_routes));
        })
        .add_route(|cfg| {
            cfg.service(web::scope("/users").configure(configure_user_routes));
        })
        .add_route(|cfg| {
            cfg.service(web::scope("/nfag").configure(configure_nfag_routes));
        })
        .add_route(|cfg| {
            cfg.service(web::scope("/cons-sit-nfag").configure(configure_cons_sit_nfag_routes));
        })
        .add_route(|cfg| {
            cfg.service(
                web::scope("/cons-stat-serv-nfag").configure(configure_cons_stat_serv_nfag_routes),
            );
        })
        .add_route(|cfg| {
            cfg.service(web::scope("/evento-nfag").configure(configure_evento_nfag_routes));
        })
        .build(cfg);
}

/// Register authentication endpoints using functional composition patterns.
///
/// Uses functional composition to build authentication routes in a composable manner.
///
/// # Examples
///
/// ```
/// use actix_web::{App, web};
///
/// let app = App::new().service(web::scope("/api/auth").configure(configure_auth_routes));
/// ```
fn configure_auth_routes(cfg: &mut web::ServiceConfig) {
    RouteBuilder::new()
        .add_route(|cfg| {
            cfg.service(web::resource("/signup").route(web::post().to(account_controller::signup)));
        })
        .add_route(|cfg| {
            cfg.service(web::resource("/login").route(web::post().to(account_controller::login)));
        })
        .add_route(|cfg| {
            cfg.service(web::resource("/login/keycloak").route(web::get().to(account_controller::keycloak_login)));
        })
        .add_route(|cfg| {
            cfg.service(web::resource("/callback").route(web::get().to(account_controller::keycloak_callback)));
        })
        .add_route(|cfg| {
            cfg.service(web::resource("/logout").route(web::post().to(account_controller::logout)));
        })
        .add_route(|cfg| {
            cfg.service(
                web::resource("/refresh").route(web::post().to(account_controller::refresh)),
            );
        })
        .add_route(|cfg| {
            cfg.service(
                web::resource("/refresh-token")
                    .route(web::post().to(account_controller::refresh_token)),
            );
        })
        .add_route(|cfg| {
            cfg.service(web::resource("/me").route(web::get().to(account_controller::me)));
        })
        .build(cfg);
}

/// Register address-book HTTP routes using functional composition patterns.
///
/// Uses RouteBuilder to compose endpoints functionally:
/// - GET `/` → `address_book_controller::find_all`
/// - POST `/` → `address_book_controller::insert`
/// - GET `/{id}` → `address_book_controller::find_by_id`
/// - PUT `/{id}` → `address_book_controller::update`
/// - DELETE `/{id}` → `address_book_controller::delete`
/// - GET `/filter` → `address_book_controller::filter`
///
/// # Examples
///
/// ```
/// use actix_web::web;
///
/// // Mount the address-book routes under `/address-book`
/// let scope = web::scope("/address-book").configure(configure_address_book_routes);
/// ```
fn configure_address_book_routes(cfg: &mut web::ServiceConfig) {
    RouteBuilder::new()
        .add_route(|cfg| {
            cfg.service(
                web::resource("")
                    .route(web::get().to(address_book_controller::find_all))
                    .route(web::post().to(address_book_controller::insert)),
            );
        })
        .add_route(|cfg| {
            cfg.service(
                web::resource("/filter").route(web::get().to(address_book_controller::filter)),
            );
        })
        .add_route(|cfg| {
            cfg.service(
                web::resource("/{id}")
                    .route(web::get().to(address_book_controller::find_by_id))
                    .route(web::put().to(address_book_controller::update))
                    .route(web::delete().to(address_book_controller::delete)),
            );
        })
        .build(cfg);
}

/// Registers the admin sub-scope using functional composition patterns.
///
/// Uses RouteBuilder to functionally mount tenant administration endpoints under two distinct scopes:
/// - `/tenant` - System-level monitoring and health checks (stats, health, status)
/// - `/tenants` - RESTful CRUD operations for tenant resource management
///
/// # Route Structure
///
/// ```text
/// /api/admin
///   ├── /tenant          (System operations - read-only monitoring)
///   │   ├── /stats       GET: System-wide tenant statistics
///   │   ├── /health      GET: All tenant database health checks
///   │   └── /status      GET: Tenant connection status map
///   └── /tenants         (Resource CRUD - tenant lifecycle management)
///       ├── /            GET: List all tenants (paginated)
///       ├── /            POST: Create new tenant
///       ├── /filter      GET: Filter tenants by criteria
///       └── /{id}        GET/PUT/DELETE: Individual tenant operations
/// ```
///
/// # Examples
///
/// ```
/// use actix_web::{App, test, web};
/// // Mount the admin routes onto an Actix-web App
/// let app = test::init_service(App::new().configure(|cfg: &mut web::ServiceConfig| {
///     configure_admin_routes(cfg);
/// }));
/// ```
fn configure_admin_routes(cfg: &mut web::ServiceConfig) {
    RouteBuilder::new()
        .add_route(|cfg| {
            // System-level monitoring endpoints: stats, health, status (read-only)
            cfg.service(web::scope("/tenant").configure(configure_tenant_admin_routes));
        })
        .add_route(|cfg| {
            // RESTful CRUD endpoints: create, read, update, delete tenant resources
            cfg.service(web::scope("/tenants").configure(configure_tenant_crud_routes));
        })
        .build(cfg);
}

/// Register tenant system-level monitoring endpoints using functional composition.
///
/// These are **read-only system operations** for monitoring and health checks across all tenants.
/// They provide aggregate statistics and health status but do not modify tenant data.
///
/// The configured routes (relative to `/admin/tenant`) are:
/// - GET `/stats` -> `tenant_controller::get_system_stats` - System-wide tenant statistics
/// - GET `/health` -> `tenant_controller::get_tenant_health` - Database health for all tenants
/// - GET `/status` -> `tenant_controller::get_tenant_status` - Connection status map by tenant ID
///
/// # Distinction from CRUD Routes
///
/// This scope (`/admin/tenant`) is for **monitoring**, while `/admin/tenants` handles **CRUD operations**.
///
/// # Examples
///
/// ```
/// use actix_web::{web, App};
///
/// let _app = App::new().service(web::scope("/admin/tenant").configure(configure_tenant_admin_routes));
/// ```
fn configure_tenant_admin_routes(cfg: &mut web::ServiceConfig) {
    RouteBuilder::new()
        .add_route(|cfg| {
            cfg.service(
                web::resource("/stats").route(web::get().to(tenant_controller::get_system_stats)),
            );
        })
        .add_route(|cfg| {
            cfg.service(
                web::resource("/health").route(web::get().to(tenant_controller::get_tenant_health)),
            );
        })
        .add_route(|cfg| {
            cfg.service(
                web::resource("/status").route(web::get().to(tenant_controller::get_tenant_status)),
            );
        })
        .build(cfg);
}

/// Register tenant resource CRUD endpoints using functional composition.
///
/// These are **RESTful CRUD operations** for tenant lifecycle management.
/// They enable creating, reading, updating, and deleting tenant resources.
///
/// The configured routes (relative to `/admin/tenants`) are:
/// - GET `/` -> `tenant_controller::find_all` - List all tenants with pagination
/// - GET `/filter` -> `tenant_controller::filter` - Filter tenants by custom criteria
/// - POST `/` -> `tenant_controller::create` - Create a new tenant
/// - GET `/{id}` -> `tenant_controller::find_by_id` - Get specific tenant by ID
/// - PUT `/{id}` -> `tenant_controller::update` - Update existing tenant
/// - DELETE `/{id}` -> `tenant_controller::delete` - Delete tenant
///
/// # Distinction from System Monitoring Routes
///
/// This scope (`/admin/tenants`) is for **CRUD operations**, while `/admin/tenant` handles **monitoring**.
///
/// # Examples
///
/// ```
/// use actix_web::{web, App};
///
/// let _app = App::new().service(web::scope("/admin/tenants").configure(configure_tenant_crud_routes));
/// ```
fn configure_tenant_crud_routes(cfg: &mut web::ServiceConfig) {
    RouteBuilder::new()
        .add_route(|cfg| {
            cfg.service(
                web::resource("")
                    .route(web::get().to(tenant_controller::find_all))
                    .route(web::post().to(tenant_controller::create)),
            );
        })
        .add_route(|cfg| {
            cfg.service(web::resource("/filter").route(web::get().to(tenant_controller::filter)));
        })
        .add_route(|cfg| {
            cfg.service(
                web::resource("/{id}")
                    .route(web::get().to(tenant_controller::find_by_id))
                    .route(web::put().to(tenant_controller::update))
                    .route(web::delete().to(tenant_controller::delete)),
            );
        })
        .build(cfg);
}

/// Registers user management HTTP routes using functional composition patterns.
///
/// Uses RouteBuilder to configure collection routes (GET list, POST create),
/// and per-user operations by `{id}` (GET, PUT, DELETE).
///
/// # Examples
///
/// ```
/// use actix_web::web;
///
/// // Attach the user routes under the `/users` scope.
/// let _scope = web::scope("/users").configure(configure_user_routes);
/// ```
fn configure_user_routes(cfg: &mut web::ServiceConfig) {
    RouteBuilder::new()
        .add_route(|cfg| {
            cfg.service(web::resource("").route(web::get().to(user_controller::find_all)));
        })
        .add_route(|cfg| {
            cfg.service(
                web::resource("/{id}")
                    .route(web::get().to(user_controller::find_by_id))
                    .route(web::put().to(user_controller::update))
                    .route(web::delete().to(user_controller::delete)),
            );
        })
        .build(cfg);
}

// Generate CRUD route configuration functions using the macro
configure_crud_routes!(
    configure_nfag_routes,
    nfag_controller,
    find_all,
    create,
    find_by_id,
    update,
    delete,
    "nfag",
    "NFAg"
);

configure_crud_routes!(
    configure_cons_sit_nfag_routes,
    cons_sit_nfag_controller,
    find_all,
    create,
    find_by_id,
    update,
    delete,
    "cons-sit-nfag",
    "ConsSitNfag"
);

configure_crud_routes!(
    configure_cons_stat_serv_nfag_routes,
    cons_stat_serv_nfag_controller,
    find_all,
    create,
    find_by_id,
    update,
    delete,
    "cons-stat-serv-nfag",
    "ConsStatServNfag"
);

configure_crud_routes!(
    configure_evento_nfag_routes,
    evento_nfag_controller,
    find_all_eventos_nfag,
    create_evento_nfag,
    find_evento_nfag_by_id,
    update_evento_nfag,
    delete_evento_nfag,
    "evento-nfag",
    "EventoNfag"
);
