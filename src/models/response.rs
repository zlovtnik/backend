use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseBody<T> {
    pub message: String,
    pub data: T,
}

impl<T> ResponseBody<T> {
    pub fn new(message: &str, data: T) -> ResponseBody<T> {
        ResponseBody {
            message: message.to_string(),
            data,
        }
    }
}

#[derive(Serialize)]
pub struct Page<T> {
    pub message: String,
    pub data: Vec<T>,
    pub current_cursor: i32,
    pub page_size: i64,
    pub total_elements: Option<i64>, // Made optional for performance - counts may be omitted
    pub next_cursor: Option<i32>,
}
impl<T> Page<T> {
    /// Create a `Page<T>` carrying a message, items, cursor, and pagination metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// let page = Page::new("ok", vec![1, 2, 3], 0, 10, Some(3_i64), Some(1));
    /// assert_eq!(page.message, "ok");
    /// assert_eq!(page.data, vec![1, 2, 3]);
    /// assert_eq!(page.current_cursor, 0);
    /// assert_eq!(page.page_size, 10);
    /// assert_eq!(page.total_elements, Some(3_i64));
    /// assert_eq!(page.next_cursor, Some(1));
    /// ```
    pub fn new(
        message: &str,
        data: Vec<T>,
        current_cursor: i32,
        page_size: i64,
        total_elements: Option<i64>, // Now optional
        next_cursor: Option<i32>,
    ) -> Page<T> {
        Page {
            message: message.to_string(),
            data,
            current_cursor,
            page_size,
            total_elements,
            next_cursor,
        }
    }
}
