//! Composable Response Transformers
//!
//! This module implements functional response composition utilities that
//! standardize API responses, provide content-type negotiation, and
//! integrate seamlessly with Actix Web's [`Responder`] trait. The design
//! embraces immutable data, pure transformation functions, and
//! declarative configuration to keep response formatting predictable and
//! testable across all endpoints.
//!
//! # Highlights
//!
//! - [`ResponseTransformer`] is a fluent, functional builder for HTTP
//!   responses that supports `map`-style transformations on payload,
//!   message, and metadata.
//! - Automatic content-type negotiation honours the caller's `Accept`
//!   header while allowing explicit format overrides where required.
//! - Optional metadata attachment enables enriched responses without
//!   forcing schema changes on the underlying data structures.
//! - A fallible API (`try_with_metadata`, `try_insert_header`) provides
//!   ergonomic error handling without sacrificing composability.
//!
//! ```no_run
//! use actix_web::http::StatusCode;
//! use actix_web::test::TestRequest;
//! use actix_web::Responder;
//! use crate::functional::response_transformers::ResponseTransformer;
//!
//! # actix_rt::System::new().block_on(async {
//! let response = ResponseTransformer::new(vec![1, 2, 3])
//!     .with_message("numbers")
//!     .with_status(StatusCode::CREATED)
//!     .map_data(|numbers| numbers.into_iter().map(|n| n * 2).collect::<Vec<_>>())
//!     .respond_to(&TestRequest::default().to_http_request());
//! assert_eq!(response.status(), StatusCode::CREATED);
//! # });
//! ```

use std::borrow::Cow;
use std::str::FromStr;

#[cfg_attr(not(test), allow(unused_imports))]
use actix_web::body::BoxBody;
use actix_web::http::header::{
    self, HeaderName, HeaderValue, InvalidHeaderName, InvalidHeaderValue,
};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, HttpResponseBuilder, Responder};
use serde::{Deserialize, Serialize};
use serde_json::{self, json, Value as JsonValue};
use thiserror::Error;

use crate::constants;
use crate::models::response::ResponseBody;

/// Supported output formats handled by the response transformer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResponseFormat {
    /// Standard JSON (`application/json`).
    Json,
    /// Pretty-printed JSON (`application/json`).
    JsonPretty,
    /// Plain text (`text/plain`).
    Text,
    /// XML format (`application/xml`).
    Xml,
    /// CSV format (`text/csv`).
    Csv,
    /// MessagePack binary format (`application/msgpack`).
    MessagePack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FormatStrategy {
    Auto,
    Forced(ResponseFormat),
}

