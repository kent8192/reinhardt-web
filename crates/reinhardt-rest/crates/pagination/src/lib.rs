//! # Reinhardt Pagination
//!
//! Pagination support for Reinhardt framework, inspired by Django REST Framework's pagination.
//!
//! ## Pagination Styles
//!
//! - **PageNumberPagination**: Simple page number based pagination
//! - **LimitOffsetPagination**: Limit/offset based pagination  
//! - **CursorPagination**: Cursor-based pagination for large datasets
//!
//! ## Example
//!
//! ```rust,ignore
//! use reinhardt_pagination::{PageNumberPagination, Paginator};
//!
//! let paginator = PageNumberPagination::new()
//!     .page_size(10)
//!     .max_page_size(100);
//!
//! let page = paginator.paginate(&items, &request)?;
//! ```

use async_trait::async_trait;
use reinhardt_exception::{Error, Result};
use serde::{Deserialize, Serialize};

/// Represents pagination metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMetadata {
    pub count: usize,
    pub next: Option<String>,
    pub previous: Option<String>,
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub count: usize,
    pub next: Option<String>,
    pub previous: Option<String>,
    pub results: Vec<T>,
}

impl<T> PaginatedResponse<T> {
    /// Creates a new paginated response with results and metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::{PaginatedResponse, PaginationMetadata};
    ///
    /// let metadata = PaginationMetadata {
    ///     count: 100,
    ///     next: Some("/api/items?page=2".to_string()),
    ///     previous: None,
    /// };
    /// let items = vec![1, 2, 3, 4, 5];
    /// let response = PaginatedResponse::new(items, metadata);
    /// assert_eq!(response.count, 100);
    /// assert_eq!(response.results.len(), 5);
    /// ```
    pub fn new(results: Vec<T>, metadata: PaginationMetadata) -> Self {
        Self {
            count: metadata.count,
            next: metadata.next,
            previous: metadata.previous,
            results,
        }
    }
}

/// Represents a single page of results
#[derive(Debug, Clone)]
pub struct Page<T> {
    /// Items in this page
    pub object_list: Vec<T>,
    /// Current page number (1-indexed)
    pub number: usize,
    /// Total number of pages
    pub num_pages: usize,
    /// Total number of items across all pages
    pub count: usize,
    /// Items per page
    pub page_size: usize,
}

impl<T: Clone> Page<T> {
    /// Creates a new page with the given parameters.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let items = vec!["item1", "item2", "item3"];
    /// let page = Page::new(items, 1, 10, 30, 3);
    /// assert_eq!(page.number, 1);
    /// assert_eq!(page.num_pages, 10);
    /// assert_eq!(page.count, 30);
    /// assert_eq!(page.object_list.len(), 3);
    /// ```
    pub fn new(
        object_list: Vec<T>,
        number: usize,
        num_pages: usize,
        count: usize,
        page_size: usize,
    ) -> Self {
        Self {
            object_list,
            number,
            num_pages,
            count,
            page_size,
        }
    }
    /// Returns the 1-based index of the first item on this page
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let items = vec!["a", "b", "c"];
    /// let page = Page::new(items, 2, 5, 15, 3);
    /// assert_eq!(page.start_index(), 4); // (2-1) * 3 + 1 = 4
    /// ```
    pub fn start_index(&self) -> usize {
        if self.object_list.is_empty() {
            0
        } else {
            (self.number - 1) * self.page_size + 1
        }
    }
    /// Returns the 1-based index of the last item on this page
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let items = vec!["a", "b", "c"];
    /// let page = Page::new(items, 2, 5, 15, 3);
    /// assert_eq!(page.end_index(), 6); // start_index (4) + len (3) - 1 = 6
    /// ```
    pub fn end_index(&self) -> usize {
        if self.object_list.is_empty() {
            0
        } else {
            self.start_index() + self.object_list.len() - 1
        }
    }
    /// Returns true if there is a next page
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let items = vec![1, 2, 3];
    /// let page = Page::new(items, 2, 5, 50, 10);
    /// assert!(page.has_next());
    ///
    /// let last_page = Page::new(vec![1], 5, 5, 50, 10);
    /// assert!(!last_page.has_next());
    /// ```
    pub fn has_next(&self) -> bool {
        self.number < self.num_pages
    }
    /// Returns true if there is a previous page
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let items = vec![1, 2, 3];
    /// let page = Page::new(items, 2, 5, 50, 10);
    /// assert!(page.has_previous());
    ///
    /// let first_page = Page::new(vec![1], 1, 5, 50, 10);
    /// assert!(!first_page.has_previous());
    /// ```
    pub fn has_previous(&self) -> bool {
        self.number > 1
    }
    /// Returns true if there are other pages (previous or next)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let middle_page = Page::new(vec![1, 2, 3], 2, 5, 50, 10);
    /// assert!(middle_page.has_other_pages());
    ///
    /// let only_page = Page::new(vec![1], 1, 1, 1, 10);
    /// assert!(!only_page.has_other_pages());
    /// ```
    pub fn has_other_pages(&self) -> bool {
        self.has_previous() || self.has_next()
    }
    /// Returns the next page number
    ///
    /// # Errors
    /// Returns `InvalidPage` if there is no next page
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let page = Page::new(vec![1, 2, 3], 2, 5, 50, 10);
    /// assert_eq!(page.next_page_number().unwrap(), 3);
    ///
    /// let last_page = Page::new(vec![1], 5, 5, 50, 10);
    /// assert!(last_page.next_page_number().is_err());
    /// ```
    pub fn next_page_number(&self) -> Result<usize> {
        if self.has_next() {
            Ok(self.number + 1)
        } else {
            Err(Error::Validation(
                "That page contains no results".to_string(),
            ))
        }
    }
    /// Returns the previous page number
    ///
    /// # Errors
    /// Returns `InvalidPage` if there is no previous page
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let page = Page::new(vec![1, 2, 3], 3, 5, 50, 10);
    /// assert_eq!(page.previous_page_number().unwrap(), 2);
    ///
    /// let first_page = Page::new(vec![1], 1, 5, 50, 10);
    /// assert!(first_page.previous_page_number().is_err());
    /// ```
    pub fn previous_page_number(&self) -> Result<usize> {
        if self.has_previous() {
            Ok(self.number - 1)
        } else {
            Err(Error::InvalidPage(
                "That page number is less than 1".to_string(),
            ))
        }
    }
    /// Returns the length of items in this page
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let items = vec!["a", "b", "c"];
    /// let page = Page::new(items, 1, 3, 10, 5);
    /// assert_eq!(page.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.object_list.len()
    }
    /// Returns true if this page contains no items
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let empty_page: Page<i32> = Page::new(vec![], 1, 1, 0, 10);
    /// assert!(empty_page.is_empty());
    ///
    /// let page = Page::new(vec![1], 1, 1, 1, 10);
    /// assert!(!page.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.object_list.is_empty()
    }
    /// Returns an iterator over all page numbers (1-indexed)
    ///
    /// This is equivalent to Django's `paginator.page_range`
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let page = Page::new(vec![1], 2, 5, 50, 10);
    /// let pages: Vec<usize> = page.page_range().collect();
    /// assert_eq!(pages, vec![1, 2, 3, 4, 5]);
    /// ```
    pub fn page_range(&self) -> std::ops::RangeInclusive<usize> {
        1..=self.num_pages
    }
    /// Returns an elided page range with ellipsis for long ranges
    ///
    /// This is equivalent to Django's `paginator.get_elided_page_range()`
    ///
    /// # Arguments
    /// * `on_each_side` - Number of pages on each side of current page (default: 3)
    /// * `on_ends` - Number of pages on start and end (default: 2)
    ///
    /// # Returns
    /// Vector of page numbers or `None` (representing ellipsis)
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    // For a large page range, ellipsis (None) are added
    /// let page = Page::new(vec![1], 10, 20, 200, 10);
    /// let elided = page.get_elided_page_range(2, 2);
    // Result like: [Some(1), Some(2), None, Some(8), Some(9), Some(10), Some(11), Some(12), None, Some(19), Some(20)]
    /// assert!(elided.contains(&None)); // Contains ellipsis
    /// assert!(elided.contains(&Some(10))); // Contains current page
    /// ```
    pub fn get_elided_page_range(&self, on_each_side: usize, on_ends: usize) -> Vec<Option<usize>> {
        let mut result = Vec::new();

        // If we have few enough pages, don't elide
        let needed_pages = on_each_side * 2 + 1 + on_ends * 2;
        if self.num_pages <= needed_pages {
            return (1..=self.num_pages).map(Some).collect();
        }

        // Start pages
        for i in 1..=on_ends {
            if i <= self.num_pages {
                result.push(Some(i));
            }
        }

        // Left ellipsis
        let left_start = self.number.saturating_sub(on_each_side);
        if left_start > on_ends + 1 {
            result.push(None); // Ellipsis
        }

        // Middle pages
        let middle_start = std::cmp::max(on_ends + 1, left_start);
        let middle_end = std::cmp::min(self.num_pages - on_ends, self.number + on_each_side);

        for i in middle_start..=middle_end {
            if i > on_ends && i <= self.num_pages - on_ends {
                result.push(Some(i));
            }
        }

        // Right ellipsis
        if middle_end < self.num_pages - on_ends {
            result.push(None); // Ellipsis
        }

        // End pages
        for i in (self.num_pages - on_ends + 1)..=self.num_pages {
            if i > middle_end {
                result.push(Some(i));
            }
        }

        result
    }
    /// Get an item by index
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let items = vec!["a", "b", "c"];
    /// let page = Page::new(items, 1, 1, 3, 10);
    /// assert_eq!(page.get(0), Some(&"a"));
    /// assert_eq!(page.get(2), Some(&"c"));
    /// assert_eq!(page.get(3), None);
    /// ```
    pub fn get(&self, index: usize) -> Option<&T> {
        self.object_list.get(index)
    }
    /// Get a slice of items
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::Page;
    ///
    /// let items = vec!["a", "b", "c", "d", "e"];
    /// let page = Page::new(items, 1, 1, 5, 10);
    /// assert_eq!(page.get_slice(1..4), Some(&["b", "c", "d"][..]));
    /// assert_eq!(page.get_slice(0..2), Some(&["a", "b"][..]));
    /// ```
    pub fn get_slice(&self, range: std::ops::Range<usize>) -> Option<&[T]> {
        self.object_list.get(range)
    }
}

