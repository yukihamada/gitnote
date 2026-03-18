use std::path::Path;

use git2::{Cred, Oid, PushOptions, RemoteCallbacks, Repository, Signature};

use crate::error::AppError;
use crate::models::CommitInfo;

/// Initialize a bare Git repository at the given path.
pub fn init_repo(path: &Path) -> Result<Repository, AppError> {
    if path.exists() {
        Ok(Repository::open_bare(path)?)
    } else {
        Ok(Repository::init_bare(path)?)
    }
}

/// Build a Markdown page with YAML frontmatter.
fn build_markdown(id: &str, title: &str, tags: &[String], icon: &str, content: &str) -> String {
    let tags_yaml = if tags.is_empty() {
        "[]".to_string()
    } else {
        format!(
            "[{}]",
            tags.iter()
                .map(|t| format!("\"{}\"", t.replace('"', "\\\"")))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    format!(
        "---\nid: \"{id}\"\ntitle: \"{title}\"\ntags: {tags_yaml}\nicon: \"{icon}\"\n---\n\n{content}"
    )
}

/// Write a page to the repo and create a commit. Returns the commit OID.
pub fn write_page(
    repo: &Repository,
    page_id: &str,
    title: &str,
    tags: &[String],
    icon: &str,
    content: &str,
    message: &str,
) -> Result<Oid, AppError> {
    let markdown = build_markdown(page_id, title, tags, icon, content);
    let blob_oid = repo.blob(markdown.as_bytes())?;

    let filename = format!("{page_id}.md");
    let sig = Signature::now("GitNote", "gitnote@localhost")?;

    // Build tree: get existing tree from HEAD (if any), add/update our file
    let mut tree_builder = if let Ok(head) = repo.head() {
        let tree = head.peel_to_commit()?.tree()?;
        repo.treebuilder(Some(&tree))?
    } else {
        repo.treebuilder(None)?
    };

    tree_builder.insert(&filename, blob_oid, 0o100644)?;
    let tree_oid = tree_builder.write()?;
    let tree = repo.find_tree(tree_oid)?;

    let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent_commit.iter().collect();

    let commit_oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;

    Ok(commit_oid)
}

/// Read a page's raw content from the HEAD tree.
pub fn read_page(repo: &Repository, page_id: &str) -> Result<Option<String>, AppError> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return Ok(None),
    };
    let tree = head.peel_to_commit()?.tree()?;
    let filename = format!("{page_id}.md");

    match tree.get_name(&filename) {
        Some(entry) => {
            let blob = repo.find_blob(entry.id())?;
            let content = std::str::from_utf8(blob.content())
                .map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(Some(content.to_string()))
        }
        None => Ok(None),
    }
}

/// Delete a page from the repo tree and create a commit.
pub fn delete_page(repo: &Repository, page_id: &str, title: &str) -> Result<Oid, AppError> {
    let head = repo.head()?;
    let parent_commit = head.peel_to_commit()?;
    let tree = parent_commit.tree()?;

    let filename = format!("{page_id}.md");
    let mut tree_builder = repo.treebuilder(Some(&tree))?;
    tree_builder.remove(&filename)?;
    let tree_oid = tree_builder.write()?;
    let new_tree = repo.find_tree(tree_oid)?;

    let sig = Signature::now("GitNote", "gitnote@localhost")?;
    let message = format!("delete: {title}");
    let commit_oid = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &message,
        &new_tree,
        &[&parent_commit],
    )?;

    Ok(commit_oid)
}

/// Get the commit history for a specific page.
pub fn page_history(
    repo: &Repository,
    page_id: &str,
    limit: usize,
) -> Result<Vec<CommitInfo>, AppError> {
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return Ok(vec![]),
    };
    let head_commit = head.peel_to_commit()?;

    let mut revwalk = repo.revwalk()?;
    revwalk.push(head_commit.id())?;
    revwalk.set_sorting(git2::Sort::TIME)?;

    let filename = format!("{page_id}.md");
    let mut commits = Vec::new();

    for oid_result in revwalk {
        if commits.len() >= limit {
            break;
        }
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        // Check if this commit touched the file
        let tree = commit.tree()?;
        let has_file = tree.get_name(&filename).is_some();

        let parent_has_file = if commit.parent_count() > 0 {
            let parent = commit.parent(0)?;
            parent.tree()?.get_name(&filename).is_some()
        } else {
            false
        };

        // If file was added, modified, or removed in this commit
        if has_file != parent_has_file || (has_file && parent_has_file) {
            // For modified case, check if blob changed
            if has_file && parent_has_file {
                let blob_id = tree.get_name(&filename).unwrap().id();
                let parent_blob_id = commit.parent(0)?.tree()?.get_name(&filename).unwrap().id();
                if blob_id == parent_blob_id {
                    continue;
                }
            }

            let time = commit.time();
            let timestamp = chrono::DateTime::from_timestamp(time.seconds(), 0)
                .unwrap_or_default();

            commits.push(CommitInfo {
                oid: oid.to_string(),
                message: commit.message().unwrap_or("").to_string(),
                author: commit.author().name().unwrap_or("unknown").to_string(),
                timestamp,
            });
        }
    }

    Ok(commits)
}

