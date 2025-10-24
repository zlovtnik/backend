//! Unified cursor-based pagination system
//!
//! This module provides a unified interface for cursor-based pagination that works
//! across different data sources (database, in-memory, streaming) and cursor types.

use std::fmt;
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use rand::RngCore;
use once_cell::sync::Lazy;
use base64::Engine;

static CURSOR_KEY: Lazy<Result<Key<Aes256Gcm>, CursorError>> = Lazy::new(|| {
    match std::env::var("CURSOR_ENCRYPTION_KEY") {
        Ok(key_b64) => {
            let key_bytes = base64::engine::general_purpose::STANDARD.decode(&key_b64)
                .map_err(|e| CursorError::KeyLoad(format!("Base64 decode failed: {}", e)))?;
            if key_bytes.len() != 32 {
                return Err(CursorError::KeyLoad("Key must be 32 bytes".to_string()));
            }
            Ok(*Key::<Aes256Gcm>::from_slice(&key_bytes))
        }
        Err(_) => Err(CursorError::KeyLoad("CURSOR_ENCRYPTION_KEY not set".to_string())),
    }
});

/// Represents a cursor that can be serialized and deserialized
pub trait Cursor: Clone + fmt::Debug + Send + Sync {
    /// Encode the cursor into a string representation
    fn encode(&self) -> String;

    /// Decode a cursor from its string representation
    fn decode(encoded: &str) -> Result<Self, CursorError>;

    /// Get the next cursor value for forward pagination
    fn next(&self) -> Option<Self>;

    /// Get the previous cursor value for backward pagination
    fn previous(&self) -> Option<Self>;

    /// Check if this cursor represents the start of the dataset
    ///
    /// For IdCursor, this compares the ID value with the configured start value.
    /// For PageCursor, this checks if the page number is 0.
    fn is_start(&self) -> bool;
}

/// Errors that can occur during cursor operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum CursorError {
    #[error("Invalid cursor format: {0}")]
    InvalidFormat(String),

    #[error("Cursor value out of range: {0}")]
    OutOfRange(String),

    #[error("Cursor type mismatch")]
    TypeMismatch,

    #[error("Base64 decoding failed: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    #[error("Integer conversion failed")]
    TryFromIntError(#[from] std::num::TryFromIntError),

    #[error("Encryption failed: {0}")]
    Encryption(String),

    #[error("Decryption failed: {0}")]
    Decryption(String),

    #[error("Key loading failed: {0}")]
    KeyLoad(String),
}

/// Utility functions for secure cursor encoding/decoding using AES-GCM
pub mod cursor_encoding {
    use super::*;
    use base64::{engine::general_purpose, Engine as _};

    /// Encode data as an authenticated encrypted cursor string
    pub fn encode_opaque(data: &str) -> Result<String, CursorError> {
        let key = match CURSOR_KEY.as_ref() {
            Ok(k) => k,
            Err(e) => return Err((*e).clone()),
        };
        let cipher = Aes256Gcm::new(key);

        // Generate random nonce
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt the data
        let ciphertext = cipher.encrypt(nonce, data.as_bytes())
            .map_err(|e| CursorError::Encryption(format!("Encryption failed: {:?}", e)))?;

        // Combine nonce + ciphertext (which includes tag)
        let mut combined = nonce_bytes.to_vec();
        combined.extend_from_slice(&ciphertext);

        // Base64 encode
        Ok(general_purpose::URL_SAFE_NO_PAD.encode(&combined))
    }

    /// Decode an authenticated encrypted cursor string back to data
    pub fn decode_opaque(encoded: &str) -> Result<String, CursorError> {
        let key = match CURSOR_KEY.as_ref() {
            Ok(k) => k,
            Err(e) => return Err((*e).clone()),
        };
        let cipher = Aes256Gcm::new(key);

        // Base64 decode
        let combined = general_purpose::URL_SAFE_NO_PAD.decode(encoded)
            .map_err(CursorError::Base64Decode)?;

        if combined.len() < 12 + 16 { // nonce + min tag
            return Err(CursorError::InvalidFormat("Cursor too short".to_string()));
        }

        // Split into nonce and ciphertext
        let nonce_bytes = &combined[..12];
        let ciphertext = &combined[12..];

        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext = cipher.decrypt(nonce, ciphertext)
            .map_err(|_| CursorError::Decryption("Authentication or decryption failed".to_string()))?;

        String::from_utf8(plaintext)
            .map_err(|_| CursorError::InvalidFormat("Invalid UTF-8 in decrypted data".to_string()))
    }

    /// Validate that a cursor string is properly formatted (basic check)
    pub fn validate_cursor_format(cursor: &str) -> bool {
        // Decode and check if it decrypts successfully
        decode_opaque(cursor).is_ok()
    }
}

/// Cursor validation utilities
pub mod cursor_validation {
    use super::*;