// Implement Index trait for direct indexing
impl<T: Clone> std::ops::Index<usize> for Page<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        &self.object_list[index]
    }
}

// Implement IntoIterator for Page
impl<T: Clone> IntoIterator for Page<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.object_list.into_iter()
    }
}

// Implement IntoIterator for &Page
impl<'a, T: Clone> IntoIterator for &'a Page<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.object_list.iter()
    }
}

/// Trait for pagination implementations
#[async_trait]
pub trait Paginator: Send + Sync {
    /// Paginate the given items based on request parameters
    fn paginate<T: Clone + Send + Sync>(
        &self,
        items: &[T],
        page_param: Option<&str>,
        base_url: &str,
    ) -> Result<PaginatedResponse<T>>;

    /// Get the pagination metadata for schema generation
    fn get_schema_parameters(&self) -> Vec<SchemaParameter>;
}

/// Async version of Paginator trait
///
/// This trait provides async pagination support, equivalent to Django's AsyncPaginator.
/// For in-memory operations, this simply wraps the sync implementation.
/// This trait becomes useful when integrating with async data sources (databases, APIs).
#[async_trait]
pub trait AsyncPaginator: Send + Sync {
    /// Async version of paginate
    ///
    /// Paginate the given items based on request parameters asynchronously.
    async fn apaginate<T: Clone + Send + Sync>(
        &self,
        items: &[T],
        page_param: Option<&str>,
        base_url: &str,
    ) -> Result<PaginatedResponse<T>>;

    /// Get the pagination metadata for schema generation
    /// (Same as sync version - no I/O involved)
    fn get_schema_parameters(&self) -> Vec<SchemaParameter>;
}

/// Schema parameter for OpenAPI/documentation
#[derive(Debug, Clone, Serialize)]
pub struct SchemaParameter {
    pub name: String,
    pub required: bool,
    pub location: String,
    pub description: String,
    pub schema_type: String,
}

/// Page number based pagination
///
/// Example URLs:
/// - `http://api.example.org/accounts/?page=4`
/// - `http://api.example.org/accounts/?page=4&page_size=100`
#[derive(Debug, Clone)]
pub struct PageNumberPagination {
    /// Default page size
    pub page_size: usize,
    /// Query parameter name for page number
    pub page_query_param: String,
    /// Query parameter name for page size (optional)
    pub page_size_query_param: Option<String>,
    /// Maximum allowed page size
    pub max_page_size: Option<usize>,
    /// Strings that represent the last page
    pub last_page_strings: Vec<String>,
    /// Minimum number of items allowed on the last page
    /// If the last page has fewer items than this, they are merged with the previous page
    pub orphans: usize,
    /// Whether to allow an empty first page
    pub allow_empty_first_page: bool,
    /// Custom error messages
    pub error_messages: ErrorMessages,
}

/// Custom error messages for pagination
#[derive(Debug, Clone)]
pub struct ErrorMessages {
    /// Error message for invalid page number (not parseable as integer)
    pub invalid_page: String,
    /// Error message for page number less than 1
    pub min_page: String,
    /// Error message for page number beyond available pages
    pub no_results: String,
}

impl Default for ErrorMessages {
    fn default() -> Self {
        Self {
            invalid_page: "Invalid page number".to_string(),
            min_page: "That page number is less than 1".to_string(),
            no_results: "That page contains no results".to_string(),
        }
    }
}

impl Default for PageNumberPagination {
    fn default() -> Self {
        Self {
            page_size: 10,
            page_query_param: "page".to_string(),
            page_size_query_param: None,
            max_page_size: None,
            last_page_strings: vec!["last".to_string()],
            orphans: 0,
            allow_empty_first_page: true,
            error_messages: ErrorMessages::default(),
        }
    }
}

impl PageNumberPagination {
    /// Creates a new PageNumberPagination with default settings
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::PageNumberPagination;
    ///
    /// let paginator = PageNumberPagination::new();
    /// assert_eq!(paginator.page_size, 10);
    /// assert_eq!(paginator.page_query_param, "page");
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
    /// Sets the default page size for pagination
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::PageNumberPagination;
    ///
    /// let paginator = PageNumberPagination::new().page_size(20);
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
    /// use reinhardt_pagination::PageNumberPagination;
    ///
    /// let paginator = PageNumberPagination::new()
    ///     .page_size(10)
    ///     .max_page_size(100);
    /// assert_eq!(paginator.max_page_size, Some(100));
    /// ```
    pub fn max_page_size(mut self, size: usize) -> Self {
        self.max_page_size = Some(size);
        self
    }
    /// Sets the query parameter name for custom page size
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::PageNumberPagination;
    ///
    /// let paginator = PageNumberPagination::new()
    ///     .page_size_query_param("limit");
    /// assert_eq!(paginator.page_size_query_param, Some("limit".to_string()));
    /// ```
    pub fn page_size_query_param(mut self, param: impl Into<String>) -> Self {
        self.page_size_query_param = Some(param.into());
        self
    }
    /// Sets the minimum number of items allowed on the last page
    ///
    /// If the last page has fewer items than this, they are merged with the previous page
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::PageNumberPagination;
    ///
    /// let paginator = PageNumberPagination::new()
    ///     .page_size(10)
    ///     .orphans(3);
    /// assert_eq!(paginator.orphans, 3);
    /// ```
    pub fn orphans(mut self, orphans: usize) -> Self {
        self.orphans = orphans;
        self
    }
    /// Sets whether to allow an empty first page
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::PageNumberPagination;
    ///
    /// let paginator = PageNumberPagination::new()
    ///     .allow_empty_first_page(false);
    /// assert_eq!(paginator.allow_empty_first_page, false);
    /// ```
    pub fn allow_empty_first_page(mut self, allow: bool) -> Self {
        self.allow_empty_first_page = allow;
        self
    }
    /// Sets custom error messages for pagination errors
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::{PageNumberPagination, ErrorMessages};
    ///
    /// let messages = ErrorMessages {
    ///     invalid_page: "Invalid page!".to_string(),
    ///     min_page: "Page too low!".to_string(),
    ///     no_results: "No results!".to_string(),
    /// };
    /// let paginator = PageNumberPagination::new()
    ///     .error_messages(messages);
    /// assert_eq!(paginator.error_messages.invalid_page, "Invalid page!");
    /// ```
    pub fn error_messages(mut self, messages: ErrorMessages) -> Self {
        self.error_messages = messages;
        self
    }
    /// Get a page, returning a valid page even with invalid arguments
    ///
    /// This is a lenient version that:
    /// - Returns the first page if the page number is invalid (not parseable)
    /// - Returns the last page if the page number is out of range
    /// - Never returns an error
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::{PageNumberPagination, Page};
    ///
    /// let paginator = PageNumberPagination::new().page_size(5);
    /// let items: Vec<i32> = (1..=20).collect();
    ///
    // Get page 2
    /// let page = paginator.get_page(&items, Some("2"));
    /// assert_eq!(page.number, 2);
    /// assert_eq!(page.len(), 5);
    ///
    // Invalid page number defaults to page 1
    /// let page = paginator.get_page(&items, Some("invalid"));
    /// assert_eq!(page.number, 1);
    ///
    // Out of range page number returns last page
    /// let page = paginator.get_page(&items, Some("100"));
    /// assert_eq!(page.number, 4);
    /// ```
    pub fn get_page<T: Clone>(&self, items: &[T], page_param: Option<&str>) -> Page<T> {
        let total_count = items.len();

        // Calculate total pages (same logic as paginate)
        let total_pages = if total_count == 0 {
            if self.allow_empty_first_page {
                1
            } else {
                1 // Return 1 even if empty to avoid error
            }
        } else {
            if total_count <= self.page_size {
                1
            } else {
                let pages = total_count / self.page_size;
                let remainder = total_count % self.page_size;
                if remainder > 0 && remainder <= self.orphans {
                    pages
                } else if remainder > 0 {
                    pages + 1
                } else {
                    pages
                }
            }
        };

        // Parse page number with fallback
        let page_number = page_param
            .and_then(|p| self.parse_page_number(p, total_pages).ok())
            .unwrap_or(1); // Default to 1 if parsing fails

        // Clamp page number to valid range
        let page_number = if page_number > total_pages && total_count > 0 {
            total_pages // Return last page if out of range
        } else if page_number == 0 {
            1
        } else {
            page_number
        };

        // Calculate offsets
        let (start, end) = if total_count == 0 {
            (0, 0)
        } else if page_number == total_pages {
            let start = (page_number - 1) * self.page_size;
            (start, total_count)
        } else {
            let start = (page_number - 1) * self.page_size;
            let end = std::cmp::min(start + self.page_size, total_count);
            (start, end)
        };

        let object_list = items[start..end].to_vec();

        Page {
            object_list,
            number: page_number,
            num_pages: total_pages,
            count: total_count,
            page_size: self.page_size,
        }
    }
    /// Async version of get_page
    ///
    /// Returns a valid page even with invalid arguments, never errors.
    /// This is the async equivalent of Django's `aget_page()`.
    ///
    /// # Note
    /// For in-memory operations (current implementation), this simply calls
    /// the sync version since no I/O is involved.
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::PageNumberPagination;
    ///
    /// async fn example() {
    ///     let paginator = PageNumberPagination::new().page_size(5);
    ///     let items: Vec<i32> = (1..=20).collect();
    ///
    ///     let page = paginator.aget_page(&items, Some("2")).await;
    ///     assert_eq!(page.number, 2);
    ///     assert_eq!(page.len(), 5);
    /// }
    /// ```
    pub async fn aget_page<T: Clone + Send + Sync>(
        &self,
        items: &[T],
        page_param: Option<&str>,
    ) -> Page<T> {
        self.get_page(items, page_param)
    }

