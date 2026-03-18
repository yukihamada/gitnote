use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::Json;
use git2::Oid;
use uuid::Uuid;

use crate::error::AppError;
use crate::git;
use crate::models::{
    CommitInfo, CreatePageRequest, ListPagesQuery, Page, PageListResponse, SearchQuery,
    SearchResult, UpdatePageRequest,
};
use crate::AppState;

pub async fn create_page(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePageRequest>,
) -> Result<Json<Page>, AppError> {
    let id = Uuid::now_v7().to_string();
    let message = format!("create: {}", &req.title);

    let repo = git::init_repo(&state.repo_path)?;
    git::write_page(&repo, &id, &req.title, &req.tags, &req.icon, &req.content, &message)?;
    git::push_to_remote(&repo);

    let summary = state.db.insert_page(&id, &req, "")?;
    Ok(Json(Page {
        id: summary.id,
        title: summary.title,
        content: req.content.clone(),
        tags: summary.tags,
        parent_id: summary.parent_id,
        icon: summary.icon,
        created_at: summary.created_at,
        updated_at: summary.updated_at,
    }))
}

pub async fn list_pages(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListPagesQuery>,
) -> Result<Json<PageListResponse>, AppError> {
    let pages = state.db.list_pages(query.parent_id.as_deref())?;
    let total = pages.len();
    Ok(Json(PageListResponse { pages, total }))
}

pub async fn get_page(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Page>, AppError> {
    let meta = state.db.get_page_meta(&id)?.ok_or(AppError::NotFound)?;

    let repo = git::init_repo(&state.repo_path)?;
    let raw = git::read_page(&repo, &id)?.ok_or(AppError::NotFound)?;
    let content = git::extract_content(&raw).to_string();

    Ok(Json(Page {
        id: meta.id,
        title: meta.title,
        content,
        tags: meta.tags,
        parent_id: meta.parent_id,
        icon: meta.icon,
        created_at: meta.created_at,
        updated_at: meta.updated_at,
    }))
}

pub async fn update_page(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdatePageRequest>,
) -> Result<Json<Page>, AppError> {
    let existing = state.db.get_page_meta(&id)?.ok_or(AppError::NotFound)?;

    let title = req.title.as_deref().unwrap_or(&existing.title);
    let tags = req.tags.as_deref().unwrap_or(&existing.tags);
    let icon = req.icon.as_deref().unwrap_or(&existing.icon);

    // Get existing content if not provided
    let repo = git::init_repo(&state.repo_path)?;
    let content = if let Some(ref c) = req.content {
        c.clone()
    } else {
        let raw = git::read_page(&repo, &id)?.unwrap_or_default();
        git::extract_content(&raw).to_string()
    };

    let message = format!("update: {title}");
    git::write_page(&repo, &id, title, tags, icon, &content, &message)?;
    git::push_to_remote(&repo);

    let summary = state.db.update_page(&id, &req, "", &content)?;
    Ok(Json(Page {
        id: summary.id,
        title: summary.title,
        content,
        tags: summary.tags,
        parent_id: summary.parent_id,
        icon: summary.icon,
        created_at: summary.created_at,
        updated_at: summary.updated_at,
    }))
}

pub async fn delete_page(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let meta = state.db.get_page_meta(&id)?.ok_or(AppError::NotFound)?;

    let repo = git::init_repo(&state.repo_path)?;
    git::delete_page(&repo, &id, &meta.title)?;
    git::push_to_remote(&repo);
    state.db.soft_delete_page(&id)?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn page_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CommitInfo>>, AppError> {
    let _ = state.db.get_page_meta(&id)?.ok_or(AppError::NotFound)?;
    let repo = git::init_repo(&state.repo_path)?;
    let commits = git::page_history(&repo, &id, 50)?;
    Ok(Json(commits))
}

pub async fn page_at_revision(
    State(state): State<Arc<AppState>>,
    Path((id, oid_str)): Path<(String, String)>,
) -> Result<Json<Page>, AppError> {
    let meta = state.db.get_page_meta(&id)?.ok_or(AppError::NotFound)?;
    let repo = git::init_repo(&state.repo_path)?;
    let oid = Oid::from_str(&oid_str).map_err(|e| AppError::Internal(e.to_string()))?;
    let raw = git::page_at_revision(&repo, &id, oid)?.ok_or(AppError::NotFound)?;
    let content = git::extract_content(&raw).to_string();
    Ok(Json(Page {
        id: meta.id,
        title: meta.title,
        content,
        tags: meta.tags,
        parent_id: meta.parent_id,
        icon: meta.icon,
        created_at: meta.created_at,
        updated_at: meta.updated_at,
    }))
}

pub async fn restore_page(
    State(state): State<Arc<AppState>>,
    Path((id, oid_str)): Path<(String, String)>,
) -> Result<Json<Page>, AppError> {
    let meta = state.db.get_page_meta(&id)?.ok_or(AppError::NotFound)?;

    let repo = git::init_repo(&state.repo_path)?;
    let oid = Oid::from_str(&oid_str).map_err(|e| AppError::Internal(e.to_string()))?;
    let raw = git::page_at_revision(&repo, &id, oid)?.ok_or(AppError::NotFound)?;
    let content = git::extract_content(&raw).to_string();

    let message = format!("restore: {} to {}", &meta.title, &oid_str[..8]);
    git::write_page(&repo, &id, &meta.title, &meta.tags, &meta.icon, &content, &message)?;
    git::push_to_remote(&repo);

    let req = UpdatePageRequest {
        title: None,
        content: Some(content.clone()),
        tags: None,
        parent_id: None,
        icon: None,
    };
    let summary = state.db.update_page(&id, &req, "", &content)?;
    Ok(Json(Page {
        id: summary.id,
        title: summary.title,
        content,
        tags: summary.tags,
        parent_id: summary.parent_id,
        icon: summary.icon,
        created_at: summary.created_at,
        updated_at: summary.updated_at,
    }))
}

pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResult>, AppError> {
    let (pages, total) = state.db.search(&query.q, query.limit, query.offset)?;
    Ok(Json(SearchResult { pages, total }))
}

pub async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}
