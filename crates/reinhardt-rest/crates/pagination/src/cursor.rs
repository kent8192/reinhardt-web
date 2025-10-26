//! Cursor-based pagination implementation

use async_trait::async_trait;
use reinhardt_exception::{Error, Result};

use crate::core::{AsyncPaginator, PaginatedResponse, Paginator, SchemaParameter};

/// Cursor-based pagination for large datasets
///
/// Provides consistent pagination even when items are added/removed.
/// Uses opaque cursor tokens instead of page numbers.
#[derive(Debug, Clone)]
pub struct CursorPagination {
    /// Default page size
    pub page_size: usize,
    /// Query parameter name for cursor
    pub cursor_query_param: String,
    /// Query parameter name for page size (optional)
    pub page_size_query_param: Option<String>,
    /// Ordering field(s) for cursor
    pub ordering: Vec<String>,
    /// Maximum allowed page size
    pub max_page_size: Option<usize>,
}

impl Default for CursorPagination {
    fn default() -> Self {
        Self {
            page_size: 10,
            cursor_query_param: "cursor".to_string(),
            page_size_query_param: Some("page_size".to_string()),
            ordering: vec!["-created".to_string()],
            max_page_size: Some(100),
        }
    }
}

impl CursorPagination {
    /// Creates a new CursorPagination with default settings
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::CursorPagination;
    ///
    /// let paginator = CursorPagination::new();
    /// assert_eq!(paginator.page_size, 10);
    /// assert_eq!(paginator.cursor_query_param, "cursor");
    /// assert_eq!(paginator.max_page_size, Some(100));
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
    /// Sets the default page size for cursor pagination
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::CursorPagination;
    ///
    /// let paginator = CursorPagination::new().page_size(20);
    /// assert_eq!(paginator.page_size, 20);
    /// ```
    pub fn page_size(mut self, size: usize) -> Self {
        self.page_size = size;
        self
    }
    /// Sets the maximum allowed page size
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::CursorPagination;
    ///
    /// let paginator = CursorPagination::new()
    ///     .page_size(10)
    ///     .max_page_size(50);
    /// assert_eq!(paginator.max_page_size, Some(50));
    /// ```
    pub fn max_page_size(mut self, size: usize) -> Self {
        self.max_page_size = Some(size);
        self
    }
    /// Sets the ordering fields for cursor-based pagination
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::CursorPagination;
    ///
    /// let paginator = CursorPagination::new()
    ///     .ordering(vec!["-created_at".to_string(), "id".to_string()]);
    /// assert_eq!(paginator.ordering, vec!["-created_at", "id"]);
    /// ```
    pub fn ordering(mut self, fields: Vec<String>) -> Self {
        self.ordering = fields;
        self
    }

    /// Encode cursor with timestamp and checksum for security
    /// Format: base64(position:timestamp:checksum)
    fn encode_cursor(&self, position: usize) -> String {
        use base64::{engine::general_purpose, Engine as _};
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Create checksum to prevent tampering
        let mut hasher = DefaultHasher::new();
        position.hash(&mut hasher);
        timestamp.hash(&mut hasher);
        let checksum = hasher.finish();

        let cursor_data = format!("{}:{}:{}", position, timestamp, checksum);
        general_purpose::STANDARD.encode(cursor_data.as_bytes())
    }

    /// Decode and validate cursor
    fn decode_cursor(&self, cursor: &str) -> Result<usize> {
        use base64::{engine::general_purpose, Engine as _};
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let decoded = general_purpose::STANDARD
            .decode(cursor)
            .map_err(|_| Error::InvalidPage("Invalid cursor".to_string()))?;
        let cursor_data = String::from_utf8(decoded)
            .map_err(|_| Error::InvalidPage("Invalid cursor encoding".to_string()))?;

        // Parse cursor components
        let parts: Vec<&str> = cursor_data.split(':').collect();
        if parts.len() != 3 {
            return Err(Error::InvalidPage("Malformed cursor".to_string()));
        }

        let position: usize = parts[0]
            .parse()
            .map_err(|_| Error::InvalidPage("Invalid cursor value".to_string()))?;
        let timestamp: u64 = parts[1]
            .parse()
            .map_err(|_| Error::InvalidPage("Invalid cursor timestamp".to_string()))?;
        let provided_checksum: u64 = parts[2]
            .parse()
            .map_err(|_| Error::InvalidPage("Invalid cursor checksum".to_string()))?;

        // Verify checksum
        let mut hasher = DefaultHasher::new();
        position.hash(&mut hasher);
        timestamp.hash(&mut hasher);
        let expected_checksum = hasher.finish();

        if provided_checksum != expected_checksum {
            return Err(Error::InvalidPage("Cursor checksum mismatch".to_string()));
        }

        // Check if cursor is too old (optional: 24 hour expiry)
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        if now - timestamp > 86400 {
            return Err(Error::Validation("Cursor expired".to_string()));
        }

        Ok(position)
    }