/// Errors that can occur while composing responses.
#[derive(Debug, Error)]
pub enum ResponseTransformError {
    #[error("metadata serialization failed: {0}")]
    MetadataSerialization(#[from] serde_json::Error),
    #[error("invalid header name: {0}")]
    InvalidHeaderName(#[from] InvalidHeaderName),
    #[error("invalid header value: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
}

/// Serializable response envelope used for standardized payloads.
#[derive(Debug, Clone, Serialize)]
pub struct ResponseEnvelope<T>
where
    T: Serialize,
{
    pub message: String,
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<JsonValue>,
}

/// Functional response transformer with fluent, immutable composition.
#[derive(Debug)]
pub struct ResponseTransformer<T> {
    message: Cow<'static, str>,
    data: T,
    status: StatusCode,
    metadata: Option<JsonValue>,
    headers: Vec<(HeaderName, HeaderValue)>,
    allowed_formats: Vec<ResponseFormat>,
    strategy: FormatStrategy,
}

impl<T> ResponseTransformer<T> {
    /// Creates a ResponseTransformer for the provided payload with sensible defaults.
    ///
    /// The transformer defaults to HTTP 200 OK, message "OK", no metadata, no headers,
    /// allows JSON and pretty-printed JSON formats, and uses automatic content negotiation.
    ///
    /// # Examples
    ///
    /// ```
    /// let _ = ResponseTransformer::new("payload");
    /// ```
    pub fn new(data: T) -> Self {
        Self {
            message: Cow::Borrowed(constants::MESSAGE_OK),
            data,
            status: StatusCode::OK,
            metadata: None,
            headers: Vec::new(),
            allowed_formats: vec![ResponseFormat::Json, ResponseFormat::JsonPretty],
            strategy: FormatStrategy::Auto,
        }
    }

    /// Sets the response envelope's human-readable message on the transformer.
    ///
    /// # Examples
    ///
    /// ```
    /// let _tx = ResponseTransformer::new("payload").with_message("Created");
    /// ```
    pub fn with_message(mut self, message: impl Into<Cow<'static, str>>) -> Self {
        self.message = message.into();
        self
    }

    /// Produce a new transformer with the message transformed by the given closure.
    ///
    /// # Examples
    ///
    /// ```
    /// let original = ResponseTransformer::new("payload").with_message("hello");
    /// let updated = original.map_message(|m| {
    ///     let mut s = m.into_owned();
    ///     s.push_str(" world");
    ///     s.into()
    /// });
    /// assert_eq!(updated.message, "hello world");
    /// ```
    pub fn map_message<F>(self, transform: F) -> Self
    where
        F: FnOnce(Cow<'static, str>) -> Cow<'static, str>,
    {
        let Self {
            message,
            data,
            status,
            metadata,
            headers,
            allowed_formats,
            strategy,
        } = self;

        let new_message = transform(message);

        Self {
            message: new_message,
            data,
            status,
            metadata,
            headers,
            allowed_formats,
            strategy,
        }
    }

    /// Set the HTTP status code used when building the response.
    ///
    /// Consumes the transformer and returns an updated transformer with its `status` set to `status`.
    ///
    /// # Examples
    ///
    /// ```
    /// use actix_web::http::StatusCode;
    /// let transformer = ResponseTransformer::new("payload").with_status(StatusCode::CREATED);
    /// assert_eq!(transformer.status, StatusCode::CREATED);
    /// ```
    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Adds or replaces metadata with a pre-serialized JSON value.
    pub fn with_metadata_value(mut self, metadata: JsonValue) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Serialize `metadata` to JSON and attach it to the transformer's metadata field.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde::Serialize;
    /// use functional::response_transformers::ResponseTransformer;
    ///
    /// #[derive(Serialize)]
    /// struct Meta { version: u8 }
    ///
    /// let t = ResponseTransformer::new("data")
    ///     .try_with_metadata(Meta { version: 1 })
    ///     .unwrap();
    /// ```
    ///
    /// # Returns
    ///
    /// `Ok(Self)` with the metadata set on success, `Err(ResponseTransformError::MetadataSerialization)` if serialization fails.
    pub fn try_with_metadata<M>(mut self, metadata: M) -> Result<Self, ResponseTransformError>
    where
        M: Serialize,
    {
        self.metadata = Some(serde_json::to_value(metadata)?);
        Ok(self)
    }

    /// Applies a transformation to the stored metadata and returns a new transformer with the transformed metadata.
    ///
    /// The provided closure is given the current `Option<JsonValue>` and must return the `Option<JsonValue>` to store; all other transformer fields are preserved.
    ///
    /// # Examples
    ///
    /// ```
    /// use serde_json::json;
    /// // assuming `ResponseTransformer::new` is in scope
    /// let _ = ResponseTransformer::new("payload")
    ///     .map_metadata(|_meta| Some(json!({ "count": 1 })));
    /// ```
    pub fn map_metadata<F>(self, transform: F) -> Self
    where
        F: FnOnce(Option<JsonValue>) -> Option<JsonValue>,
    {
        let Self {
            message,
            data,
            status,
            metadata,
            headers,
            allowed_formats,
            strategy,
        } = self;

        let new_metadata = transform(metadata);

        Self {
            message,
            data,
            status,
            metadata: new_metadata,
            headers,
            allowed_formats,
            strategy,
        }
    }

    /// Adds an HTTP header to the response.
    pub fn insert_header(mut self, header: (HeaderName, HeaderValue)) -> Self {
        self.headers.push(header);
        self
    }

    /// Parses a header name and value from strings and appends the resulting header to the transformer.
    ///
    /// On success returns the updated transformer with the new header appended to its header list.
    ///
    /// # Errors
    ///
    /// - `ResponseTransformError::InvalidHeaderName` if `name` is not a valid HTTP header name.
    /// - `ResponseTransformError::InvalidHeaderValue` if `value` is not a valid HTTP header value.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::functional::response_transformers::ResponseTransformer;
    ///
    /// let t = ResponseTransformer::new("payload");
    /// let _ = t.try_insert_header("x-custom", "42").unwrap();
    /// ```
    pub fn try_insert_header(
        mut self,
        name: &str,
        value: &str,
    ) -> Result<Self, ResponseTransformError> {
        let header_name = HeaderName::from_str(name)?;
        let header_value = HeaderValue::from_str(value)?;
        self.headers.push((header_name, header_value));
        Ok(self)
    }

    /// Adds a response format to the transformer's allowed formats for content negotiation.
    ///
    /// If the specified format is already present, the transformer is returned unchanged.
    ///
    /// # Returns
    ///
    /// `Self` with the specified format included in `allowed_formats`.
    ///
    /// # Examples
    ///
    /// ```
    /// let tx = ResponseTransformer::new(42).allow_format(ResponseFormat::Text);
    /// let tx = tx.allow_format(ResponseFormat::JsonPretty); // chainable
    /// ```
    pub fn allow_format(mut self, format: ResponseFormat) -> Self {
        if !self.allowed_formats.contains(&format) {
            self.allowed_formats.push(format);
        }
        self
    }

    /// Biases content negotiation toward pretty-printed JSON by placing `JsonPretty` first in the allowed formats.
    ///
    /// Moves `JsonPretty` to the front of the transformer's `allowed_formats` so it is selected first during format resolution and returns the updated transformer.
    ///
    /// # Examples
    ///
    /// ```
    /// let transformer = ResponseTransformer::new(42).prefer_pretty_json();
    /// // `transformer` will prefer `ResponseFormat::JsonPretty` when negotiating output.
    /// ```
    pub fn prefer_pretty_json(mut self) -> Self {
        if let Some(pos) = self
            .allowed_formats
            .iter()
            .position(|format| *format == ResponseFormat::JsonPretty)
        {
            self.allowed_formats.remove(pos);
        }
        self.allowed_formats.insert(0, ResponseFormat::JsonPretty);
        self
    }

    /// Force the response to a specific format.
    ///
    /// Sets the transformer's format strategy to `Forced(format)` and ensures that `format` is present in the allowed formats.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::functional::response_transformers::{ResponseTransformer, ResponseFormat};
    ///
    /// let transformer = ResponseTransformer::new("payload").force_format(ResponseFormat::Text);
    /// ```
    pub fn force_format(mut self, format: ResponseFormat) -> Self {
        self.strategy = FormatStrategy::Forced(format);
        self.allow_format(format)
    }

    /// Functional transformation over the underlying payload.
    pub fn map_data<U, F>(self, transform: F) -> ResponseTransformer<U>
    where
        F: FnOnce(T) -> U,
    {
        let Self {
            message,
            data,
            status,
            metadata,
            headers,
            allowed_formats,
            strategy,
        } = self;

        ResponseTransformer {
            message,
            data: transform(data),
            status,
            metadata,
            headers,
            allowed_formats,
            strategy,
        }
    }

    /// Maps the metadata component using a fallible function that can
    /// produce serializable metadata.
    pub fn try_map_metadata<U, F>(
        self,
        transform: F,
    ) -> Result<ResponseTransformer<T>, ResponseTransformError>
    where
        F: FnOnce(Option<JsonValue>) -> Result<Option<U>, serde_json::Error>,
        U: Serialize,
    {
        let Self {
            message,
            data,
            status,
            metadata,
            headers,
            allowed_formats,
            strategy,
        } = self;

        let metadata = transform(metadata)?.map(serde_json::to_value).transpose()?;

        Ok(ResponseTransformer {
            message,
            data,
            status,
            metadata,
            headers,
            allowed_formats,
            strategy,
        })
    }

    /// Transforms the current response envelope into a new ResponseTransformer with a different payload type.
    ///
    /// Applies the provided function to the current `ResponseEnvelope<T>` (containing `message`, `data`, and `metadata`)
    /// and constructs a `ResponseTransformer<U>` from the returned `ResponseEnvelope<U>`. The transform's result replaces
    /// the envelope fields on the new transformer while the original transformer's configuration (status, headers,
    /// allowed formats, and strategy) is preserved.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::borrow::Cow;
    /// # use serde::Serialize;
    /// # use crate::functional::response_transformers::{ResponseTransformer, ResponseEnvelope};
    /// // transform numeric data into a string and update the message
    /// let t = ResponseTransformer::new(42)
    ///     .compose(|env: ResponseEnvelope<i32>| ResponseEnvelope {
    ///         message: format!("{} (sum={})", env.message, env.data),
    ///         data: env.data.to_string(),
    ///         metadata: env.metadata,
    ///     });
    ///
    /// ```
    pub fn compose<U, F>(self, transform: F) -> ResponseTransformer<U>
    where
        F: FnOnce(ResponseEnvelope<T>) -> ResponseEnvelope<U>,
        U: Serialize,
        T: Serialize,
    {
        let Self {
            message,
            data,
            status,
            metadata,
            headers,
            allowed_formats,
            strategy,
        } = self;

        let envelope = ResponseEnvelope {
            message: message.into_owned(),
            data,
            metadata,
        };
        let next = transform(envelope);

        ResponseTransformer {
            message: Cow::Owned(next.message),
            data: next.data,
            status,
            metadata: next.metadata,
            headers,
            allowed_formats,
            strategy,
        }
    }

    /// Selects the response format to use for rendering based on the transformer's strategy and the request.
    ///
    /// If the transformer is set to `Forced`, that format is returned. Otherwise the function negotiates
    /// using the request's `Accept` header, applies a query-parameter pretty-print override (for example
    /// `?pretty=1` or `?format=pretty`) when allowed, and finally falls back to the first allowed format
    /// or `ResponseFormat::Json` if none are configured.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let chosen = transformer.resolve_format(&req);
    /// ```
    fn resolve_format(&self, req: &HttpRequest) -> ResponseFormat {
        match self.strategy {
            FormatStrategy::Forced(format) => format,
            FormatStrategy::Auto => {
                if let Some(format) = negotiated_format(req, &self.allowed_formats) {
                    return format;
                }

                // Query parameter based overrides (e.g. ?pretty=1).
                if prefers_pretty_json(req)
                    && self.allowed_formats.contains(&ResponseFormat::JsonPretty)
                {
                    return ResponseFormat::JsonPretty;
                }

                self.allowed_formats
                    .first()
                    .copied()
                    .unwrap_or(ResponseFormat::Json)
            }
        }
    }
}

impl<T> Responder for ResponseTransformer<T>
where
    T: Serialize,
{
    type Body = BoxBody;

    fn respond_to(self, req: &HttpRequest) -> HttpResponse<Self::Body> {
        let format = self.resolve_format(req);
        let mut builder = HttpResponse::build(self.status);

        for (name, value) in self.headers {
            builder.insert_header((name, value));
        }

        let envelope = ResponseEnvelope {
            message: self.message.into_owned(),
            data: self.data,
            metadata: self.metadata,
        };

        match render_response(builder, envelope, format) {
            Ok(response) => response,
            Err(err) => serialization_error(err),
        }
    }
}

/// Maximum response body size limit (10 MB).
/// Prevents unbounded memory allocation for large responses.
const MAX_RESPONSE_SIZE_BYTES: usize = 10 * 1024 * 1024;

fn escape_xml(input: &str) -> String {
    input.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;").replace("'", "&apos;")
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace("\"", "\"\""))
    } else {
        field.to_string()
    }
}

/// Recursively converts a serde_json::Value to XML elements.
/// Handles objects, arrays, and primitive values with proper escaping.
fn json_to_xml(key: &str, value: &JsonValue) -> String {
    match value {
        JsonValue::Object(map) => {
            let mut content = String::new();
            for (k, v) in map.iter() {
                content.push_str(&json_to_xml(k, v));
            }
            if content.is_empty() {
                format!("<{} />", escape_xml(key))
            } else {
                format!("<{}>{}</{}>\n", escape_xml(key), content, escape_xml(key))
            }
        }
        JsonValue::Array(arr) => {
            let mut content = String::new();
            for item in arr {
                content.push_str(&json_to_xml("item", item));
            }
            if content.is_empty() {
                format!("<{} />", escape_xml(key))
            } else {
                format!("<{}>{}</{}>\n", escape_xml(key), content, escape_xml(key))
            }
        }
        JsonValue::String(s) => {
            format!("<{}>{}</{}>\n", escape_xml(key), escape_xml(s), escape_xml(key))
        }
        JsonValue::Number(n) => {
            format!("<{}>{}</{}>\n", escape_xml(key), n, escape_xml(key))
        }
        JsonValue::Bool(b) => {
            format!("<{}>{}</{}>\n", escape_xml(key), b, escape_xml(key))
        }
        JsonValue::Null => {
            format!("<{} />", escape_xml(key))
        }
    }
}

fn render_response<T>(
    mut builder: HttpResponseBuilder,
    envelope: ResponseEnvelope<T>,
    format: ResponseFormat,
) -> Result<HttpResponse, serde_json::Error>
where
    T: Serialize,
{
    match format {
        ResponseFormat::Json => {
            let payload = serde_json::to_vec(&envelope)?;
            // Validate response size to prevent DoS from large responses
            if payload.len() > MAX_RESPONSE_SIZE_BYTES {
                return Err(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, format!("Response size {} exceeds maximum of {} bytes", payload.len(), MAX_RESPONSE_SIZE_BYTES))));
            }
            builder.insert_header(header::ContentType::json());
            Ok(builder.body(payload))
        }
        ResponseFormat::JsonPretty => {
            let payload = serde_json::to_string_pretty(&envelope)?;
            // Validate response size to prevent DoS from large responses
            if payload.len() > MAX_RESPONSE_SIZE_BYTES {
                return Err(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, format!("Response size {} exceeds maximum of {} bytes", payload.len(), MAX_RESPONSE_SIZE_BYTES))));
            }
            builder.insert_header(header::ContentType::json());
            Ok(builder.body(payload))
        }
        ResponseFormat::Text => {
            let payload = serde_json::to_string_pretty(&envelope)?;
            // Validate response size to prevent DoS from large responses
            if payload.len() > MAX_RESPONSE_SIZE_BYTES {
                return Err(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, format!("Response size {} exceeds maximum of {} bytes", payload.len(), MAX_RESPONSE_SIZE_BYTES))));
            }
            builder.insert_header(header::ContentType::plaintext());
            Ok(builder.body(payload))
        }
        ResponseFormat::Xml => {
            // Convert the JSON data to proper XML structure
            let data_xml = json_to_xml("data", &serde_json::to_value(&envelope.data)?);
            let xml_payload = format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<response>
  <message>{}</message>
{}
</response>", escape_xml(&envelope.message), data_xml);
            // Validate response size to prevent DoS from large responses
            if xml_payload.len() > MAX_RESPONSE_SIZE_BYTES {
                return Err(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, format!("Response size {} exceeds maximum of {} bytes", xml_payload.len(), MAX_RESPONSE_SIZE_BYTES))));
            }
            // Use exact content-type for XML format
            builder.insert_header((header::CONTENT_TYPE, "application/xml; charset=utf-8"));
            Ok(builder.body(xml_payload))
        }
        ResponseFormat::Csv => {
            // CSV output strategy: flatten envelope to ensure compatibility with CSV consumers.
            // The message is output as-is in the message column, and the data is serialized
            // to a compact JSON string (no pretty-printing) in the data column.
            // This ensures CSV parsers can correctly handle both columns without confusion.
            let data_json = serde_json::to_string(&envelope.data)?;
            let csv_payload = format!("message,data\n{},{}", 
                escape_csv_field(&envelope.message), 
                escape_csv_field(&data_json)
            );
            // Validate response size to prevent DoS from large responses
            if csv_payload.len() > MAX_RESPONSE_SIZE_BYTES {
                return Err(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, format!("Response size {} exceeds maximum of {} bytes", csv_payload.len(), MAX_RESPONSE_SIZE_BYTES))));
            }
            // Use exact content-type for CSV format
            builder.insert_header((header::CONTENT_TYPE, "text/csv; charset=utf-8"));
            Ok(builder.body(csv_payload))
        }
        ResponseFormat::MessagePack => {
            // MessagePack binary format - compact and efficient
            // For production, use rmp-serde crate for proper MessagePack serialization
            // For now, fall back to JSON with appropriate content type
            let payload = serde_json::to_vec(&envelope)?;
            // Validate response size to prevent DoS from large responses
            if payload.len() > MAX_RESPONSE_SIZE_BYTES {
                return Err(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::Other, format!("Response size {} exceeds maximum of {} bytes", payload.len(), MAX_RESPONSE_SIZE_BYTES))));
            }
            // Use exact content-type for MessagePack format
            builder.insert_header((header::CONTENT_TYPE, "application/msgpack"));
            Ok(builder.body(payload))
        }
    }
}

