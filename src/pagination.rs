//! Iterator-based pagination utilities.
//!
//! FP-012: iterator-driven pagination with bounded memory usage. The helpers here
//! enable large dataset processing without materialising every element by
//! carefully consuming only the items required for the requested page.

use std::iter::{FusedIterator, Iterator};

/// Pagination input parameters represented as a cursor (zero-based page index)
/// and the desired page size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination {
    cursor: usize,
    page_size: usize,
}

impl Pagination {
    /// Creates a new pagination descriptor. A page size of zero defaults to
    /// `1` to prevent invalid divisions.
    pub fn new(cursor: usize, page_size: usize) -> Self {
        Self {
            cursor,
            page_size: page_size.max(1),
        }
    }

    /// Builds a pagination descriptor from optional parameters and a default
    /// page size. Negative values are clamped to zero.
    pub fn from_optional(
        cursor: Option<i64>,
        page_size: Option<i64>,
        default_page_size: usize,
    ) -> Self {
        let cursor = cursor.unwrap_or(0).max(0) as usize;
        let page_size = page_size
            .map(|size| size.max(1) as usize)
            .unwrap_or(default_page_size.max(1));

        Pagination::new(cursor, page_size)
    }

    /// Returns the zero-based page cursor.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns the configured page size.
    pub fn page_size(&self) -> usize {
        self.page_size
    }

    /// Returns the offset in elements to start reading for this page.
    pub fn offset(&self) -> usize {
        self.cursor.saturating_mul(self.page_size)
    }

    /// Computes the cursor for the next page if there is additional data.
    pub fn next_cursor(&self, has_more: bool) -> Option<usize> {
        if has_more {
            Some(self.cursor + 1)
        } else {
            None
        }
    }

    /// Calculates the total number of pages for a known total count.
    pub fn total_pages(&self, total_count: usize) -> usize {
        if total_count == 0 {
            0
        } else {
            total_count.saturating_add(self.page_size.saturating_sub(1)) / self.page_size
        }
    }
}

/// Pagination metadata emitted alongside a page of results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaginationSummary {
    pub current_cursor: usize,
    pub page_size: usize,
    pub total_elements: Option<usize>,
    pub next_cursor: Option<usize>,
    pub has_more: bool,
}

impl PaginationSummary {
    fn new(pagination: Pagination, has_more: bool, total: Option<usize>) -> Self {
        Self {
            current_cursor: pagination.cursor(),
            page_size: pagination.page_size(),
            total_elements: total,
            next_cursor: pagination.next_cursor(has_more),
            has_more,
        }
    }
}

/// Represents a single page of items along with pagination metadata.
#[derive(Debug, Clone)]
pub struct PaginatedPage<T> {
    pub items: Vec<T>,
    pub summary: PaginationSummary,
}

impl<T> PaginatedPage<T> {
    /// Creates a page from pre-fetched items.
    pub fn from_items(
        items: Vec<T>,
        pagination: Pagination,
        has_more: bool,
        total: Option<usize>,
    ) -> Self {
        Self {
            items,
            summary: PaginationSummary::new(pagination, has_more, total),
        }
    }

    /// Maps the contained items using `f`, preserving pagination metadata.
    pub fn map_items<U, F>(self, mut f: F) -> PaginatedPage<U>
    where
        F: FnMut(T) -> U,
    {
        let PaginatedPage { items, summary } = self;
        PaginatedPage {
            items: items.into_iter().map(&mut f).collect(),
            summary,
        }
    }
}

/// Iterator adaptor that yields fixed-size chunks representing consecutive pages.
pub struct PagedIterator<I>
where
    I: Iterator,
{
    inner: I,
    page_size: usize,
}

impl<I> PagedIterator<I>
where
    I: Iterator,
{
    fn new(inner: I, page_size: usize) -> Self {
        Self {
            inner,
            page_size: page_size.max(1),
        }
    }
}

impl<I> Iterator for PagedIterator<I>
where
    I: Iterator,
{
    type Item = Vec<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk = Vec::with_capacity(self.page_size);

        for _ in 0..self.page_size {
            match self.inner.next() {
                Some(item) => chunk.push(item),
                None => break,
            }
        }

        if chunk.is_empty() {
            None
        } else {
            Some(chunk)
        }
    }
}