    /// Validates a cursor and returns detailed validation result
    pub fn validate_cursor<C: Cursor>(encoded_cursor: &str) -> CursorValidationResult<C> {
        // First check basic format
        if !cursor_encoding::validate_cursor_format(encoded_cursor) {
            return CursorValidationResult::InvalidFormat;
        }

        // Try to decode the cursor
        match C::decode(encoded_cursor) {
            Ok(cursor) => {
                // Check for additional validation rules
                // For now, just basic validation
                CursorValidationResult::Valid(cursor)
            }
            Err(_) => CursorValidationResult::InvalidFormat,
        }
    }

    /// Result of cursor validation
    pub enum CursorValidationResult<C: Cursor> {
        Valid(C),
        InvalidFormat,
    }

    impl<C: Cursor> CursorValidationResult<C> {
        pub fn is_valid(&self) -> bool {
            matches!(self, CursorValidationResult::Valid(_))
        }

        pub fn into_cursor(self) -> Option<C> {
            match self {
                CursorValidationResult::Valid(cursor) => Some(cursor),
                _ => None,
            }
        }
    }
}

/// ID-based cursor for database queries (uses primary key)
///
/// This cursor uses an ID value to represent a position in a dataset. The start of the dataset
/// is configurable through the `start_value` field, which defaults to 0 but can be set to
/// any value that makes sense for the specific dataset.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IdCursor<T: Into<i64> + TryFrom<i64> + Clone + fmt::Debug + Send + Sync> {
    id: T,
    /// The value that represents the start of the dataset for this cursor type
    start_value: i64,
}

impl<T: Into<i64> + TryFrom<i64> + Clone + fmt::Debug + Send + Sync> IdCursor<T> {
    pub fn new(id: T) -> Self {
        Self { id, start_value: 0 }
    }

    /// Create a new IdCursor with a custom start value.
    ///
    /// This allows you to specify what ID value represents the start of your dataset.
    /// For example, if your dataset starts with ID 1 instead of 0, you would use:
    /// `IdCursor::with_start_value(5i32, 1)`
    pub fn with_start_value(id: T, start_value: i64) -> Self {
        Self { id, start_value }
    }

    pub fn id(&self) -> T {
        self.id.clone()
    }

    pub fn start_value(&self) -> i64 {
        self.start_value
    }
}

impl<T: Into<i64> + TryFrom<i64> + Clone + fmt::Debug + fmt::Display + Send + Sync> Cursor
    for IdCursor<T>
{
    fn encode(&self) -> String {
        let data = format!("id:{}", self.id().into());
        cursor_encoding::encode_opaque(&data).unwrap()
    }

    fn decode(encoded: &str) -> Result<Self, CursorError> {
        let decoded = cursor_encoding::decode_opaque(encoded)?;

        let parts: Vec<&str> = decoded.split(':').collect();
        if parts.len() != 2 || parts[0] != "id" {
            return Err(CursorError::InvalidFormat(decoded));
        }

        let id: i64 = parts[1]
            .parse()
            .map_err(|_| CursorError::InvalidFormat(decoded.clone()))?;
        let id_converted = T::try_from(id).map_err(|_| CursorError::OutOfRange(decoded))?;
        Ok(Self::with_start_value(id_converted, 0)) // Default start value is 0
    }

    fn next(&self) -> Option<Self> {
        // For ID cursors, we can't predict the next ID
        None
    }

    fn previous(&self) -> Option<Self> {
        // For ID cursors, we can't predict the previous ID
        None
    }

    fn is_start(&self) -> bool {
        // Check if the ID matches the configured start value
        self.id().into() == self.start_value
    }
}

/// Page-based cursor for offset-style pagination
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageCursor {
    page: usize,
}

impl PageCursor {
    pub fn new(page: usize) -> Self {
        Self { page }
    }

    pub fn page(&self) -> usize {
        self.page
    }
}

impl Cursor for PageCursor {
    fn encode(&self) -> String {
        let data = format!("page:{}", self.page());
        cursor_encoding::encode_opaque(&data).unwrap()
    }

    fn decode(encoded: &str) -> Result<Self, CursorError> {
        let decoded = cursor_encoding::decode_opaque(encoded)?;

        let parts: Vec<&str> = decoded.split(':').collect();
        if parts.len() != 2 || parts[0] != "page" {
            return Err(CursorError::InvalidFormat(decoded));
        }

        let page: usize = parts[1]
            .parse()
            .map_err(|_| CursorError::InvalidFormat(decoded))?;
        Ok(Self::new(page))
    }

    fn next(&self) -> Option<Self> {
        self.page().checked_add(1).map(Self::new)
    }

    fn previous(&self) -> Option<Self> {
        if self.page() > 0 {
            Some(Self::new(self.page() - 1))
        } else {
            None
        }
    }

    fn is_start(&self) -> bool {
        self.page() == 0
    }
}

/// Unified pagination request parameters
#[derive(Debug, Clone)]
pub struct PaginationRequest<C: Cursor> {
    pub cursor: Option<C>,
    pub page_size: usize,
    pub direction: PaginationDirection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaginationDirection {
    Forward,
    Backward,
}

impl<C: Cursor> PaginationRequest<C> {
    pub fn new(cursor: Option<C>, page_size: usize) -> Self {
        Self {
            cursor,
            page_size,
            direction: PaginationDirection::Forward,
        }
    }