    fn parse_page_number(&self, page_str: &str, total_pages: usize) -> Result<usize> {
        // Check if it's a "last" page string
        if self.last_page_strings.iter().any(|s| s == page_str) {
            return Ok(total_pages);
        }

        // Try to parse as integer first
        if let Ok(n) = page_str.parse::<usize>() {
            if n == 0 {
                return Err(Error::InvalidPage(self.error_messages.min_page.clone()));
            }
            return Ok(n);
        }

        // Try to parse as float and convert to integer
        if let Ok(f) = page_str.parse::<f64>() {
            // Check if it's a valid integer float (e.g., 1.0, 2.0)
            if f.fract() == 0.0 && f > 0.0 {
                let n = f as usize;
                if n == 0 {
                    return Err(Error::InvalidPage(self.error_messages.min_page.clone()));
                }
                return Ok(n);
            }
        }

        // If all parsing failed, return error
        Err(Error::InvalidPage(self.error_messages.invalid_page.clone()))
    }

    fn build_url(&self, base_url: &str, page: usize) -> String {
        let url = url::Url::parse(base_url)
            .unwrap_or_else(|_| url::Url::parse(&format!("http://localhost{}", base_url)).unwrap());

        let mut new_url = url.clone();
        new_url
            .query_pairs_mut()
            .clear()
            .append_pair(&self.page_query_param, &page.to_string());

        // Copy other query parameters
        for (key, value) in url.query_pairs() {
            if key != self.page_query_param {
                new_url.query_pairs_mut().append_pair(&key, &value);
            }
        }

        new_url.to_string()
    }
}

#[async_trait]
impl Paginator for PageNumberPagination {
    fn paginate<T: Clone + Send + Sync>(
        &self,
        items: &[T],
        page_param: Option<&str>,
        base_url: &str,
    ) -> Result<PaginatedResponse<T>> {
        let total_count = items.len();

        // Handle empty list with allow_empty_first_page=false
        if total_count == 0 && !self.allow_empty_first_page {
            return Err(Error::InvalidPage(self.error_messages.no_results.clone()));
        }

        // Calculate total pages considering orphans
        let total_pages = if total_count == 0 {
            if self.allow_empty_first_page {
                1
            } else {
                0
            }
        } else {
            // Calculate pages with orphans consideration
            if total_count <= self.page_size {
                1
            } else {
                let pages = total_count / self.page_size;
                let remainder = total_count % self.page_size;

                // If remainder is small enough (orphans), merge with previous page
                if remainder > 0 && remainder <= self.orphans {
                    // Merge with previous page
                    pages
                } else if remainder > 0 {
                    // Create new page
                    pages + 1
                } else {
                    pages
                }
            }
        };

        // Get page number
        let page_number = if let Some(param) = page_param {
            self.parse_page_number(param, total_pages)?
        } else {
            1
        };

        // Validate page number
        if page_number > total_pages && total_count > 0 {
            return Err(Error::InvalidPage(self.error_messages.no_results.clone()));
        }

        // Calculate offset considering orphans
        let (start, end) = if total_count == 0 {
            (0, 0)
        } else if page_number == total_pages {
            // Last page: might include orphans from calculation
            let start = (page_number - 1) * self.page_size;
            (start, total_count)
        } else {
            let start = (page_number - 1) * self.page_size;
            let end = std::cmp::min(start + self.page_size, total_count);
            (start, end)
        };

        // Get page results
        let results = items[start..end].to_vec();

        // Build next/previous links
        let next = if page_number < total_pages {
            Some(self.build_url(base_url, page_number + 1))
        } else {
            None
        };

        let previous = if page_number > 1 {
            Some(self.build_url(base_url, page_number - 1))
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
            name: self.page_query_param.clone(),
            required: false,
            location: "query".to_string(),
            description: "A page number within the paginated result set.".to_string(),
            schema_type: "integer".to_string(),
        }];

