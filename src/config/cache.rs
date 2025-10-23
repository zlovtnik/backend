use crate::config::functional_config::EitherConvert;
use crate::services::functional_patterns::Either;
use r2d2;
use redis;

pub type Pool = r2d2::Pool<RedisManager>;

pub struct RedisManager {
    client: redis::Client,
}

impl r2d2::ManageConnection for RedisManager {
    type Connection = redis::Connection;
    type Error = redis::RedisError;

    /// Establishes a new connection to the Redis server using functional composition.
    ///
    /// Uses Either pattern for better error handling and composition.
    ///
    /// # Examples
    ///
    /// ```
    /// let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    /// let manager = RedisManager { client };
    /// let conn = manager.connect().unwrap();
    /// // `conn` is a `redis::Connection` ready to execute commands
    /// ```
    fn connect(&self) -> Result<Self::Connection, Self::Error> {
        // Use functional composition for connection establishment
        let connection_result = Either::from_result(self.client.get_connection());

        match connection_result {
            Either::Right(conn) => Ok(conn),
            Either::Left(error) => {
                // Try once more with functional composition
                log::warn!("First connection attempt failed, retrying: {}", error);
                self.client.get_connection()
            }
        }
    }

    /// Checks whether a Redis connection is alive using functional validation.
    ///
    /// Uses Either pattern for composable validation logic.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::RedisManager;
    ///
    /// let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    /// let manager = RedisManager { client };
    /// let mut conn = manager.client.get_connection().unwrap();
    /// manager.is_valid(&mut conn).unwrap();
    /// ```
    fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        // Use functional composition for validation
        let ping_result = Either::from_result(redis::cmd("PING").exec(conn));

        match ping_result {
            Either::Right(_) => Ok(()),
            Either::Left(error) => {
                log::debug!("Redis PING validation failed: {}", error);
                Err(error)
            }
        }
    }

    /// Indicates whether a pooled Redis connection should be treated as broken and removed from the pool.
    ///
    /// Returns `true` if the connection must be discarded, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Obtain a RedisManager and a connection from its pool in real usage.
    /// let mut manager = /* RedisManager instance */ unimplemented!();
    /// let mut conn = /* redis::Connection from pool */ unimplemented!();
    /// assert!(!manager.has_broken(&mut conn));
    /// ```
    fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
        false
    }
}

/// Initializes a Redis connection pool using functional composition patterns.
///
/// Uses Either pattern for error handling and functional URL masking.
/// Applies functional composition for pool creation with proper error handling.
///
/// # Examples
///
/// ```no_run
/// let pool = init_redis_client("redis://localhost:6379");
/// ```
pub fn init_redis_client(url: &str) -> Pool {
    use log::info;
    info!("Initializing Redis client with functional patterns...");

    // Use functional URL masking
    let masked_url = mask_redis_url_functional(url);

    // Functional client creation with Either pattern
    let client_result = Either::from_result(redis::Client::open(url));
    let client = match client_result {
        Either::Right(client) => client,
        Either::Left(e) => {
            panic!("Failed to create Redis client for {}: {}", masked_url, e);
        }
    };

    // Functional pool creation with composition
    let manager = RedisManager { client };
    let pool_result = Either::from_result(r2d2::Pool::builder().build(manager));

    match pool_result {
        Either::Right(pool) => {
            info!("Redis pool created successfully for {}", masked_url);
            pool
        }
        Either::Left(e) => {
            panic!("Failed to create Redis pool for {}: {}", masked_url, e);
        }
    }
}

/// Functional URL masking using composition patterns.
///
/// Uses functional composition to mask sensitive credentials in URLs.
fn mask_redis_url_functional(input: &str) -> String {
    // Functional approach to URL masking
    let find_credentials = |url: &str| -> Option<(usize, usize)> {
        let at_pos = url.find('@')?;
        let colon_pos = url[..at_pos].rfind(':')?;
        Some((colon_pos, at_pos))
    };

    let mask_url = |(colon_pos, at_pos): (usize, usize)| -> String {
        format!("{}:<redacted>{}", &input[..colon_pos], &input[at_pos..])
    };

    // Apply functional composition
    find_credentials(input)
        .map(mask_url)
        .unwrap_or_else(|| input.to_string())
}

