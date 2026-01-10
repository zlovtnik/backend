# Hybrid Database Architecture

## Overview

The application uses a **hybrid database architecture**:

- **PostgreSQL**: Admin operations, user management, tenant metadata, NFe schema
- **Oracle**: Per-tenant application data (NFAg, address books, etc.)

## Architecture Benefits

1. **Separation of Concerns**: Control plane (admin) vs. data plane (tenant apps)
2. **Database Flexibility**: Use Oracle where mandated, PostgreSQL where it makes sense
3. **Gradual Migration**: Tenants can migrate from PG → Oracle incrementally
4. **Cost Optimization**: PostgreSQL for lightweight admin, Oracle for heavyweight tenants

## Configuration

### Enable Hybrid Mode

Add to `Cargo.toml`:

```toml
[features]
default = ["hybrid-db"]
```

Or build with:

```bash
cargo build --features hybrid-db
```

### Environment Variables

```bash
# PostgreSQL admin database (users, tenants, login_history)
DATABASE_URL=postgres://user:pass@localhost/admin_db

# Tenant database URLs stored in tenants table:
# - PostgreSQL: postgres://user:pass@localhost/tenant_db
# - Oracle: hostname:port/service_name or oracle://...
```

## Database Layout

### PostgreSQL (Admin DB)

```sql
-- Tables that stay in PostgreSQL:
- users (authentication)
- tenants (tenant metadata + db_url routing)
- login_history (audit logs)
- nfe schema (if needed for centralized reporting)
```

### Oracle (Tenant DBs)

```sql
-- Per-tenant application data:
- nfag (Nota Fiscal tables)
- address_book
- cons_sit_nfag
- ret_nfag
- evento_nfag
```

## Usage Examples

### Initialize Hybrid Manager

```rust
use rcs::config::{db::init_db_pool, hybrid_manager::HybridDatabaseManager};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // PostgreSQL admin pool
    let pg_admin_pool = init_db_pool(&std::env::var("DATABASE_URL").unwrap());
    
    // Create hybrid manager
    let hybrid_manager = HybridDatabaseManager::new(pg_admin_pool);
    
    // Store in app data
    let app_data = web::Data::new(hybrid_manager);
    
    HttpServer::new(move || {
        App::new()
            .app_data(app_data.clone())
            .configure(routes)
    })
    .bind("0.0.0.0:3000")?
    .run()
    .await
}
```

### Get Admin Connection (PostgreSQL)

```rust
pub async fn list_tenants(
    hybrid_manager: web::Data<HybridDatabaseManager>,
) -> Result<HttpResponse, ServiceError> {
    use diesel::prelude::*;
    use crate::schema::tenants::dsl::*;
    
    // Always uses PostgreSQL
    let mut conn = hybrid_manager.get_admin_pool().get()?;
    
    let all_tenants = tenants.load::<Tenant>(&mut conn)?;
    
    Ok(HttpResponse::Ok().json(all_tenants))
}
```

### Get Tenant Connection (Auto-routed)

```rust
pub async fn list_nfag(
    req: HttpRequest,
    hybrid_manager: web::Data<HybridDatabaseManager>,
) -> Result<HttpResponse, ServiceError> {
    let tenant_id = extract_tenant_id(&req)?;
    
    // Automatically routes to Oracle or PostgreSQL based on tenant config
    let mut conn = hybrid_manager.get_tenant_connection(&tenant_id)?;
    
    match conn {
        DbConnection::Oracle(oracle_conn) => {
            // Use raw Oracle SQL with explicit column list
            let stmt = oracle_conn.statement(
                "SELECT id, chave, tenant_id, versao, xml_content, status, created_at, updated_at FROM nfag WHERE tenant_id = :1"
            ).build()?;
            let rows = stmt.query(&[&tenant_id])?;
            // ... process Oracle rows
        }
        DbConnection::Postgres(pg_conn) => {
            // Use Diesel ORM with explicit imports to avoid name collisions
            use crate::schema::nfag;
            let results = nfag::table
                .filter(nfag::tenant_id.eq(&tenant_id))
                .load::<Nfag>(pg_conn)?;
            // ... process Diesel results
        }
    }
    
    Ok(HttpResponse::Ok().json(results))
}
```

