use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{Connection, params};

use crate::error::AppError;
use crate::models::{CreatePageRequest, PageSummary, UpdatePageRequest};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new(path: &str) -> Result<Self, AppError> {
        let conn = if path == ":memory:" {
            Connection::open_in_memory()?
        } else {
            Connection::open(path)?
        };
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS pages (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL DEFAULT '',
                tags TEXT DEFAULT '[]',
                parent_id TEXT,
                icon TEXT DEFAULT '',
                content_hash TEXT NOT NULL DEFAULT '',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                deleted_at TEXT,
                FOREIGN KEY (parent_id) REFERENCES pages(id)
            );
            CREATE INDEX IF NOT EXISTS idx_pages_parent ON pages(parent_id);
            CREATE INDEX IF NOT EXISTS idx_pages_updated ON pages(updated_at DESC);

            CREATE VIRTUAL TABLE IF NOT EXISTS pages_fts USING fts5(
                title, content, tags, content_rowid='rowid'
            );",
        )?;
        Ok(())
    }

    pub fn insert_page(
        &self,
        id: &str,
        req: &CreatePageRequest,
        content_hash: &str,
    ) -> Result<PageSummary, AppError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        let tags_json = serde_json::to_string(&req.tags).unwrap_or_default();

        conn.execute(
            "INSERT INTO pages (id, title, tags, parent_id, icon, content_hash, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                req.title,
                tags_json,
                req.parent_id,
                req.icon,
                content_hash,
                now.to_rfc3339(),
                now.to_rfc3339(),
            ],
        )?;

        // Update FTS index
        conn.execute(
            "INSERT INTO pages_fts (rowid, title, content, tags) VALUES (
                (SELECT rowid FROM pages WHERE id = ?1), ?2, ?3, ?4
            )",
            params![id, req.title, req.content, tags_json],
        )?;

        Ok(PageSummary {
            id: id.to_string(),
            title: req.title.clone(),
            tags: req.tags.clone(),
            parent_id: req.parent_id.clone(),
            icon: req.icon.clone(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn get_page_meta(&self, id: &str) -> Result<Option<PageSummary>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, tags, parent_id, icon, created_at, updated_at
             FROM pages WHERE id = ?1 AND deleted_at IS NULL",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let tags_str: String = row.get(2)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            Ok(Some(PageSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                tags,
                parent_id: row.get(3)?,
                icon: row.get(4)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn list_pages(&self, parent_id: Option<&str>) -> Result<Vec<PageSummary>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut pages = Vec::new();

        let (sql, param_values): (&str, Vec<Box<dyn rusqlite::types::ToSql>>) = match parent_id {
            Some(pid) => (
                "SELECT id, title, tags, parent_id, icon, created_at, updated_at
                 FROM pages WHERE parent_id = ?1 AND deleted_at IS NULL ORDER BY updated_at DESC",
                vec![Box::new(pid.to_string()) as Box<dyn rusqlite::types::ToSql>],
            ),
            None => (
                "SELECT id, title, tags, parent_id, icon, created_at, updated_at
                 FROM pages WHERE parent_id IS NULL AND deleted_at IS NULL ORDER BY updated_at DESC",
                vec![],
            ),
        };

        let mut stmt = conn.prepare(sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = param_values.iter().map(|p| p.as_ref()).collect();
        let mut rows = stmt.query(params.as_slice())?;

        while let Some(row) = rows.next()? {
            let tags_str: String = row.get(2)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            pages.push(PageSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                tags,
                parent_id: row.get(3)?,
                icon: row.get(4)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            });
        }

        Ok(pages)
    }

    pub fn update_page(
        &self,
        id: &str,
        req: &UpdatePageRequest,
        content_hash: &str,
        content: &str,
    ) -> Result<PageSummary, AppError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();

        // Get current values
        let mut stmt = conn.prepare(
            "SELECT title, tags, parent_id, icon, created_at FROM pages WHERE id = ?1 AND deleted_at IS NULL",
        )?;
        let mut rows = stmt.query(params![id])?;
        let row = rows.next()?.ok_or(AppError::NotFound)?;

        let title = req.title.clone().unwrap_or_else(|| row.get(0).unwrap());
        let tags_json = req
            .tags
            .as_ref()
            .map(|t| serde_json::to_string(t).unwrap())
            .unwrap_or_else(|| row.get(1).unwrap());
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
        let parent_id: Option<String> = match &req.parent_id {
            Some(p) => p.clone(),
            None => row.get(2).unwrap(),
        };
        let icon = req.icon.clone().unwrap_or_else(|| row.get(3).unwrap());
        let created_at_str: String = row.get(4)?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .unwrap()
            .with_timezone(&chrono::Utc);

        drop(rows);
        drop(stmt);

        conn.execute(
            "UPDATE pages SET title=?1, tags=?2, parent_id=?3, icon=?4, content_hash=?5, updated_at=?6
             WHERE id=?7",
            params![title, tags_json, parent_id, icon, content_hash, now.to_rfc3339(), id],
        )?;

        // Update FTS
        conn.execute(
            "DELETE FROM pages_fts WHERE rowid = (SELECT rowid FROM pages WHERE id = ?1)",
            params![id],
        )?;
        conn.execute(
            "INSERT INTO pages_fts (rowid, title, content, tags) VALUES (
                (SELECT rowid FROM pages WHERE id = ?1), ?2, ?3, ?4
            )",
            params![id, title, content, tags_json],
        )?;

        Ok(PageSummary {
            id: id.to_string(),
            title,
            tags,
            parent_id,
            icon,
            created_at,
            updated_at: now,
        })
    }

    pub fn soft_delete_page(&self, id: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now();
        let changed = conn.execute(
            "UPDATE pages SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL",
            params![now.to_rfc3339(), id],
        )?;
        if changed == 0 {
            return Err(AppError::NotFound);
        }

        // Remove from FTS
        conn.execute(
            "DELETE FROM pages_fts WHERE rowid = (SELECT rowid FROM pages WHERE id = ?1)",
            params![id],
        )?;

        Ok(())
    }

    pub fn search(&self, query: &str, limit: usize, offset: usize) -> Result<(Vec<PageSummary>, usize), AppError> {
        let conn = self.conn.lock().unwrap();

        // Count total
        let total: usize = conn.query_row(
            "SELECT COUNT(*) FROM pages_fts f
             JOIN pages p ON p.rowid = f.rowid
             WHERE pages_fts MATCH ?1 AND p.deleted_at IS NULL",
            params![query],
            |row| row.get(0),
        ).unwrap_or(0);

        let mut stmt = conn.prepare(
            "SELECT p.id, p.title, p.tags, p.parent_id, p.icon, p.created_at, p.updated_at
             FROM pages_fts f
             JOIN pages p ON p.rowid = f.rowid
             WHERE pages_fts MATCH ?1 AND p.deleted_at IS NULL
             ORDER BY rank
             LIMIT ?2 OFFSET ?3",
        )?;

        let mut pages = Vec::new();
        let mut rows = stmt.query(params![query, limit, offset])?;

        while let Some(row) = rows.next()? {
            let tags_str: String = row.get(2)?;
            let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
            pages.push(PageSummary {
                id: row.get(0)?,
                title: row.get(1)?,
                tags,
                parent_id: row.get(3)?,
                icon: row.get(4)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .unwrap()
                    .with_timezone(&chrono::Utc),
            });
        }

        Ok((pages, total))
    }
}
