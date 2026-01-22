use diesel::PgConnection;
use oracle::Connection as OracleConnection;

/// Unified database connection type supporting both PostgreSQL and Oracle
pub enum DbConnection {
    Postgres(Box<diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<PgConnection>>>),
    Oracle(OracleConnection),
}

impl DbConnection {
    pub fn is_postgres(&self) -> bool {
        matches!(self, DbConnection::Postgres(_))
    }

    pub fn is_oracle(&self) -> bool {
        matches!(self, DbConnection::Oracle(_))
    }

    /// Get PostgreSQL connection or panic (use when you know it's PG)
    pub fn as_postgres(&mut self) -> &mut PgConnection {
        match self {
            DbConnection::Postgres(conn) => conn,
            DbConnection::Oracle(_) => panic!("Expected PostgreSQL connection, got Oracle"),
        }
    }

    /// Get Oracle connection or panic (use when you know it's Oracle)
    pub fn as_oracle(&mut self) -> &mut OracleConnection {
        match self {
            DbConnection::Oracle(conn) => conn,
            DbConnection::Postgres(_) => panic!("Expected Oracle connection, got PostgreSQL"),
        }
    }

    /// Try to get PostgreSQL connection
    pub fn try_as_postgres(&mut self) -> Option<&mut PgConnection> {
        match self {
            DbConnection::Postgres(conn) => Some(conn),
            DbConnection::Oracle(_) => None,
        }
    }

    /// Try to get Oracle connection
    pub fn try_as_oracle(&mut self) -> Option<&mut OracleConnection> {
        match self {
            DbConnection::Oracle(conn) => Some(conn),
            DbConnection::Postgres(_) => None,
        }
    }

    /// Try to get immutable PostgreSQL connection reference
    pub fn try_as_postgres_ref(&self) -> Option<&PgConnection> {
        match self {
            DbConnection::Postgres(conn) => Some(conn),
            DbConnection::Oracle(_) => None,
        }
    }

    /// Try to get immutable Oracle connection reference
    pub fn try_as_oracle_ref(&self) -> Option<&OracleConnection> {
        match self {
            DbConnection::Oracle(conn) => Some(conn),
            DbConnection::Postgres(_) => None,
        }
    }
}