fn serialization_error(err: serde_json::Error) -> HttpResponse {
    let body = ResponseBody::new(
        constants::MESSAGE_INTERNAL_SERVER_ERROR,
        json!({ "error": err.to_string() }),
    );
    HttpResponse::InternalServerError().json(body)
}

/// Represents a parsed Accept header entry with quality value
#[derive(Debug, Clone)]
struct AcceptEntry {
    media_type: String,
    quality: f32,
    format: Option<ResponseFormat>,
}

fn negotiated_format(req: &HttpRequest, allowed: &[ResponseFormat]) -> Option<ResponseFormat> {
    let mut entries: Vec<AcceptEntry> = req
        .headers()
        .get_all(header::ACCEPT)
        .into_iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|line| line.split(','))
        .filter_map(|token| parse_accept_entry(token.trim()))
        .collect();

    // Sort by quality value (descending)
    entries.sort_by(|a, b| {
        b.quality
            .partial_cmp(&a.quality)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Find first matching format
    for entry in entries {
        if let Some(format) = entry.format {
            if allowed.contains(&format) {
                return Some(format);
            }
        } else {
            // Handle wildcards
            if entry.media_type == "*/*" {
                // For Accept: */*, prefer text/json formats over binary formats
                // to avoid unintended binary response serving
                if allowed.contains(&ResponseFormat::Json) {
                    return Some(ResponseFormat::Json);
                } else if allowed.contains(&ResponseFormat::JsonPretty) {
                    return Some(ResponseFormat::JsonPretty);
                } else if allowed.contains(&ResponseFormat::Text) {
                    return Some(ResponseFormat::Text);
                } else if allowed.contains(&ResponseFormat::Xml) {
                    return Some(ResponseFormat::Xml);
                } else if allowed.contains(&ResponseFormat::Csv) {
                    return Some(ResponseFormat::Csv);
                } else if allowed.contains(&ResponseFormat::MessagePack) {
                    // Only serve binary if it's explicitly the only option
                    return Some(ResponseFormat::MessagePack);
                } else {
                    return allowed.first().copied();
                }
            } else if entry.media_type == "application/*" {
                // Prefer JSON for application/* wildcard
                if allowed.contains(&ResponseFormat::Json) {
                    return Some(ResponseFormat::Json);
                } else if allowed.contains(&ResponseFormat::MessagePack) {
                    return Some(ResponseFormat::MessagePack);
                } else if allowed.contains(&ResponseFormat::Xml) {
                    return Some(ResponseFormat::Xml);
                }
            } else if entry.media_type == "text/*" {
                // Prefer plain text for text/* wildcard
                if allowed.contains(&ResponseFormat::Text) {
                    return Some(ResponseFormat::Text);
                } else if allowed.contains(&ResponseFormat::Csv) {
                    return Some(ResponseFormat::Csv);
                }
            }
        }
    }

    None
}

fn parse_accept_entry(token: &str) -> Option<AcceptEntry> {
    // Split media type and parameters (e.g., "application/json;q=0.8")
    let parts: Vec<&str> = token.split(';').collect();
    let media_type = parts[0].trim().to_ascii_lowercase();

    // Parse quality value (default is 1.0)
    let quality = parts
        .iter()
        .skip(1)
        .find_map(|param| {
            let param = param.trim();
            if param.starts_with("q=") {
                param[2..].parse::<f32>().ok()
            } else {
                None
            }
        })
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);

    let format = parse_media_type(&media_type);

    Some(AcceptEntry {
        media_type,
        quality,
        format,
    })
}