        if let Some(ref page_size_param) = self.page_size_query_param {
            params.push(SchemaParameter {
                name: page_size_param.clone(),
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
impl AsyncPaginator for PageNumberPagination {
    async fn apaginate<T: Clone + Send + Sync>(
        &self,
        items: &[T],
        page_param: Option<&str>,
        base_url: &str,
    ) -> Result<PaginatedResponse<T>> {
        // For in-memory operations, just call the sync version
        self.paginate(items, page_param, base_url)
    }

    fn get_schema_parameters(&self) -> Vec<SchemaParameter> {
        Paginator::get_schema_parameters(self)
    }
}

/// Limit/offset based pagination
///
/// Example URLs:
/// - `http://api.example.org/accounts/?limit=100`
/// - `http://api.example.org/accounts/?offset=400&limit=100`
#[derive(Debug, Clone)]
pub struct LimitOffsetPagination {
    /// Default limit (page size)
    pub default_limit: usize,
    /// Query parameter name for limit
    pub limit_query_param: String,
    /// Query parameter name for offset
    pub offset_query_param: String,
    /// Maximum allowed limit
    pub max_limit: Option<usize>,
}

impl Default for LimitOffsetPagination {
    fn default() -> Self {
        Self {
            default_limit: 10,
            limit_query_param: "limit".to_string(),
            offset_query_param: "offset".to_string(),
            max_limit: None,
        }
    }
}

impl LimitOffsetPagination {
    /// Creates a new LimitOffsetPagination with default settings
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::LimitOffsetPagination;
    ///
    /// let paginator = LimitOffsetPagination::new();
    /// assert_eq!(paginator.default_limit, 10);
    /// assert_eq!(paginator.limit_query_param, "limit");
    /// assert_eq!(paginator.offset_query_param, "offset");
    /// ```
    pub fn new() -> Self {
        Self::default()
    }
    /// Sets the default limit (page size) for pagination
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::LimitOffsetPagination;
    ///
    /// let paginator = LimitOffsetPagination::new().default_limit(25);
    /// assert_eq!(paginator.default_limit, 25);
    /// ```
    pub fn default_limit(mut self, limit: usize) -> Self {
        self.default_limit = limit;
        self
    }
    /// Sets the maximum allowed limit
    ///
    /// # Examples
    ///
    /// ```
    /// use reinhardt_pagination::LimitOffsetPagination;
    ///
    /// let paginator = LimitOffsetPagination::new()
    ///     .default_limit(10)
    ///     .max_limit(100);
    /// assert_eq!(paginator.max_limit, Some(100));
    /// ```
    pub fn max_limit(mut self, limit: usize) -> Self {
        self.max_limit = Some(limit);
        self
    }

    fn parse_positive_int(value: &str) -> Result<usize> {
        value
            .parse::<usize>()
            .map_err(|_| Error::Validation(format!("Invalid number: {}", value)))
    }

    /// Parse limit and offset from URL query parameters
    fn parse_params(&self, params: &str, _base_url: &str) -> Result<(usize, usize)> {
        // Try to parse as URL first, otherwise treat as query string
        let query_string = if params.starts_with("http") || params.starts_with('/') {
            if let Ok(url) = url::Url::parse(params)
                .or_else(|_| url::Url::parse(&format!("http://localhost{}", params)))
            {
                url.query().unwrap_or("").to_string()
            } else {
                params.to_string()
            }
        } else {
            params.to_string()
        };

        let mut limit = self.default_limit;
        let mut offset = 0;

        // Parse query parameters
        for pair in query_string.split('&') {
            let parts: Vec<&str> = pair.split('=').collect();
            if parts.len() == 2 {
                let key = parts[0];
                let value = parts[1];

                if key == self.limit_query_param {
                    limit = Self::parse_positive_int(value)?;
                    // Apply max_limit if configured
                    if let Some(max) = self.max_limit {
                        if limit > max {
                            return Err(Error::InvalidLimit(format!(
                                "Limit {} exceeds maximum {}",
                                limit, max
                            )));
                        }
                    }
                } else if key == self.offset_query_param {
                    offset = Self::parse_positive_int(value)?;
                }
            }
        }

        Ok((limit, offset))
    }

    fn build_url(&self, base_url: &str, offset: usize, limit: usize) -> String {
        let url = url::Url::parse(base_url)
            .unwrap_or_else(|_| url::Url::parse(&format!("http://localhost{}", base_url)).unwrap());

        let mut new_url = url.clone();
        new_url
            .query_pairs_mut()
            .clear()
            .append_pair(&self.offset_query_param, &offset.to_string())
            .append_pair(&self.limit_query_param, &limit.to_string());

        // Copy other query parameters
        for (key, value) in url.query_pairs() {
            if key != self.offset_query_param && key != self.limit_query_param {
                new_url.query_pairs_mut().append_pair(&key, &value);
            }
        }

        new_url.to_string()
    }
}

#[async_trait]
impl Paginator for LimitOffsetPagination {
    fn paginate<T: Clone + Send + Sync>(
        &self,
        items: &[T],
        params: Option<&str>,
        base_url: &str,
    ) -> Result<PaginatedResponse<T>> {
        // Parse query parameters from URL or params string
        let (limit, offset) = if let Some(param_str) = params {
            self.parse_params(param_str, base_url)?
        } else {
            (self.default_limit, 0)
        };

        let total_count = items.len();

        // Validate offset
        if offset > total_count {
            return Ok(PaginatedResponse {
                count: total_count,
                next: None,
                previous: None,
                results: vec![],
            });
        }

        // Calculate slice bounds
        let start = offset;
        let end = std::cmp::min(start + limit, total_count);

        // Get results
        let results = items[start..end].to_vec();

        // Build next/previous links
        let next = if end < total_count {
            Some(self.build_url(base_url, offset + limit, limit))
        } else {
            None
        };

        let previous = if offset > 0 {
            let prev_offset = if offset >= limit { offset - limit } else { 0 };
            Some(self.build_url(base_url, prev_offset, limit))
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
        vec![
            SchemaParameter {
                name: self.limit_query_param.clone(),
                required: false,
                location: "query".to_string(),
                description: "Number of results to return per page.".to_string(),
                schema_type: "integer".to_string(),
            },
            SchemaParameter {
                name: self.offset_query_param.clone(),
                required: false,
                location: "query".to_string(),
                description: "The initial index from which to return the results.".to_string(),
                schema_type: "integer".to_string(),
            },
        ]
    }
}

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

#[async_trait]
impl AsyncPaginator for LimitOffsetPagination {
    async fn apaginate<T: Clone + Send + Sync>(
        &self,
        items: &[T],
        params: Option<&str>,
        base_url: &str,
    ) -> Result<PaginatedResponse<T>> {
        // For in-memory operations, just call the sync version
        self.paginate(items, params, base_url)
    }

    fn get_schema_parameters(&self) -> Vec<SchemaParameter> {
        Paginator::get_schema_parameters(self)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // PageNumberPagination Tests
    // ========================================

    #[test]
    fn test_page_number_pagination_first_page() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let page = paginator
            .paginate(&items, Some("1"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results.len(), 10);
        assert_eq!(page.results[0], 1);
        assert_eq!(page.count, 25);
        assert!(page.next.is_some());
        assert!(page.previous.is_none());
    }

    #[test]
    fn test_page_number_pagination_second_page() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let page = paginator
            .paginate(&items, Some("2"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results.len(), 10);
        assert_eq!(page.results[0], 11);
        assert!(page.next.is_some());
        assert!(page.previous.is_some());
    }

    #[test]
    fn test_page_number_pagination_last_page() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let page = paginator
            .paginate(&items, Some("3"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results.len(), 5);
        assert_eq!(page.results[0], 21);
        assert!(page.next.is_none());
        assert!(page.previous.is_some());
    }

    #[test]
    fn test_page_number_pagination_last_keyword() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let page = paginator
            .paginate(&items, Some("last"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results.len(), 5);
        assert_eq!(page.results[0], 21);
        assert!(page.next.is_none());
        assert!(page.previous.is_some());
    }

    #[test]
    fn test_page_number_pagination_no_page_param() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = PageNumberPagination::new().page_size(5);

        let page = paginator
            .paginate(&items, None, "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results, vec![1, 2, 3, 4, 5]);
        assert!(page.next.is_some());
        assert!(page.previous.is_none());
    }

    #[test]
    fn test_page_number_pagination_invalid_page() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let result = paginator.paginate(&items, Some("invalid"), "http://api.example.com/items");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidPage(_)));
    }

    #[test]
    fn test_page_number_pagination_zero_page() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let result = paginator.paginate(&items, Some("0"), "http://api.example.com/items");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidPage(_)));
    }