    pub fn forward(cursor: Option<C>, page_size: usize) -> Self {
        Self {
            cursor,
            page_size,
            direction: PaginationDirection::Forward,
        }
    }

    pub fn backward(cursor: Option<C>, page_size: usize) -> Self {
        Self {
            cursor,
            page_size,
            direction: PaginationDirection::Backward,
        }
    }
}

/// Unified pagination response
#[derive(Debug, Clone)]
pub struct PaginationResponse<T, C: Cursor> {
    pub items: Vec<T>,
    pub next_cursor: Option<C>,
    pub previous_cursor: Option<C>,
    pub has_more: bool,
    pub total_count: Option<usize>,
}

impl<T, C: Cursor> PaginationResponse<T, C> {
    pub fn new(
        items: Vec<T>,
        next_cursor: Option<C>,
        previous_cursor: Option<C>,
        has_more: bool,
        total_count: Option<usize>,
    ) -> Self {
        Self {
            items,
            next_cursor,
            previous_cursor,
            has_more,
            total_count,
        }
    }
}

/// Trait for data sources that support cursor-based pagination
#[async_trait::async_trait]
pub trait CursorPaginated<T, C: Cursor> {
    async fn paginate(
        &self,
        request: PaginationRequest<C>,
    ) -> Result<PaginationResponse<T, C>, PaginationError>;
}

/// Errors that can occur during pagination operations
#[derive(Debug, thiserror::Error)]
pub enum PaginationError {
    #[error("Cursor error: {0}")]
    Cursor(#[from] CursorError),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Invalid page size: {0}")]
    InvalidPageSize(usize),

    #[error("Pagination not supported")]
    NotSupported,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init_test_key() {
        INIT.call_once(|| {
            if env::var("CURSOR_ENCRYPTION_KEY").is_err() {
                env::set_var("CURSOR_ENCRYPTION_KEY", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=");
            }
        });
    }

    #[test]
    fn test_id_cursor_encoding() {
        init_test_key();
        let cursor = IdCursor::new(42i32);
        let encoded = cursor.encode();

        // Encoded cursor should be opaque (not contain readable "42")
        assert!(!encoded.contains("42"));
        assert!(!encoded.contains("id:"));

        let decoded = IdCursor::<i32>::decode(&encoded).unwrap();
        assert_eq!(decoded.id(), 42);
    }

    #[test]
    fn test_page_cursor_encoding() {
        init_test_key();
        let cursor = PageCursor::new(10);
        let encoded = cursor.encode();

        // Encoded cursor should be opaque
        assert!(!encoded.contains("10"));
        assert!(!encoded.contains("page:"));

        let decoded = PageCursor::decode(&encoded).unwrap();
        assert_eq!(decoded.page(), 10);
    }

    #[test]
    fn test_cursor_navigation() {
        let page_cursor = PageCursor::new(5);
        assert_eq!(page_cursor.next().unwrap().page(), 6);
        assert_eq!(page_cursor.previous().unwrap().page(), 4);
        assert!(!page_cursor.is_start());

        let start_cursor = PageCursor::new(0);
        assert!(start_cursor.is_start());
        assert!(start_cursor.previous().is_none());
    }

    #[test]
    fn test_invalid_cursor_decoding() {
        init_test_key();
        // Test invalid base64
        assert!(IdCursor::<i32>::decode("invalid-base64!").is_err());

        // Test invalid format after decoding
        assert!(IdCursor::<i32>::decode("aW52YWxpZA").is_err()); // "invalid" encoded

        // Test wrong cursor type
        let page_encoded = PageCursor::new(1).encode();
        assert!(IdCursor::<i32>::decode(&page_encoded).is_err());
    }

    #[test]
    fn test_cursor_validation() {
        init_test_key();
        let cursor = PageCursor::new(5);
        let encoded = cursor.encode();

        // Test valid cursor
        let validation_result = cursor_validation::validate_cursor::<PageCursor>(&encoded);
        assert!(validation_result.is_valid());
        assert_eq!(validation_result.into_cursor().unwrap().page(), 5);

        // Test invalid format
        let invalid_result = cursor_validation::validate_cursor::<PageCursor>("invalid-cursor");
        assert!(!invalid_result.is_valid());

        // Test wrong cursor type
        let page_encoded = PageCursor::new(1).encode();
        let id_validation = cursor_validation::validate_cursor::<IdCursor<i64>>(&page_encoded);
        assert!(!id_validation.is_valid());
    }

    #[test]
    fn test_cursor_format_validation() {
        init_test_key();
        let valid_cursor = PageCursor::new(1).encode();
        assert!(cursor_encoding::validate_cursor_format(&valid_cursor));

        assert!(!cursor_encoding::validate_cursor_format("invalid"));
        assert!(!cursor_encoding::validate_cursor_format(""));
    }
}
