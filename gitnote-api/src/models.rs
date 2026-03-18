use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Page {
    pub id: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    pub icon: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PageSummary {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    pub icon: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePageRequest {
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub parent_id: Option<String>,
    #[serde(default)]
    pub icon: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePageRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
    pub parent_id: Option<Option<String>>,
    pub icon: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PageListResponse {
    pub pages: Vec<PageSummary>,
    pub total: usize,
}

#[derive(Debug, Serialize, Clone)]
pub struct CommitInfo {
    pub oid: String,
    pub message: String,
    pub author: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListPagesQuery {
    pub parent_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    20
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub pages: Vec<PageSummary>,
    pub total: usize,
}
