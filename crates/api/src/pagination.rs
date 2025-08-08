use serde::Deserialize;

pub const DEFAULT_PAGE_SIZE: u32 = 25;
pub const MAX_PAGE_SIZE: u32 = 100;

#[derive(Debug, Clone, Deserialize)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// Compute start/end indices for a slice based on total length and pagination params.
/// Returns (start, end, effective_page, effective_page_size)
pub fn slice_bounds(total: usize, page: Option<u32>, page_size: Option<u32>) -> (usize, usize, u32, u32) {
    let page_size = page_size.unwrap_or(DEFAULT_PAGE_SIZE).clamp(1, MAX_PAGE_SIZE);
    let page = page.unwrap_or(1).max(1);
    let start = (page as usize - 1).saturating_mul(page_size as usize);
    let end = (start + page_size as usize).min(total);
    (start.min(total), end, page, page_size)
}


