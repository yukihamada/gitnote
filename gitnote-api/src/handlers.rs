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
    let filename = git::title_to_filename(&req.title);

    let repo = git::init_repo(&state.repo_path)?;
    git::write_page(&repo, &id, &req.title, &req.tags, &req.icon, &req.content, &message, None)?;
    update_readme(&repo, &state)?;
    git::push_to_remote(&repo);

    let summary = state.db.insert_page(&id, &req, "", &filename)?;
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
    let filename = state.db.get_filename(&id)?.ok_or(AppError::NotFound)?;

    let repo = git::init_repo(&state.repo_path)?;
    let raw = git::read_page(&repo, &filename)?.ok_or(AppError::NotFound)?;
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
    let old_filename = state.db.get_filename(&id)?.ok_or(AppError::NotFound)?;

    let title = req.title.as_deref().unwrap_or(&existing.title);
    let tags = req.tags.as_deref().unwrap_or(&existing.tags);
    let icon = req.icon.as_deref().unwrap_or(&existing.icon);
    let new_filename = git::title_to_filename(title);

    let repo = git::init_repo(&state.repo_path)?;
    let content = if let Some(ref c) = req.content {
        c.clone()
    } else {
        let raw = git::read_page(&repo, &old_filename)?.unwrap_or_default();
        git::extract_content(&raw).to_string()
    };

    let message = format!("update: {title}");
    git::write_page(&repo, &id, title, tags, icon, &content, &message, Some(&old_filename))?;
    update_readme(&repo, &state)?;
    git::push_to_remote(&repo);

    let summary = state.db.update_page(&id, &req, "", &content, &new_filename)?;
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
    let filename = state.db.get_filename(&id)?.ok_or(AppError::NotFound)?;

    let repo = git::init_repo(&state.repo_path)?;
    git::delete_page(&repo, &filename, &meta.title)?;
    update_readme(&repo, &state)?;
    git::push_to_remote(&repo);
    state.db.soft_delete_page(&id)?;

    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub async fn page_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<CommitInfo>>, AppError> {
    let _ = state.db.get_page_meta(&id)?.ok_or(AppError::NotFound)?;
    let filename = state.db.get_filename(&id)?.ok_or(AppError::NotFound)?;
    let repo = git::init_repo(&state.repo_path)?;
    let commits = git::page_history(&repo, &filename, 50)?;
    Ok(Json(commits))
}

pub async fn page_at_revision(
    State(state): State<Arc<AppState>>,
    Path((id, oid_str)): Path<(String, String)>,
) -> Result<Json<Page>, AppError> {
    let meta = state.db.get_page_meta(&id)?.ok_or(AppError::NotFound)?;
    let filename = state.db.get_filename(&id)?.ok_or(AppError::NotFound)?;
    let repo = git::init_repo(&state.repo_path)?;
    let oid = Oid::from_str(&oid_str).map_err(|e| AppError::Internal(e.to_string()))?;
    let raw = git::page_at_revision(&repo, &filename, oid)?.ok_or(AppError::NotFound)?;
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
    let filename = state.db.get_filename(&id)?.ok_or(AppError::NotFound)?;

    let repo = git::init_repo(&state.repo_path)?;
    let oid = Oid::from_str(&oid_str).map_err(|e| AppError::Internal(e.to_string()))?;
    let raw = git::page_at_revision(&repo, &filename, oid)?.ok_or(AppError::NotFound)?;
    let content = git::extract_content(&raw).to_string();

    let message = format!("restore: {} to {}", &meta.title, &oid_str[..8]);
    git::write_page(&repo, &id, &meta.title, &meta.tags, &meta.icon, &content, &message, Some(&filename))?;
    update_readme(&repo, &state)?;
    git::push_to_remote(&repo);

    let req = UpdatePageRequest {
        title: None,
        content: Some(content.clone()),
        tags: None,
        parent_id: None,
        icon: None,
    };
    let summary = state.db.update_page(&id, &req, "", &content, &filename)?;
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

/// Generate a README.md in the git repo with a table of all pages.
fn update_readme(repo: &git2::Repository, state: &AppState) -> Result<(), AppError> {
    let pages = state.db.list_pages(None)?;
    let mut readme = String::from("# GitNote\n\n> Notion-like notes backed by Git. Edit at [gitnote-api.fly.dev](https://gitnote-api.fly.dev)\n\n## Pages\n\n| | Title | Tags |\n|---|---|---|\n");
    for p in &pages {
        let tags = if p.tags.is_empty() {
            String::new()
        } else {
            p.tags.iter().map(|t| format!("`{t}`")).collect::<Vec<_>>().join(" ")
        };
        let filename = git::title_to_filename(&p.title);
        let icon = if p.icon.is_empty() { "📄" } else { &p.icon };
        readme.push_str(&format!("| {icon} | [{}]({}) | {tags} |\n", p.title, filename));
    }
    readme.push_str(&format!("\n---\n*{} pages total*\n", pages.len()));

    // Write README.md to the repo tree
    let blob_oid = repo.blob(readme.as_bytes())?;
    let sig = git2::Signature::now("GitNote", "gitnote@localhost")?;

    let mut tree_builder = if let Ok(head) = repo.head() {
        let tree = head.peel_to_commit()?.tree()?;
        repo.treebuilder(Some(&tree))?
    } else {
        repo.treebuilder(None)?
    };

    tree_builder.insert("README.md", blob_oid, 0o100644)?;
    let tree_oid = tree_builder.write()?;
    let tree = repo.find_tree(tree_oid)?;

    let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, "update: README.md", &tree, &parents)?;

    Ok(())
}