    #[test]
    fn test_page_number_pagination_out_of_range() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let result = paginator.paginate(&items, Some("10"), "http://api.example.com/items");
        assert!(result.is_err());
    }

    #[test]
    fn test_page_number_pagination_empty_list() {
        let items: Vec<i32> = vec![];
        let paginator = PageNumberPagination::new().page_size(10);

        let page = paginator
            .paginate(&items, Some("1"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results.len(), 0);
        assert_eq!(page.count, 0);
        assert!(page.next.is_none());
        assert!(page.previous.is_none());
    }

    #[test]
    fn test_page_number_pagination_single_item() {
        let items: Vec<i32> = vec![1];
        let paginator = PageNumberPagination::new().page_size(10);

        let page = paginator
            .paginate(&items, Some("1"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results, vec![1]);
        assert_eq!(page.count, 1);
        assert!(page.next.is_none());
        assert!(page.previous.is_none());
    }

    #[test]
    fn test_page_number_pagination_items_equal_to_page_size() {
        let items: Vec<i32> = (1..=10).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let page = paginator
            .paginate(&items, Some("1"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results.len(), 10);
        assert!(page.next.is_none());
        assert!(page.previous.is_none());
    }

    #[test]
    fn test_page_number_pagination_items_one_more_than_page_size() {
        let items: Vec<i32> = (1..=11).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let page1 = paginator
            .paginate(&items, Some("1"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page1.results.len(), 10);
        assert!(page1.next.is_some());

        let page2 = paginator
            .paginate(&items, Some("2"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page2.results.len(), 1);
        assert!(page2.next.is_none());
    }

    // ========================================
    // LimitOffsetPagination Tests
    // ========================================

    #[test]
    fn test_limit_offset_pagination_no_params() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10);

        let page = paginator
            .paginate(&items, None, "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results.len(), 10);
        assert_eq!(page.results[0], 1);
        assert_eq!(page.count, 25);
        assert!(page.next.is_some());
        assert!(page.previous.is_none());
    }

    #[test]
    fn test_limit_offset_pagination_with_offset() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10);

        let page = paginator
            .paginate(
                &items,
                Some("offset=10&limit=10"),
                "http://api.example.com/items",
            )
            .unwrap();
        assert_eq!(page.results.len(), 10);
        assert_eq!(page.results[0], 11);
        assert!(page.next.is_some());
        assert!(page.previous.is_some());
    }

    #[test]
    fn test_limit_offset_pagination_ending_offset() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10);

        let page = paginator
            .paginate(
                &items,
                Some("offset=20&limit=10"),
                "http://api.example.com/items",
            )
            .unwrap();
        assert_eq!(page.results.len(), 5);
        assert_eq!(page.results[0], 21);
        assert!(page.next.is_none());
        assert!(page.previous.is_some());
    }

    #[test]
    fn test_limit_offset_pagination_offset_beyond_count() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10);

        let page = paginator
            .paginate(
                &items,
                Some("offset=100&limit=10"),
                "http://api.example.com/items",
            )
            .unwrap();
        assert_eq!(page.results.len(), 0);
        assert!(page.next.is_none());
        assert!(page.previous.is_none());
    }

    #[test]
    fn test_limit_offset_pagination_invalid_limit() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10);

        let result = paginator.paginate(
            &items,
            Some("limit=invalid"),
            "http://api.example.com/items",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_limit_offset_pagination_invalid_offset() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10);

        let result = paginator.paginate(
            &items,
            Some("offset=invalid"),
            "http://api.example.com/items",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_limit_offset_pagination_max_limit() {
        let items: Vec<i32> = (1..=100).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10).max_limit(20);

        let result = paginator.paginate(&items, Some("limit=50"), "http://api.example.com/items");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidLimit(_)));
    }

    #[test]
    fn test_limit_offset_pagination_within_max_limit() {
        let items: Vec<i32> = (1..=100).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10).max_limit(20);

        let page = paginator
            .paginate(&items, Some("limit=15"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results.len(), 15);
    }

    // ========================================
    // CursorPagination Tests
    // ========================================

    #[test]
    fn test_cursor_pagination_first_page() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = CursorPagination::new().page_size(10);

        let page = paginator
            .paginate(&items, None, "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results.len(), 10);
        assert_eq!(page.results[0], 1);
        assert!(page.next.is_some());
        assert!(page.previous.is_none());
    }

    #[test]
    fn test_cursor_pagination_navigation() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = CursorPagination::new().page_size(10);

        // First page
        let page1 = paginator
            .paginate(&items, None, "http://api.example.com/items")
            .unwrap();
        assert!(page1.next.is_some());

        // Extract cursor from next URL
        let next_url = page1.next.unwrap();
        let url = url::Url::parse(&next_url).unwrap();
        let cursor = url
            .query_pairs()
            .find(|(key, _)| key == "cursor")
            .map(|(_, value)| value.to_string())
            .unwrap();

        // Second page using cursor
        let page2 = paginator
            .paginate(&items, Some(&cursor), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page2.results[0], 11);
        assert!(page2.next.is_some());
        assert!(page2.previous.is_some());
    }

    #[test]
    fn test_cursor_pagination_invalid_cursor() {
        let items: Vec<i32> = (1..=25).collect();
        let paginator = CursorPagination::new().page_size(10);

        let result = paginator.paginate(
            &items,
            Some("invalid_cursor"),
            "http://api.example.com/items",
        );
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidPage(_)));
    }

    #[test]
    fn test_cursor_pagination_empty_list() {
        let items: Vec<i32> = vec![];
        let paginator = CursorPagination::new().page_size(10);

        let page = paginator
            .paginate(&items, None, "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results.len(), 0);
        assert!(page.next.is_none());
        assert!(page.previous.is_none());
    }

    // ========================================
    // Schema and Configuration Tests
    // ========================================

    #[test]
    fn test_page_number_pagination_schema_parameters() {
        let paginator = PageNumberPagination::new().page_size_query_param("page_size");

        let params = Paginator::get_schema_parameters(&paginator);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "page");
        assert_eq!(params[1].name, "page_size");
    }

    #[test]
    fn test_limit_offset_pagination_schema_parameters() {
        let paginator = LimitOffsetPagination::new();

        let params = Paginator::get_schema_parameters(&paginator);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "limit");
        assert_eq!(params[1].name, "offset");
    }

    #[test]
    fn test_cursor_pagination_schema_parameters() {
        let paginator = CursorPagination::new();

        let params = Paginator::get_schema_parameters(&paginator);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "cursor");
        assert_eq!(params[1].name, "page_size");
    }

    // ========================================
    // Additional Edge Cases
    // ========================================

    #[test]
    fn test_paginated_response_new() {
        let metadata = PaginationMetadata {
            count: 100,
            next: Some("http://example.com/next".to_string()),
            previous: None,
        };
        let results = vec![1, 2, 3];

        let response = PaginatedResponse::new(results.clone(), metadata);
        assert_eq!(response.count, 100);
        assert_eq!(response.results, results);
        assert!(response.next.is_some());
        assert!(response.previous.is_none());
    }

    #[test]
    fn test_page_number_pagination_builder_pattern() {
        let paginator = PageNumberPagination::new()
            .page_size(20)
            .max_page_size(100)
            .page_size_query_param("size");

        assert_eq!(paginator.page_size, 20);
        assert_eq!(paginator.max_page_size, Some(100));
        assert_eq!(paginator.page_size_query_param, Some("size".to_string()));
    }

    #[test]
    fn test_limit_offset_pagination_builder_pattern() {
        let paginator = LimitOffsetPagination::new()
            .default_limit(30)
            .max_limit(200);

        assert_eq!(paginator.default_limit, 30);
        assert_eq!(paginator.max_limit, Some(200));
    }

    #[test]
    fn test_cursor_pagination_builder_pattern() {
        let paginator = CursorPagination::new()
            .page_size(15)
            .max_page_size(50)
            .ordering(vec!["created".to_string()]);

        assert_eq!(paginator.page_size, 15);
        assert_eq!(paginator.max_page_size, Some(50));
        assert_eq!(paginator.ordering, vec!["created".to_string()]);
    }

    #[test]
    fn test_cursor_pagination_with_page_size() {
        let items: Vec<i32> = (1..=30).collect();
        let paginator = CursorPagination::new().page_size(5);

        // Request page_size=20 in URL
        let page1 = paginator
            .paginate(&items, None, "http://api.example.com/items?page_size=20")
            .unwrap();
        assert_eq!(page1.results.len(), 20);
        assert_eq!(page1.results, (1..=20).collect::<Vec<i32>>());

        // Navigate to next page - should preserve page_size
        let next_url = page1.next.unwrap();
        assert!(next_url.contains("page_size=20"));
    }

    #[test]
    fn test_cursor_pagination_with_page_size_over_limit() {
        let items: Vec<i32> = (1..=30).collect();
        let paginator = CursorPagination::new().page_size(5).max_page_size(20);

        // Request page_size=30, should be clamped to max_page_size=20
        let page1 = paginator
            .paginate(&items, None, "http://api.example.com/items?page_size=30")
            .unwrap();
        assert_eq!(page1.results.len(), 20);
        assert_eq!(page1.results, (1..=20).collect::<Vec<i32>>());
    }

    #[test]
    fn test_cursor_pagination_with_page_size_zero() {
        let items: Vec<i32> = (1..=30).collect();
        let paginator = CursorPagination::new().page_size(5);

        // page_size=0 should fall back to default page_size=5
        let page1 = paginator
            .paginate(&items, None, "http://api.example.com/items?page_size=0")
            .unwrap();
        assert_eq!(page1.results.len(), 5);
        assert_eq!(page1.results, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_cursor_pagination_with_page_size_invalid() {
        let items: Vec<i32> = (1..=30).collect();
        let paginator = CursorPagination::new().page_size(5);

        // Invalid page_size should fall back to default
        let page1 = paginator
            .paginate(&items, None, "http://api.example.com/items?page_size=abc")
            .unwrap();
        assert_eq!(page1.results.len(), 5);
        assert_eq!(page1.results, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_cursor_pagination_page_size_in_schema() {
        let paginator = CursorPagination::new();
        let params = Paginator::get_schema_parameters(&paginator);

        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "cursor");
        assert_eq!(params[1].name, "page_size");
        assert_eq!(params[1].schema_type, "integer");
    }

    // ========================================
    // Page Structure Tests
    // ========================================

    #[test]
    fn test_page_indexes() {
        // Test with full page
        let page = Page::new(vec![1, 2, 3, 4, 5], 1, 3, 15, 5);
        assert_eq!(page.start_index(), 1);
        assert_eq!(page.end_index(), 5);

        // Test with partial page
        let page2 = Page::new(vec![11, 12, 13, 14, 15], 3, 3, 15, 5);
        assert_eq!(page2.start_index(), 11);
        assert_eq!(page2.end_index(), 15);

        // Test with empty page
        let empty_page: Page<i32> = Page::new(vec![], 1, 1, 0, 5);
        assert_eq!(empty_page.start_index(), 0);
        assert_eq!(empty_page.end_index(), 0);
    }

    #[test]
    fn test_page_sequence() {
        let items: Vec<char> = "abcdefghijk".chars().collect();
        let page = Page::new(items[5..11].to_vec(), 2, 2, 11, 5);

        assert_eq!(page.len(), 6);
        assert!(page.get(0).is_some());
        assert_eq!(*page.get(0).unwrap(), 'f');

        // Test iteration
        let collected: Vec<char> = page.clone().into_iter().collect();
        assert_eq!(collected, vec!['f', 'g', 'h', 'i', 'j', 'k']);

        // Test reverse iteration
        let reversed: Vec<char> = collected.into_iter().rev().collect();
        assert_eq!(reversed, vec!['k', 'j', 'i', 'h', 'g', 'f']);
    }

    #[test]
    fn test_page_has_next_previous() {
        // First page
        let page1 = Page::new(vec![1, 2, 3], 1, 3, 9, 3);
        assert!(!page1.has_previous());
        assert!(page1.has_next());
        assert!(page1.has_other_pages());
        assert_eq!(page1.next_page_number().unwrap(), 2);
        assert!(page1.previous_page_number().is_err());

        // Middle page
        let page2 = Page::new(vec![4, 5, 6], 2, 3, 9, 3);
        assert!(page2.has_previous());
        assert!(page2.has_next());
        assert!(page2.has_other_pages());
        assert_eq!(page2.next_page_number().unwrap(), 3);
        assert_eq!(page2.previous_page_number().unwrap(), 1);

        // Last page
        let page3 = Page::new(vec![7, 8, 9], 3, 3, 9, 3);
        assert!(page3.has_previous());
        assert!(!page3.has_next());
        assert!(page3.has_other_pages());
        assert!(page3.next_page_number().is_err());
        assert_eq!(page3.previous_page_number().unwrap(), 2);

        // Single page
        let single_page = Page::new(vec![1, 2, 3], 1, 1, 3, 10);
        assert!(!single_page.has_previous());
        assert!(!single_page.has_next());
        assert!(!single_page.has_other_pages());
    }

    #[test]
    fn test_page_indexing() {
        let page = Page::new(vec![10, 20, 30, 40, 50], 1, 1, 5, 5);

        // Test Index trait
        assert_eq!(page[0], 10);
        assert_eq!(page[2], 30);
        assert_eq!(page[4], 50);

        // Test get method
        assert_eq!(page.get(1), Some(&20));
        assert_eq!(page.get(10), None);

        // Test get_slice method
        assert_eq!(page.get_slice(1..3), Some(&[20, 30][..]));
        assert_eq!(page.get_slice(10..20), None);
    }

    #[test]
    fn test_page_empty() {
        let empty_page: Page<i32> = Page::new(vec![], 1, 0, 0, 10);
        assert!(empty_page.is_empty());
        assert_eq!(empty_page.len(), 0);

        let non_empty_page = Page::new(vec![1], 1, 1, 1, 10);
        assert!(!non_empty_page.is_empty());
        assert_eq!(non_empty_page.len(), 1);
    }

    // ========================================
    // Orphans and allow_empty_first_page Tests
    // ========================================

    #[test]
    fn test_orphans_merge_last_page() {
        let items: Vec<i32> = (1..=11).collect(); // 11 items
        let paginator = PageNumberPagination::new().page_size(10).orphans(2); // Merge if last page has <= 2 items

        // With orphans=2, last page with 1 item should merge with page 1
        let page1 = paginator
            .paginate(&items, Some("1"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page1.results.len(), 11); // All items on one page
        assert_eq!(page1.count, 11);
        assert!(page1.next.is_none()); // No next page

        // Trying to access page 2 should fail
        let result = paginator.paginate(&items, Some("2"), "http://api.example.com/items");
        assert!(result.is_err());
    }

    #[test]
    fn test_orphans_no_merge() {
        let items: Vec<i32> = (1..=13).collect(); // 13 items
        let paginator = PageNumberPagination::new().page_size(10).orphans(2); // Merge if last page has <= 2 items

        // With orphans=2, last page with 3 items should NOT merge
        let page1 = paginator
            .paginate(&items, Some("1"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page1.results.len(), 10);
        assert!(page1.next.is_some());

        let page2 = paginator
            .paginate(&items, Some("2"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page2.results.len(), 3);
        assert!(page2.next.is_none());
    }

    #[test]
    fn test_allow_empty_first_page_true() {
        let items: Vec<i32> = vec![];
        let paginator = PageNumberPagination::new()
            .page_size(10)
            .allow_empty_first_page(true);

        let page = paginator
            .paginate(&items, Some("1"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page.results.len(), 0);
        assert_eq!(page.count, 0);
    }

    #[test]
    fn test_allow_empty_first_page_false() {
        let items: Vec<i32> = vec![];
        let paginator = PageNumberPagination::new()
            .page_size(10)
            .allow_empty_first_page(false);

        let result = paginator.paginate(&items, Some("1"), "http://api.example.com/items");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidPage(_)));
    }

    #[test]
    fn test_orphans_with_various_counts() {
        // Test case from Django: 10 items, page_size=4, orphans=2
        let items: Vec<i32> = (1..=10).collect();

        // orphans=0: should have 3 pages (4+4+2)
        let paginator0 = PageNumberPagination::new().page_size(4).orphans(0);
        let page1 = paginator0
            .paginate(&items, Some("1"), "http://api.example.com/items")
            .unwrap();
        assert!(page1.next.is_some());

        // orphans=1: should have 3 pages (4+4+2, because 2 > 1)
        let paginator1 = PageNumberPagination::new().page_size(4).orphans(1);
        let page1 = paginator1
            .paginate(&items, Some("1"), "http://api.example.com/items")
            .unwrap();
        assert!(page1.next.is_some());

        // orphans=2: should have 2 pages (4+6, because 2 <= 2)
        let paginator2 = PageNumberPagination::new().page_size(4).orphans(2);
        let page1 = paginator2
            .paginate(&items, Some("1"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page1.results.len(), 4);
        let page2 = paginator2
            .paginate(&items, Some("2"), "http://api.example.com/items")
            .unwrap();
        assert_eq!(page2.results.len(), 6); // 4 + 2 orphans
        assert!(page2.next.is_none());
    }

    #[test]
    fn test_error_messages_custom() {
        let items: Vec<i32> = (1..=3).collect();
        let custom_messages = ErrorMessages {
            invalid_page: "Wrong page number".to_string(),
            min_page: "Too small".to_string(),
            no_results: "There is nothing here".to_string(),
        };
        let paginator = PageNumberPagination::new()
            .page_size(2)
            .error_messages(custom_messages);

        // Test invalid page (non-numeric)
        let result = paginator.paginate(&items, Some("abc"), "http://api.example.com/items");
        assert!(result.is_err());
        if let Err(Error::InvalidPage(msg)) = result {
            assert_eq!(msg, "Wrong page number");
        } else {
            panic!("Expected InvalidPage error");
        }

        // Test min page (page 0)
        let result = paginator.paginate(&items, Some("0"), "http://api.example.com/items");
        assert!(result.is_err());
        if let Err(Error::InvalidPage(msg)) = result {
            assert_eq!(msg, "Too small");
        } else {
            panic!("Expected InvalidPage error");
        }

        // Test no results (page beyond range)
        let result = paginator.paginate(&items, Some("10"), "http://api.example.com/items");
        assert!(result.is_err());
        if let Err(Error::InvalidPage(msg)) = result {
            assert_eq!(msg, "There is nothing here");
        } else {
            panic!("Expected InvalidPage error");
        }
    }

    #[test]
    fn test_error_messages_default() {
        let items: Vec<i32> = (1..=3).collect();
        let paginator = PageNumberPagination::new().page_size(2);

        // Test default error message for out of range
        let result = paginator.paginate(&items, Some("10"), "http://api.example.com/items");
        assert!(result.is_err());
        if let Err(Error::InvalidPage(msg)) = result {
            assert_eq!(msg, "That page contains no results");
        } else {
            panic!("Expected InvalidPage error");
        }

        // Test default error message for page 0
        let result = paginator.paginate(&items, Some("0"), "http://api.example.com/items");
        assert!(result.is_err());
        if let Err(Error::InvalidPage(msg)) = result {
            assert_eq!(msg, "That page number is less than 1");
        } else {
            panic!("Expected InvalidPage error");
        }
    }

    #[test]
    fn test_error_messages_partial_custom() {
        let items: Vec<i32> = (1..=3).collect();
        let custom_messages = ErrorMessages {
            min_page: "Too small".to_string(),
            ..ErrorMessages::default()
        };
        let paginator = PageNumberPagination::new()
            .page_size(2)
            .error_messages(custom_messages);

        // Custom message for min_page
        let result = paginator.paginate(&items, Some("0"), "http://api.example.com/items");
        assert!(result.is_err());
        if let Err(Error::InvalidPage(msg)) = result {
            assert_eq!(msg, "Too small");
        } else {
            panic!("Expected InvalidPage error");
        }

        // Default message for no_results
        let result = paginator.paginate(&items, Some("10"), "http://api.example.com/items");
        assert!(result.is_err());
        if let Err(Error::InvalidPage(msg)) = result {
            assert_eq!(msg, "That page contains no results");
        } else {
            panic!("Expected InvalidPage error");
        }
    }

    #[test]
    fn test_get_page_valid() {
        let items: Vec<i32> = (1..=10).collect();
        let paginator = PageNumberPagination::new().page_size(3);

        let page1 = paginator.get_page(&items, Some("1"));
        assert_eq!(page1.number, 1);
        assert_eq!(page1.object_list, vec![1, 2, 3]);
        assert_eq!(page1.num_pages, 4);
        assert_eq!(page1.count, 10);

        let page2 = paginator.get_page(&items, Some("2"));
        assert_eq!(page2.number, 2);
        assert_eq!(page2.object_list, vec![4, 5, 6]);
    }

    #[test]
    fn test_get_page_out_of_range_returns_last_page() {
        let items: Vec<i32> = (1..=10).collect();
        let paginator = PageNumberPagination::new().page_size(3);

        // Out of range returns last page (page 4)
        let page = paginator.get_page(&items, Some("100"));
        assert_eq!(page.number, 4);
        assert_eq!(page.object_list, vec![10]);
    }

    #[test]
    fn test_get_page_invalid_returns_first_page() {
        let items: Vec<i32> = (1..=10).collect();
        let paginator = PageNumberPagination::new().page_size(3);

        // Invalid page number returns first page
        let page = paginator.get_page(&items, Some("abc"));
        assert_eq!(page.number, 1);
        assert_eq!(page.object_list, vec![1, 2, 3]);

        // None returns first page
        let page = paginator.get_page(&items, None);
        assert_eq!(page.number, 1);
        assert_eq!(page.object_list, vec![1, 2, 3]);
    }

    #[test]
    fn test_get_page_empty_list() {
        let items: Vec<i32> = vec![];
        let paginator = PageNumberPagination::new().page_size(3);

        let page = paginator.get_page(&items, Some("1"));
        assert_eq!(page.number, 1);
        assert_eq!(page.object_list, Vec::<i32>::new());
        assert_eq!(page.num_pages, 1);
        assert_eq!(page.count, 0);
    }

    #[test]
    fn test_get_page_with_orphans() {
        let items: Vec<i32> = (1..=11).collect();
        let paginator = PageNumberPagination::new().page_size(10).orphans(2);

        // With orphans=2, should have only 1 page
        let page = paginator.get_page(&items, Some("1"));
        assert_eq!(page.number, 1);
        assert_eq!(page.object_list.len(), 11);
        assert_eq!(page.num_pages, 1);

        // Requesting page 2 should return page 1 (last available page)
        let page = paginator.get_page(&items, Some("2"));
        assert_eq!(page.number, 1);
        assert_eq!(page.object_list.len(), 11);
    }

    #[test]
    fn test_float_integer_page() {
        let items: Vec<i32> = (1..=10).collect();
        let paginator = PageNumberPagination::new().page_size(3);

        // Float that represents an integer should work
        let page = paginator.paginate(&items, Some("1.0"), "http://api.example.com/items");
        assert!(page.is_ok());
        let page = page.unwrap();
        assert_eq!(page.results, vec![1, 2, 3]);

        let page = paginator.paginate(&items, Some("2.0"), "http://api.example.com/items");
        assert!(page.is_ok());
        let page = page.unwrap();
        assert_eq!(page.results, vec![4, 5, 6]);
    }

    #[test]
    fn test_float_non_integer_page_fails() {
        let items: Vec<i32> = (1..=10).collect();
        let paginator = PageNumberPagination::new().page_size(3);

        // Float with fractional part should fail
        let result = paginator.paginate(&items, Some("1.5"), "http://api.example.com/items");
        assert!(result.is_err());

        let result = paginator.paginate(&items, Some("2.3"), "http://api.example.com/items");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_page_with_float() {
        let items: Vec<i32> = (1..=10).collect();
        let paginator = PageNumberPagination::new().page_size(3);

        // get_page accepts float integers but returns first page on non-integer floats
        let page = paginator.get_page(&items, Some("1.0"));
        assert_eq!(page.number, 1);
        assert_eq!(page.object_list, vec![1, 2, 3]);

        // Non-integer floats return first page (graceful fallback)
        let page = paginator.get_page(&items, Some("1.5"));
        assert_eq!(page.number, 1);
        assert_eq!(page.object_list, vec![1, 2, 3]);
    }

    #[test]
    fn test_page_range() {
        let items: Vec<i32> = (1..=10).collect();
        let paginator = PageNumberPagination::new().page_size(3);

        let page = paginator.get_page(&items, Some("1"));
        let range: Vec<usize> = page.page_range().collect();
        assert_eq!(range, vec![1, 2, 3, 4]);

        // Works with any page
        let page = paginator.get_page(&items, Some("2"));
        let range: Vec<usize> = page.page_range().collect();
        assert_eq!(range, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_page_range_single_page() {
        let items: Vec<i32> = (1..=5).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let page = paginator.get_page(&items, Some("1"));
        let range: Vec<usize> = page.page_range().collect();
        assert_eq!(range, vec![1]);
    }

    #[test]
    fn test_page_range_empty() {
        let items: Vec<i32> = vec![];
        let paginator = PageNumberPagination::new().page_size(10);

        let page = paginator.get_page(&items, Some("1"));
        let range: Vec<usize> = page.page_range().collect();
        assert_eq!(range, vec![1]);
    }

    #[test]
    fn test_elided_page_range_not_elided() {
        // Test when range is not elided (10 pages or less with default settings)
        let items: Vec<i32> = (1..=100).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let page = paginator.get_page(&items, Some("1"));
        let range = page.get_elided_page_range(3, 2);
        let expected: Vec<Option<usize>> = (1..=10).map(Some).collect();
        assert_eq!(range, expected);
    }

    #[test]
    fn test_elided_page_range_with_ellipsis() {
        // Test Django's example: 50 pages, on_each_side=3, on_ends=2
        let items: Vec<i32> = (1..=5000).collect();
        let paginator = PageNumberPagination::new().page_size(100);

        // Page 1: [1, 2, 3, 4, ..., 49, 50]
        let page1 = paginator.get_page(&items, Some("1"));
        let range1 = page1.get_elided_page_range(3, 2);
        assert_eq!(
            range1,
            vec![
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                None, // Ellipsis
                Some(49),
                Some(50)
            ]
        );

        // Page 8: [1, 2, ..., 5, 6, 7, 8, 9, 10, 11, ..., 49, 50]
        let page8 = paginator.get_page(&items, Some("8"));
        let range8 = page8.get_elided_page_range(3, 2);
        assert_eq!(
            range8,
            vec![
                Some(1),
                Some(2),
                None, // Ellipsis
                Some(5),
                Some(6),
                Some(7),
                Some(8),
                Some(9),
                Some(10),
                Some(11),
                None, // Ellipsis
                Some(49),
                Some(50)
            ]
        );

        // Page 50: [1, 2, ..., 47, 48, 49, 50]
        let page50 = paginator.get_page(&items, Some("50"));
        let range50 = page50.get_elided_page_range(3, 2);
        assert_eq!(
            range50,
            vec![
                Some(1),
                Some(2),
                None, // Ellipsis
                Some(47),
                Some(48),
                Some(49),
                Some(50)
            ]
        );
    }

    #[test]
    fn test_elided_page_range_custom_params() {
        // Test with custom on_each_side and on_ends
        let items: Vec<i32> = (1..=3000).collect();
        let paginator = PageNumberPagination::new().page_size(100);

        let page15 = paginator.get_page(&items, Some("15"));

        // on_each_side=1, on_ends=1: [1, ..., 14, 15, 16, ..., 30]
        let range = page15.get_elided_page_range(1, 1);
        assert_eq!(
            range,
            vec![
                Some(1),
                None, // Ellipsis
                Some(14),
                Some(15),
                Some(16),
                None, // Ellipsis
                Some(30)
            ]
        );
    }

    // ========================================
    // Async Tests - PageNumberPagination
    // ========================================

    #[tokio::test]
    async fn test_page_number_pagination_apaginate() {
        let items: Vec<i32> = (1..=100).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        // Async pagination
        let page1 = paginator
            .apaginate(&items, Some("1"), "http://api.example.com/items")
            .await
            .unwrap();

        assert_eq!(page1.results.len(), 10);
        assert_eq!(page1.results, (1..=10).collect::<Vec<i32>>());
        assert!(page1.next.is_some());
        assert!(page1.previous.is_none());
    }

    #[tokio::test]
    async fn test_page_number_pagination_aget_page() {
        let items: Vec<i32> = (1..=100).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        // Async get_page
        let page = paginator.aget_page(&items, Some("5")).await;

        assert_eq!(page.number, 5);
        assert_eq!(page.object_list.len(), 10);
        assert_eq!(page.object_list, (41..=50).collect::<Vec<i32>>());
    }

    #[tokio::test]
    async fn test_page_number_pagination_aget_page_empty() {
        let items: Vec<i32> = vec![];
        let paginator = PageNumberPagination::new().page_size(10);

        let page = paginator.aget_page(&items, Some("1")).await;

        assert_eq!(page.number, 1);
        assert_eq!(page.object_list.len(), 0);
        assert_eq!(page.count, 0);
    }

    #[tokio::test]
    async fn test_page_number_pagination_aget_page_out_of_range() {
        let items: Vec<i32> = (1..=30).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        // Out of range returns last page
        let page = paginator.aget_page(&items, Some("100")).await;

        assert_eq!(page.number, 3); // Last page
        assert_eq!(page.object_list.len(), 10);
    }

    #[tokio::test]
    async fn test_page_number_pagination_aget_page_invalid() {
        let items: Vec<i32> = (1..=30).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        // Invalid returns first page
        let page = paginator.aget_page(&items, Some("abc")).await;

        assert_eq!(page.number, 1);
        assert_eq!(page.object_list, (1..=10).collect::<Vec<i32>>());
    }

    #[tokio::test]
    async fn test_page_number_pagination_async_with_orphans() {
        let items: Vec<i32> = (1..=11).collect();
        let paginator = PageNumberPagination::new().page_size(10).orphans(2);

        let page = paginator.aget_page(&items, Some("1")).await;

        assert_eq!(page.object_list.len(), 11); // All items on one page
        assert_eq!(page.num_pages, 1);
    }

    #[tokio::test]
    async fn test_page_number_pagination_async_with_custom_errors() {
        let items: Vec<i32> = (1..=30).collect();
        let custom_messages = ErrorMessages {
            invalid_page: "Custom invalid".to_string(),
            min_page: "Custom min".to_string(),
            no_results: "Custom no results".to_string(),
        };
        let paginator = PageNumberPagination::new()
            .page_size(10)
            .error_messages(custom_messages);

        let result = paginator
            .apaginate(&items, Some("100"), "http://api.example.com/items")
            .await;

        assert!(result.is_err());
        if let Err(Error::InvalidPage(msg)) = result {
            assert_eq!(msg, "Custom no results");
        }
    }

    #[tokio::test]
    async fn test_page_number_pagination_async_iteration() {
        let items: Vec<i32> = (1..=30).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let page = paginator.aget_page(&items, Some("2")).await;

        // Test iteration
        let collected: Vec<i32> = page.into_iter().collect();
        assert_eq!(collected, (11..=20).collect::<Vec<i32>>());
    }

    #[tokio::test]
    async fn test_page_number_pagination_async_elided_range() {
        let items: Vec<i32> = (1..=5000).collect();
        let paginator = PageNumberPagination::new().page_size(100);

        let page = paginator.aget_page(&items, Some("8")).await;
        let range = page.get_elided_page_range(3, 2);

        // Should have ellipsis
        assert!(range.contains(&None));
        assert!(range.contains(&Some(8)));
    }

    #[tokio::test]
    async fn test_page_number_pagination_async_float_page() {
        let items: Vec<i32> = (1..=30).collect();
        let paginator = PageNumberPagination::new().page_size(10);

        let result = paginator
            .apaginate(&items, Some("2.0"), "http://api.example.com/items")
            .await;

        assert!(result.is_ok());
        let page = result.unwrap();
        assert_eq!(page.results, (11..=20).collect::<Vec<i32>>());
    }

    // ========================================
    // Async Tests - LimitOffsetPagination
    // ========================================

    #[tokio::test]
    async fn test_limit_offset_pagination_apaginate() {
        let items: Vec<i32> = (1..=100).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10);

        let page = paginator
            .apaginate(&items, None, "http://api.example.com/items")
            .await
            .unwrap();

        assert_eq!(page.results.len(), 10);
        assert_eq!(page.results, (1..=10).collect::<Vec<i32>>());
    }

    #[tokio::test]
    async fn test_limit_offset_pagination_async_with_limit() {
        let items: Vec<i32> = (1..=100).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10);

        let page = paginator
            .apaginate(&items, Some("limit=20"), "http://api.example.com/items")
            .await
            .unwrap();

        assert_eq!(page.results.len(), 20);
        assert_eq!(page.results, (1..=20).collect::<Vec<i32>>());
    }

    #[tokio::test]
    async fn test_limit_offset_pagination_async_with_offset() {
        let items: Vec<i32> = (1..=100).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10);

        let page = paginator
            .apaginate(&items, Some("offset=20"), "http://api.example.com/items")
            .await
            .unwrap();

        assert_eq!(page.results.len(), 10);
        assert_eq!(page.results, (21..=30).collect::<Vec<i32>>());
    }

    #[tokio::test]
    async fn test_limit_offset_pagination_async_max_limit() {
        let items: Vec<i32> = (1..=100).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10).max_limit(50);

        let result = paginator
            .apaginate(&items, Some("limit=100"), "http://api.example.com/items")
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidLimit(_)));
    }

    #[tokio::test]
    async fn test_limit_offset_pagination_async_invalid_params() {
        let items: Vec<i32> = (1..=100).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10);

        let result = paginator
            .apaginate(&items, Some("limit=abc"), "http://api.example.com/items")
            .await;

        assert!(result.is_err()); // Invalid params cause error
    }

    #[tokio::test]
    async fn test_limit_offset_pagination_async_edge_cases() {
        let items: Vec<i32> = (1..=100).collect();
        let paginator = LimitOffsetPagination::new().default_limit(10);

        // Offset beyond count
        let page = paginator
            .apaginate(&items, Some("offset=200"), "http://api.example.com/items")
            .await
            .unwrap();

        assert_eq!(page.results.len(), 0);
        assert!(page.next.is_none());
    }

    // ========================================
    // Async Tests - CursorPagination
    // ========================================

    #[tokio::test]
    async fn test_cursor_pagination_apaginate() {
        let items: Vec<i32> = (1..=30).collect();
        let paginator = CursorPagination::new().page_size(10);

        let page = paginator
            .apaginate(&items, None, "http://api.example.com/items")
            .await
            .unwrap();

        assert_eq!(page.results.len(), 10);
        assert_eq!(page.results, (1..=10).collect::<Vec<i32>>());
        assert!(page.next.is_some());
    }

    #[tokio::test]
    async fn test_cursor_pagination_async_navigation() {
        let items: Vec<i32> = (1..=30).collect();
        let paginator = CursorPagination::new().page_size(10);

        let page1 = paginator
            .apaginate(&items, None, "http://api.example.com/items")
            .await
            .unwrap();

        let next_url = page1.next.unwrap();
        let url = url::Url::parse(&next_url).unwrap();
        let cursor = url
            .query_pairs()
            .find(|(key, _)| key == "cursor")
            .map(|(_, value)| value.to_string())
            .unwrap();

        let page2 = paginator
            .apaginate(&items, Some(&cursor), &next_url)
            .await
            .unwrap();

        assert_eq!(page2.results.len(), 10);
        assert_eq!(page2.results, (11..=20).collect::<Vec<i32>>());
    }

    #[tokio::test]
    async fn test_cursor_pagination_async_with_page_size() {
        let items: Vec<i32> = (1..=30).collect();
        let paginator = CursorPagination::new().page_size(5);

        let page = paginator
            .apaginate(&items, None, "http://api.example.com/items?page_size=20")
            .await
            .unwrap();

        assert_eq!(page.results.len(), 20);
        assert_eq!(page.results, (1..=20).collect::<Vec<i32>>());
    }

    #[tokio::test]
    async fn test_cursor_pagination_async_invalid_cursor() {
        let items: Vec<i32> = (1..=30).collect();
        let paginator = CursorPagination::new().page_size(10);

        let result = paginator
            .apaginate(
                &items,
                Some("invalid_cursor"),
                "http://api.example.com/items",
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cursor_pagination_async_empty_list() {
        let items: Vec<i32> = vec![];
        let paginator = CursorPagination::new().page_size(10);

        let page = paginator
            .apaginate(&items, None, "http://api.example.com/items")
            .await
            .unwrap();

        assert_eq!(page.results.len(), 0);
        assert!(page.next.is_none());
    }

    #[tokio::test]
    async fn test_cursor_pagination_async_edge_cases() {
        let items: Vec<i32> = (1..=10).collect();
        let paginator = CursorPagination::new().page_size(10);

        // Exact page size
        let page = paginator
            .apaginate(&items, None, "http://api.example.com/items")
            .await
            .unwrap();

        assert_eq!(page.results.len(), 10);
        assert!(page.next.is_none()); // No more items
    }
}

// ============================================================================
// Enum Wrapper for dyn Paginator Compatibility
// ============================================================================

/// Enum wrapper for Paginator implementations to enable dyn compatibility
///
/// This wrapper allows using different pagination strategies through a single
/// type, solving the issue that `Paginator` trait with generic methods cannot
/// be used as `dyn Paginator`.
#[derive(Debug, Clone)]
pub enum PaginatorImpl {
    /// Page number based pagination
    PageNumber(PageNumberPagination),
    /// Limit/offset based pagination
    LimitOffset(LimitOffsetPagination),
    /// Cursor based pagination
    Cursor(CursorPagination),
}

impl Paginator for PaginatorImpl {
    fn paginate<T: Clone + Send + Sync>(
        &self,
        items: &[T],
        page_param: Option<&str>,
        base_url: &str,
    ) -> Result<PaginatedResponse<T>> {
        match self {
            Self::PageNumber(p) => p.paginate(items, page_param, base_url),
            Self::LimitOffset(p) => p.paginate(items, page_param, base_url),
            Self::Cursor(p) => p.paginate(items, page_param, base_url),
        }
    }

    fn get_schema_parameters(&self) -> Vec<SchemaParameter> {
        match self {
            Self::PageNumber(p) => Paginator::get_schema_parameters(p),
            Self::LimitOffset(p) => Paginator::get_schema_parameters(p),
            Self::Cursor(p) => Paginator::get_schema_parameters(p),
        }
    }
}

#[async_trait]
impl AsyncPaginator for PaginatorImpl {
    async fn apaginate<T: Clone + Send + Sync>(
        &self,
        items: &[T],
        page_param: Option<&str>,
        base_url: &str,
    ) -> Result<PaginatedResponse<T>> {
        match self {
            Self::PageNumber(p) => p.apaginate(items, page_param, base_url).await,
            Self::LimitOffset(p) => p.apaginate(items, page_param, base_url).await,
            Self::Cursor(p) => p.apaginate(items, page_param, base_url).await,
        }
    }

    fn get_schema_parameters(&self) -> Vec<SchemaParameter> {
        match self {
            Self::PageNumber(p) => AsyncPaginator::get_schema_parameters(p),
            Self::LimitOffset(p) => AsyncPaginator::get_schema_parameters(p),
            Self::Cursor(p) => AsyncPaginator::get_schema_parameters(p),
        }
    }
}

impl PaginatorImpl {
    /// Create a page number pagination instance
    pub fn page_number(pagination: PageNumberPagination) -> Self {
        Self::PageNumber(pagination)
    }

    /// Create a limit/offset pagination instance
    pub fn limit_offset(pagination: LimitOffsetPagination) -> Self {
        Self::LimitOffset(pagination)
    }

    /// Create a cursor pagination instance
    pub fn cursor(pagination: CursorPagination) -> Self {
        Self::Cursor(pagination)
    }
}