    fn build_url(&self, base_url: &str, cursor: &str) -> String {
        let url = url::Url::parse(base_url)
            .unwrap_or_else(|_| url::Url::parse(&format!("http://localhost{}", base_url)).unwrap());

        let mut new_url = url.clone();
        new_url
            .query_pairs_mut()
            .clear()
            .append_pair(&self.cursor_query_param, cursor);

        // Copy other query parameters (including page_size)
        for (key, value) in url.query_pairs() {
            if key != self.cursor_query_param {
                new_url.query_pairs_mut().append_pair(&key, &value);
            }
        }

        new_url.to_string()
    }
}

#[async_trait]
impl Paginator for CursorPagination {
    fn paginate<T: Clone + Send + Sync>(
        &self,
        items: &[T],
        cursor_param: Option<&str>,
        base_url: &str,
    ) -> Result<PaginatedResponse<T>> {
        let total_count = items.len();

        // Parse page_size from URL if page_size_query_param is set
        let page_size = if let Some(ref param_name) = self.page_size_query_param {
            if let Ok(url) = url::Url::parse(base_url) {
                url.query_pairs()
                    .find(|(key, _)| key == param_name)
                    .and_then(|(_, value)| value.parse::<usize>().ok())
                    .filter(|&size| size > 0) // Reject 0 or negative
                    .map(|size| {
                        // Clamp to max_page_size if set
                        if let Some(max) = self.max_page_size {
                            std::cmp::min(size, max)
                        } else {
                            size
                        }
                    })
                    .unwrap_or(self.page_size)
            } else {
                self.page_size
            }
        } else {
            self.page_size
        };

        // Get position from cursor
        let position = if let Some(cursor) = cursor_param {
            self.decode_cursor(cursor)?
        } else {
            0
        };

        // Calculate slice bounds
        let start = position;
        let end = std::cmp::min(start + page_size, total_count);

        // Get results
        let results = items[start..end].to_vec();

        // Build next/previous cursors
        let next = if end < total_count {
            let next_cursor = self.encode_cursor(end);
            Some(self.build_url(base_url, &next_cursor))
        } else {
            None
        };

        let previous = if position > 0 {
            let prev_position = if position >= self.page_size {
                position - self.page_size
            } else {
                0
            };
            let prev_cursor = self.encode_cursor(prev_position);
            Some(self.build_url(base_url, &prev_cursor))
        } else {
            None
        };

        Ok(PaginatedResponse {
            count: total_count,
            next,
            previous,
            results,
        })
    }

    fn get_schema_parameters(&self) -> Vec<SchemaParameter> {
        let mut params = vec![SchemaParameter {
            name: self.cursor_query_param.clone(),
            required: false,
            location: "query".to_string(),
            description: "The pagination cursor value.".to_string(),
            schema_type: "string".to_string(),
        }];

        if let Some(ref param_name) = self.page_size_query_param {
            params.push(SchemaParameter {
                name: param_name.clone(),
                required: false,
                location: "query".to_string(),
                description: "Number of results to return per page.".to_string(),
                schema_type: "integer".to_string(),
            });
        }

        params
    }
}

#[async_trait]
impl AsyncPaginator for CursorPagination {
    async fn apaginate<T: Clone + Send + Sync>(
        &self,
        items: &[T],
        cursor_param: Option<&str>,
        base_url: &str,
    ) -> Result<PaginatedResponse<T>> {
        // For in-memory operations, just call the sync version
        self.paginate(items, cursor_param, base_url)
    }

    fn get_schema_parameters(&self) -> Vec<SchemaParameter> {
        Paginator::get_schema_parameters(self)
    }
}
