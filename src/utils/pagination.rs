use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams, Clone)]
pub struct PaginationQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
}

impl fmt::Display for PaginationQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "page:{}:limit:{}", self.page(), self.limit())
    }
}

impl PaginationQuery {
    pub fn page(&self) -> u32 {
        let page = self.page.unwrap_or(1); // Default page to 1
        if page > 1 { page } else { 1 }
    }

    pub fn limit(&self) -> u32 {
        let limit = self.limit.unwrap_or(20); // Default limit to 20
        if limit > 0 { limit } else { 20 }
    }

    pub fn skip(&self) -> u32 {
        let skip = (self.page() - 1) * self.limit();
        if skip > 0 { skip } else { 0 }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PaginationResponseDto<T> {
    #[schema(example = 1)]
    pub total_docs: u32,

    #[schema(example = 1)]
    pub total_pages: u32,

    #[schema(example = 1)]
    pub page: u32,

    #[schema(example = 10)]
    pub limit: u32,

    pub docs: Vec<T>,
}

impl<T> PaginationResponseDto<T> {
    pub fn empty() -> Self {
        PaginationResponseDto {
            page: 1,
            limit: 20,
            total_docs: 0,
            total_pages: 0,
            docs: vec![],
        }
    }
}