/// Read a page at a specific commit revision.
pub fn page_at_revision(
    repo: &Repository,
    page_id: &str,
    commit_oid: Oid,
) -> Result<Option<String>, AppError> {
    let commit = repo.find_commit(commit_oid)?;
    let tree = commit.tree()?;
    let filename = format!("{page_id}.md");

    match tree.get_name(&filename) {
        Some(entry) => {
            let blob = repo.find_blob(entry.id())?;
            let content = std::str::from_utf8(blob.content())
                .map_err(|e| AppError::Internal(e.to_string()))?;
            Ok(Some(content.to_string()))
        }
        None => Ok(None),
    }
}

/// Set up the remote for GitHub sync.
pub fn setup_remote(repo: &Repository, remote_url: &str) -> Result<(), AppError> {
    match repo.find_remote("origin") {
        Ok(_) => {
            repo.remote_set_url("origin", remote_url)?;
        }
        Err(_) => {
            repo.remote("origin", remote_url)?;
        }
    }
    Ok(())
}

/// Push to the remote (GitHub). Runs in background, logs errors but doesn't fail the request.
pub fn push_to_remote(repo: &Repository) {
    let token = match std::env::var("GITHUB_TOKEN") {
        Ok(t) => t,
        Err(_) => {
            tracing::debug!("GITHUB_TOKEN not set, skipping push");
            return;
        }
    };

    let mut remote = match repo.find_remote("origin") {
        Ok(r) => r,
        Err(_) => {
            tracing::debug!("No remote 'origin', skipping push");
            return;
        }
    };

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |_url, _username, _allowed| {
        Cred::userpass_plaintext("x-access-token", &token)
    });

    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    // Determine the refspec based on HEAD
    let refspec = if repo.head().is_ok() {
        "refs/heads/main:refs/heads/main"
    } else {
        return;
    };

    // Ensure HEAD points to refs/heads/main
    if let Ok(head) = repo.head() {
        if head.name() != Some("refs/heads/main") {
            if let Ok(commit) = head.peel_to_commit() {
                let _ = repo.branch("main", &commit, true);
                let _ = repo.set_head("refs/heads/main");
            }
        }
    }

    match remote.push(&[refspec], Some(&mut push_options)) {
        Ok(_) => tracing::info!("Pushed to GitHub successfully"),
        Err(e) => tracing::warn!("Failed to push to GitHub: {e}"),
    }
}

/// Parse content from a Markdown file (strip frontmatter).
pub fn extract_content(raw: &str) -> &str {
    if raw.starts_with("---") {
        if let Some(end) = raw[3..].find("---") {
            let after = &raw[3 + end + 3..];
            return after.trim_start_matches('\n');
        }
    }
    raw
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_read_delete() {
        let tmp = TempDir::new().unwrap();
        let repo_path = tmp.path().join("test.git");
        let repo = init_repo(&repo_path).unwrap();

        // Write
        let oid = write_page(&repo, "page1", "Test Page", &["tag1".into()], "📝", "Hello world", "create: Test Page").unwrap();
        assert!(!oid.is_zero());

        // Read
        let content = read_page(&repo, "page1").unwrap().unwrap();
        assert!(content.contains("Hello world"));
        assert!(content.contains("title: \"Test Page\""));

        // Extract content
        let body = extract_content(&content);
        assert_eq!(body, "Hello world");

        // Update
        write_page(&repo, "page1", "Updated", &[], "📝", "Updated body", "update: Updated").unwrap();
        let content2 = read_page(&repo, "page1").unwrap().unwrap();
        assert!(content2.contains("Updated body"));

        // History
        let history = page_history(&repo, "page1", 10).unwrap();
        assert_eq!(history.len(), 2);

        // Delete
        delete_page(&repo, "page1", "Updated").unwrap();
        let content3 = read_page(&repo, "page1").unwrap();
        assert!(content3.is_none());
    }
}