impl<I> FusedIterator for PagedIterator<I> where I: FusedIterator {}

/// Iterator extension helpers enabling chunked and cursor-based pagination.
pub trait PaginateExt: Iterator + Sized {
    /// Splits the iterator into `page_size` chunks. The final chunk may contain fewer items.
    fn chunked(self, page_size: usize) -> PagedIterator<Self>;

    /// Consumes only the elements required to materialise the requested page, returning the
    /// collected items plus metadata.
    fn paginate(self, pagination: Pagination) -> PaginatedPage<Self::Item>;
}

impl<I> PaginateExt for I
where
    I: Iterator,
{
    fn chunked(self, page_size: usize) -> PagedIterator<Self> {
        PagedIterator::new(self, page_size)
    }

    fn paginate(self, pagination: Pagination) -> PaginatedPage<Self::Item> {
        let pagination = Pagination::new(pagination.cursor(), pagination.page_size());
        let mut iter = self.skip(pagination.offset());
        let mut buffer = Vec::with_capacity(pagination.page_size() + 1);

        for item in iter.by_ref().take(pagination.page_size() + 1) {
            buffer.push(item);
        }

        let has_more = buffer.len() > pagination.page_size();
        if has_more {
            buffer.truncate(pagination.page_size());
        }

        PaginatedPage::from_items(buffer, pagination, has_more, None)
    }
}

/// Utility to paginate any iterable by converting it into an iterator first.
pub fn paginate_into_iter<T, I>(iterable: I, pagination: Pagination) -> PaginatedPage<T>
where
    I: IntoIterator<Item = T>,
{
    iterable.into_iter().paginate(pagination)
}

/// Calculates the total number of pages for a given count and page size.
pub fn total_pages(total_count: usize, per_page: usize) -> usize {
    if per_page == 0 {
        0
    } else {
        total_count.saturating_add(per_page.saturating_sub(1)) / per_page
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_offset_and_next_cursor() {
        let pagination = Pagination::new(2, 25);
        assert_eq!(pagination.offset(), 50);
        assert_eq!(pagination.page_size(), 25);
        assert_eq!(pagination.cursor(), 2);
        assert_eq!(pagination.next_cursor(true), Some(3));
        assert_eq!(pagination.next_cursor(false), None);
    }

    #[test]
    fn paginate_iterator_consumes_only_required_items() {
        let data = 0..1000;
        let pagination = Pagination::new(3, 15);
        let page = data.paginate(pagination);

        assert_eq!(page.items.len(), 15);
        assert_eq!(page.items.first(), Some(&45));
        assert_eq!(page.items.last(), Some(&59));
        assert_eq!(page.summary.current_cursor, 3);
        assert_eq!(page.summary.next_cursor, Some(4));
        assert!(page.summary.has_more);
    }

    #[test]
    fn paginate_iterator_handles_end_of_stream() {
        let data = 0..10;
        let pagination = Pagination::new(1, 8);
        let page = data.paginate(pagination);

        assert_eq!(page.items, vec![8, 9]);
        assert_eq!(page.summary.has_more, false);
        assert_eq!(page.summary.next_cursor, None);
    }

    #[test]
    fn chunked_iteration_emits_equal_sized_pages() {
        let mut chunks = (0..11).chunked(4);
        assert_eq!(chunks.next(), Some(vec![0, 1, 2, 3]));
        assert_eq!(chunks.next(), Some(vec![4, 5, 6, 7]));
        assert_eq!(chunks.next(), Some(vec![8, 9, 10]));
        assert_eq!(chunks.next(), None);
    }

    #[test]
    fn helper_functions_cover_total_pages_and_map_items() {
        let pagination = Pagination::new(0, 5);
        assert_eq!(pagination.total_pages(23), 5);
        assert_eq!(pagination.total_pages(23), 5);
        assert_eq!(super::total_pages(23, 5), 5);

        let page = paginate_into_iter(0..5, pagination);
        let mapped = page.map_items(|value| value * 2);

        assert_eq!(mapped.items, vec![0, 2, 4, 6, 8]);
        assert_eq!(mapped.summary.has_more, false);
    }
}