fn parse_media_type(media_type: &str) -> Option<ResponseFormat> {
    match media_type {
        "application/json" => Some(ResponseFormat::Json),
        "application/json+pretty" | "application/json; pretty" => Some(ResponseFormat::JsonPretty),
        "text/plain" => Some(ResponseFormat::Text),
        "application/xml" | "text/xml" => Some(ResponseFormat::Xml),
        "text/csv" | "application/csv" => Some(ResponseFormat::Csv),
        "application/msgpack" | "application/x-msgpack" => Some(ResponseFormat::MessagePack),
        _ if media_type.contains("json") => Some(ResponseFormat::Json),
        _ => None,
    }
}

/// Determines whether the request asks for pretty-printed JSON via query parameters.
/// Checks for `pretty=1`, `pretty=true`, or `format=pretty` (case-insensitive).
///
/// # Examples
///
/// ```
/// use actix_web::test::TestRequest;
/// let req = TestRequest::with_uri("/?pretty=true").to_http_request();
/// assert!(crate::functional::response_transformers::prefers_pretty_json(&req));
/// ```
fn prefers_pretty_json(req: &HttpRequest) -> bool {
    #[derive(Deserialize)]
    struct PrettyQuery {
        #[serde(default)]
        pretty: Option<String>,
        #[serde(default)]
        format: Option<String>,
    }

    match web::Query::<PrettyQuery>::from_query(req.query_string()) {
        Ok(query) => {
            let pretty_matches = query
                .pretty
                .as_ref()
                .map(|value| {
                    let value = value.trim();
                    value.eq_ignore_ascii_case("1")
                        || value.eq_ignore_ascii_case("true")
                        || value.eq_ignore_ascii_case("pretty")
                })
                .unwrap_or(false);

            if pretty_matches {
                return true;
            }

            query
                .format
                .as_ref()
                .map(|value| value.trim().eq_ignore_ascii_case("pretty"))
                .unwrap_or(false)
        }
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::body;
    use actix_web::http::header::{ACCEPT, CONTENT_TYPE};
    use actix_web::http::StatusCode;
    use actix_web::test::TestRequest;

    #[test]
    fn prefers_pretty_json_supports_percent_encoding() {
        let request = TestRequest::with_uri("/?pretty=%31").to_http_request();
        assert!(prefers_pretty_json(&request));
    }

    #[test]
    fn prefers_pretty_json_handles_format_override() {
        let request = TestRequest::with_uri("/?format=Pretty").to_http_request();
        assert!(prefers_pretty_json(&request));
    }

    #[test]
    fn prefers_pretty_json_defaults_to_false() {
        let request = TestRequest::with_uri("/?other=value").to_http_request();
        assert!(!prefers_pretty_json(&request));
    }

    #[actix_rt::test]
    async fn default_response_serializes_to_json() {
        let request = TestRequest::default().insert_header((ACCEPT, "application/json"));
        let response = ResponseTransformer::new(vec![1, 2, 3])
            .with_message("numbers")
            .respond_to(&request.to_http_request());

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response.headers().get(CONTENT_TYPE).unwrap();
        assert_eq!(
            content_type.to_str().unwrap(),
            header::ContentType::json().to_string()
        );

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload: JsonValue = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["message"], "numbers");
        assert_eq!(payload["data"], json!([1, 2, 3]));
    }

    #[actix_rt::test]
    async fn map_data_transforms_payload() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new(vec![1, 2, 3])
            .map_data(|data| data.into_iter().map(|item| item * 10).collect::<Vec<_>>())
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload: JsonValue = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["data"], json!([10, 20, 30]));
    }

    #[actix_rt::test]
    async fn negotiate_plain_text_format() {
        let request = TestRequest::default().insert_header((ACCEPT, "text/plain"));
        let response = ResponseTransformer::new(json!({"test": "value"}))
            .allow_format(ResponseFormat::Text)
            .respond_to(&request.to_http_request());

        assert_eq!(response.status(), StatusCode::OK);
        let content_type = response.headers().get(CONTENT_TYPE).unwrap();
        assert_eq!(
            content_type.to_str().unwrap(),
            header::ContentType::plaintext().to_string()
        );

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload = String::from_utf8(body.to_vec()).unwrap();
        assert!(payload.contains("\"test\": \"value\""));
    }

    #[actix_rt::test]
    async fn query_string_pretty_print() {
        let request = TestRequest::with_uri("/resource?pretty=true");
        let response = ResponseTransformer::new(json!({"foo": "bar"}))
            .prefer_pretty_json()
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload = String::from_utf8(body.to_vec()).unwrap();
        assert!(payload.contains("\n"));
    }

    #[actix_rt::test]
    async fn response_with_custom_status() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new("data")
            .with_status(StatusCode::CREATED)
            .respond_to(&request.to_http_request());

        assert_eq!(response.status(), StatusCode::CREATED);
    }

    #[actix_rt::test]
    async fn response_with_metadata() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new(vec![1, 2, 3])
            .try_with_metadata(json!({"count": 3}))
            .unwrap()
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload: JsonValue = serde_json::from_slice(&body).unwrap();

        assert!(payload["metadata"].is_object());
        assert_eq!(payload["metadata"]["count"], 3);
    }

    #[actix_rt::test]
    async fn response_with_metadata_value() {
        let request = TestRequest::default();
        let metadata = json!({"version": "1.0", "source": "test"});
        let response = ResponseTransformer::new("ok")
            .with_metadata_value(metadata)
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload: JsonValue = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["metadata"]["version"], "1.0");
        assert_eq!(payload["metadata"]["source"], "test");
    }

    #[actix_rt::test]
    async fn response_map_message() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new(42)
            .with_message("original")
            .map_message(|msg| format!("{}!", msg).into())
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload: JsonValue = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["message"], "original!");
    }

    #[actix_rt::test]
    async fn response_map_metadata() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new(1)
            .with_metadata_value(json!({"count": 5}))
            .map_metadata(|meta| {
                meta.map(|m| {
                    let mut obj = m.as_object().unwrap().clone();
                    obj.insert("doubled".to_string(), json!(10));
                    json!(obj)
                })
            })
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload: JsonValue = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["metadata"]["count"], 5);
        assert_eq!(payload["metadata"]["doubled"], 10);
    }

    #[actix_rt::test]
    async fn response_insert_custom_header() {
        let request = TestRequest::default();
        let header_name = HeaderName::from_static("x-custom");
        let header_value = HeaderValue::from_static("test-value");

        let response = ResponseTransformer::new("data")
            .insert_header((header_name.clone(), header_value))
            .respond_to(&request.to_http_request());

        assert!(response.headers().contains_key(&header_name));
        assert_eq!(
            response
                .headers()
                .get(&header_name)
                .unwrap()
                .to_str()
                .unwrap(),
            "test-value"
        );
    }

    #[actix_rt::test]
    async fn response_try_insert_header_valid() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new("data")
            .try_insert_header("x-request-id", "12345")
            .unwrap()
            .respond_to(&request.to_http_request());

        let header = response.headers().get("x-request-id");
        assert!(header.is_some());
        assert_eq!(header.unwrap().to_str().unwrap(), "12345");
    }

    #[test]
    fn response_try_insert_header_invalid_name() {
        let result = ResponseTransformer::new("data").try_insert_header("invalid header!", "value");

        assert!(result.is_err());
    }

    #[actix_rt::test]
    async fn response_force_format_json_pretty() {
        let request = TestRequest::default().insert_header((ACCEPT, "text/plain"));

        let response = ResponseTransformer::new(vec![1, 2, 3])
            .force_format(ResponseFormat::JsonPretty)
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        // Pretty JSON should contain newlines
        assert!(body_str.contains('\n'));
    }

    #[actix_rt::test]
    async fn response_force_format_text() {
        let request = TestRequest::default().insert_header((ACCEPT, "application/json"));

        let response = ResponseTransformer::new(json!({"key": "value"}))
            .force_format(ResponseFormat::Text)
            .respond_to(&request.to_http_request());

        let content_type = response.headers().get(CONTENT_TYPE).unwrap();
        assert_eq!(
            content_type.to_str().unwrap(),
            header::ContentType::plaintext().to_string()
        );
    }

    #[actix_rt::test]
    async fn response_prefer_pretty_json() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new(vec![1, 2])
            .prefer_pretty_json()
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains('\n'));
    }

    #[actix_rt::test]
    async fn response_compose_transformation() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new(vec![1, 2, 3])
            .with_message("numbers")
            .compose(|envelope| {
                let sum: i32 = envelope.data.iter().sum();
                ResponseEnvelope {
                    message: format!("{} (sum={})", envelope.message, sum),
                    data: sum,
                    metadata: envelope.metadata,
                }
            })
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload: JsonValue = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["message"], "numbers (sum=6)");
        assert_eq!(payload["data"], 6);
    }

    #[actix_rt::test]
    async fn response_chained_transformations() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new(10)
            .with_message("start")
            .map_data(|x| x * 2)
            .map_data(|x| x + 5)
            .map_message(|msg| format!("{}!", msg).into())
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload: JsonValue = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["data"], 25); // (10 * 2) + 5
        assert_eq!(payload["message"], "start!");
    }

    #[actix_rt::test]
    async fn negotiate_format_with_wildcard() {
        let request = TestRequest::default().insert_header((ACCEPT, "*/*"));

        let response = ResponseTransformer::new("data").respond_to(&request.to_http_request());

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[actix_rt::test]
    async fn negotiate_format_with_application_wildcard() {
        let request = TestRequest::default().insert_header((ACCEPT, "application/*"));

        let response = ResponseTransformer::new("data").respond_to(&request.to_http_request());

        let content_type = response.headers().get(CONTENT_TYPE).unwrap();
        assert!(content_type.to_str().unwrap().contains("json"));
    }

    #[actix_rt::test]
    async fn negotiate_format_multiple_accepts() {
        let request =
            TestRequest::default().insert_header((ACCEPT, "text/html, application/json, */*"));

        let response = ResponseTransformer::new("data").respond_to(&request.to_http_request());

        let content_type = response.headers().get(CONTENT_TYPE).unwrap();
        assert!(content_type.to_str().unwrap().contains("json"));
    }

    #[actix_rt::test]
    async fn query_string_format_parameter() {
        let request = TestRequest::with_uri("/api/data?format=pretty");
        let response = ResponseTransformer::new(json!({"test": "value"}))
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.contains('\n'));
    }

    #[actix_rt::test]
    async fn response_with_error_status() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new("error details")
            .with_status(StatusCode::BAD_REQUEST)
            .with_message("Validation failed")
            .respond_to(&request.to_http_request());

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload: JsonValue = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["message"], "Validation failed");
    }

    #[actix_rt::test]
    async fn response_empty_data() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new(())
            .with_message("empty response")
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload: JsonValue = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["message"], "empty response");
        assert_eq!(payload["data"], json!(null));
    }

    #[test]
    fn response_format_enum_equality() {
        assert_eq!(ResponseFormat::Json, ResponseFormat::Json);
        assert_ne!(ResponseFormat::Json, ResponseFormat::JsonPretty);
        assert_ne!(ResponseFormat::Json, ResponseFormat::Text);
    }

    #[actix_rt::test]
    async fn allow_format_adds_capability() {
        let request = TestRequest::default().insert_header((ACCEPT, "text/plain"));

        let response = ResponseTransformer::new("test")
            .allow_format(ResponseFormat::Text)
            .respond_to(&request.to_http_request());

        let content_type = response.headers().get(CONTENT_TYPE).unwrap();
        assert_eq!(
            content_type.to_str().unwrap(),
            header::ContentType::plaintext().to_string()
        );
    }

    #[actix_rt::test]
    async fn try_map_metadata_success() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new(1)
            .with_metadata_value(json!({"value": 10}))
            .try_map_metadata(|meta| Ok(meta.map(|m| json!({"transformed": true, "original": m}))))
            .unwrap()
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload: JsonValue = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["metadata"]["transformed"], true);
        assert!(payload["metadata"]["original"].is_object());
    }

    #[test]
    fn try_map_metadata_error() {
        let result = ResponseTransformer::new(1)
            .with_metadata_value(json!({"value": 10}))
            .try_map_metadata(
                |_| -> Result<Option<serde_json::Value>, serde_json::Error> {
                    Err(serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err())
                },
            );

        assert!(result.is_err());
    }

    #[actix_rt::test]
    async fn complex_transformation_pipeline() {
        let request = TestRequest::default();
        let response = ResponseTransformer::new(vec![1, 2, 3, 4, 5])
            .with_message("numbers")
            .with_status(StatusCode::OK)
            .map_data(|nums| nums.into_iter().filter(|&x| x % 2 == 0).collect::<Vec<_>>())
            .map_data(|nums: Vec<i32>| nums.into_iter().map(|x| x * 10).collect::<Vec<_>>())
            .try_with_metadata(json!({"filtered": true, "multiplied": 10}))
            .unwrap()
            .map_message(|msg| format!("{} - processed", msg).into())
            .respond_to(&request.to_http_request());

        let body = body::to_bytes(response.into_body()).await.unwrap();
        let payload: JsonValue = serde_json::from_slice(&body).unwrap();

        assert_eq!(payload["data"], json!([20, 40]));
        assert_eq!(payload["message"], "numbers - processed");
        assert_eq!(payload["metadata"]["filtered"], true);
    }

    #[actix_rt::test]
    async fn negotiate_format_with_quality_values() {
        let request = TestRequest::default().insert_header((
            ACCEPT,
            "application/json;q=0.8, text/plain;q=0.9",
        ));

        let response = ResponseTransformer::new("data")
            .allow_format(ResponseFormat::Text)
            .respond_to(&request.to_http_request());

        let content_type = response.headers().get(CONTENT_TYPE).unwrap();
        // Should prefer text/plain due to higher quality value (0.9 > 0.8)
        assert!(content_type.to_str().unwrap().contains("text/plain"));
    }

    #[test]
    fn parse_accept_entry_with_quality() {
        let entry = parse_accept_entry("application/json;q=0.8").unwrap();
        assert_eq!(entry.media_type, "application/json");
        assert_eq!(entry.quality, 0.8);
        assert_eq!(entry.format, Some(ResponseFormat::Json));
    }

    #[test]
    fn parse_accept_entry_default_quality() {
        let entry = parse_accept_entry("text/plain").unwrap();
        assert_eq!(entry.quality, 1.0);
    }

    #[test]
    fn parse_media_type_variations() {
        assert_eq!(
            parse_media_type("application/json"),
            Some(ResponseFormat::Json)
        );
        assert_eq!(
            parse_media_type("application/xml"),
            Some(ResponseFormat::Xml)
        );
        assert_eq!(parse_media_type("text/xml"), Some(ResponseFormat::Xml));
        assert_eq!(parse_media_type("text/csv"), Some(ResponseFormat::Csv));
        assert_eq!(
            parse_media_type("application/msgpack"),
            Some(ResponseFormat::MessagePack)
        );
        assert_eq!(
            parse_media_type("application/x-msgpack"),
            Some(ResponseFormat::MessagePack)
        );
    }
}