### Configure Tenant Database Type

Update tenant record to set database type:

```sql
-- PostgreSQL tenant
UPDATE tenants 
SET db_url = 'postgres://user:pass@localhost/tenant_a_db'
WHERE id = 'tenant_a';

-- Oracle tenant  
UPDATE tenants
SET db_url = 'oraclehost:1521/PRODDB'
WHERE id = 'tenant_b';
```

The `HybridDatabaseManager` automatically detects the database type from the URL prefix.

## Migration Path

### Phase 1: Everything on PostgreSQL (Current)
```
Admin DB (PG)     Tenant A (PG)     Tenant B (PG)
   ↓                  ↓                 ↓
[users]           [nfag]            [nfag]
[tenants]         [address_book]    [address_book]
```

### Phase 2: Hybrid (Target)
```
Admin DB (PG)     Tenant A (PG)     Tenant B (Oracle)
   ↓                  ↓                    ↓
[users]           [nfag]              [nfag]
[tenants]         [address_book]      [address_book]
```

### Migration Steps

1. **Add hybrid-db feature**:
   ```bash
   cargo build --features hybrid-db
   ```

2. **Update tenant record**:
   ```sql
   UPDATE tenants SET db_url = 'oraclehost:1521/SERVICE' WHERE id = 'tenant_b';
   ```

3. **Migrate data**:
   ```bash
   # Export from PostgreSQL
   pg_dump -t nfag tenant_b_db > nfag_export.sql
   
   # Transform SQL (PG → Oracle dialect)
   # Convert SERIAL → SEQUENCE, LIMIT → ROWNUM, etc.
   
   # Import to Oracle
   sqlplus user/pass@oraclehost:1521/SERVICE @nfag_import.sql
   ```

4. **Test tenant connection**:
   ```bash
   curl http://tenant-b.example.com/api/nfag
   ```

5. **Verify dual operation**: Tenant A stays on PG, Tenant B now on Oracle

## Health Check

```rust
pub async fn health_check(
    hybrid_manager: web::Data<HybridDatabaseManager>,
) -> Result<HttpResponse, ServiceError> {
    let health = hybrid_manager.health_check_all();
    Ok(HttpResponse::Ok().json(health))
}
```

Response:
```json
{
  "pg_admin_healthy": true,
  "pg_tenant_health": [
    ["tenant_a", true]
  ],
  "oracle_tenant_health": [
    {
      "tenant_id": "tenant_b",
      "active_connections": 3,
      "max_connections": 20,
      "is_healthy": true,
      "last_check": "2026-01-09T10:30:00"
    }
  ]
}
```

## Oracle Client Requirements

Install Oracle Instant Client:

```bash
# macOS
brew install instantclient-basic

# Linux
wget https://download.oracle.com/otn_software/linux/instantclient/instantclient-basic-linux.x64-21.1.0.0.0.zip
unzip instantclient-basic-linux.x64-21.1.0.0.0.zip
export LD_LIBRARY_PATH=/path/to/instantclient_21_1:$LD_LIBRARY_PATH
```

## Performance Considerations

- **Connection Pooling**: 20 connections per tenant pool
- **Admin Queries**: Fast (PostgreSQL, indexed tenants table)
- **Tenant Queries**: Performance depends on tenant's database
- **Caching**: Tenant → DB type mapping cached in memory

## Troubleshooting

### "Oracle pool not found"
- Check tenant.db_url format in PostgreSQL
- Ensure Oracle connection string is correct

### "Failed to get PG connection"
- Verify PostgreSQL tenant pool was created
- Check tenant database exists

### Compilation errors with `oracle` crate
- Install Oracle Instant Client
- Build with: `cargo build --features hybrid-db`
