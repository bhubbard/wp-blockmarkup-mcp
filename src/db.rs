use rusqlite::{params, Connection, Result};
use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use crate::constants::db_path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub id: i64,
    pub name: String,
    pub r#type: String, // "github-public", "github-private", "local-folder"
    pub repo_url: Option<String>,
    pub subfolder: Option<String>,
    pub local_path: Option<String>,
    pub token_env_var: Option<String>,
    pub branch: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub id: i64,
    pub source_id: i64,
    pub block_name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub block_type: String, // "static", "dynamic", "hybrid"
    pub api_version: Option<i64>,
    pub validation_status: String, // "verified", "structural_only", "attributes_only", "unverified"
    pub confidence: i64,
    pub file_path: String,
    pub content_hash: Option<String>,
    pub status: String, // "active", "removed"
    pub removed_at: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockAttribute {
    pub id: i64,
    pub block_id: i64,
    pub name: String,
    pub r#type: Option<String>,
    pub default_val: Option<String>,
    pub source: Option<String>,
    pub selector: Option<String>,
    pub enum_values: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSupport {
    pub id: i64,
    pub block_id: i64,
    pub feature: String,
    pub config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkupExample {
    pub id: i64,
    pub block_id: i64,
    pub title: String,
    pub description: Option<String>,
    pub markup: String,
    pub attributes_json: Option<String>,
    pub validation_status: String,
    pub features_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variation {
    pub id: i64,
    pub block_id: i64,
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub attributes: Option<String>,
    pub inner_blocks: Option<String>,
    pub scope: Option<String>,
    pub markup: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullBlockSchema {
    #[serde(flatten)]
    pub block: Block,
    pub attributes: Vec<BlockAttribute>,
    pub supports: Vec<BlockSupport>,
    pub variations: Vec<Variation>,
    pub examples: Vec<MarkupExample>,
    pub source_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsTotals {
    pub sources: i64,
    pub active_blocks: i64,
    pub removed_blocks: i64,
    pub static_blocks: i64,
    pub dynamic_blocks: i64,
    pub hybrid_blocks: i64,
    pub verified_blocks: i64,
    pub total_attributes: i64,
    pub total_variations: i64,
    pub total_examples: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceStats {
    pub name: String,
    pub blocks: i64,
    pub removed_blocks: i64,
    pub static_blocks: i64,
    pub dynamic_blocks: i64,
    pub verified_blocks: i64,
    pub attributes: i64,
    pub variations: i64,
    pub examples: i64,
    pub files: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub totals: StatsTotals,
    pub per_source: Vec<SourceStats>,
}

pub fn get_connection() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    let conn = Connection::open(&path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    init_db(&conn)?;
    Ok(conn)
}

fn init_db(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sources (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT UNIQUE NOT NULL,
            type TEXT NOT NULL,
            repo_url TEXT,
            subfolder TEXT,
            local_path TEXT,
            token_env_var TEXT,
            branch TEXT DEFAULT 'main',
            enabled INTEGER DEFAULT 1
        );",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS blocks (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
            block_name TEXT NOT NULL,
            title TEXT,
            description TEXT,
            category TEXT,
            block_type TEXT NOT NULL DEFAULT 'static',
            api_version INTEGER,
            validation_status TEXT DEFAULT 'unverified',
            confidence INTEGER DEFAULT 0,
            file_path TEXT NOT NULL,
            content_hash TEXT,
            status TEXT DEFAULT 'active',
            removed_at TEXT,
            first_seen_at TEXT DEFAULT (datetime('now')),
            last_seen_at TEXT DEFAULT (datetime('now')),
            UNIQUE(source_id, block_name)
        );",
        [],
    )?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_blocks_source_id ON blocks(source_id);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_blocks_block_name ON blocks(block_name);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_blocks_category ON blocks(category);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_blocks_block_type ON blocks(block_type);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_blocks_status ON blocks(status);", [])?;
    conn.execute("CREATE INDEX IF NOT EXISTS idx_blocks_source_status ON blocks(source_id, status);", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS attributes (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            block_id INTEGER NOT NULL REFERENCES blocks(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            type TEXT,
            default_val TEXT,
            source TEXT,
            selector TEXT,
            enum_values TEXT,
            UNIQUE(block_id, name)
        );",
        [],
    )?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_attributes_block_id ON attributes(block_id);", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS supports (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            block_id INTEGER NOT NULL REFERENCES blocks(id) ON DELETE CASCADE,
            feature TEXT NOT NULL,
            config TEXT,
            UNIQUE(block_id, feature)
        );",
        [],
    )?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_supports_block_id ON supports(block_id);", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS markup_examples (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            block_id INTEGER NOT NULL REFERENCES blocks(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            description TEXT,
            markup TEXT NOT NULL,
            attributes_json TEXT,
            validation_status TEXT DEFAULT 'unverified',
            features_used TEXT
        );",
        [],
    )?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_markup_block_id ON markup_examples(block_id);", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS variations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            block_id INTEGER NOT NULL REFERENCES blocks(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            title TEXT,
            description TEXT,
            attributes TEXT,
            inner_blocks TEXT,
            scope TEXT,
            markup TEXT,
            UNIQUE(block_id, name)
        );",
        [],
    )?;

    conn.execute("CREATE INDEX IF NOT EXISTS idx_variations_block_id ON variations(block_id);", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS indexed_files (
            source_id INTEGER NOT NULL REFERENCES sources(id) ON DELETE CASCADE,
            file_path TEXT NOT NULL,
            mtime_ms REAL,
            content_hash TEXT,
            UNIQUE(source_id, file_path)
        );",
        [],
    )?;

    // Create FTS5 virtual table
    conn.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS blocks_fts USING fts5(
            block_name,
            title,
            description,
            category,
            content='blocks',
            content_rowid='id'
        );",
        [],
    )?;

    Ok(())
}

// --- Sources ---

pub fn add_source(conn: &Connection, source: &Source) -> Result<()> {
    conn.execute(
        "INSERT INTO sources (name, type, repo_url, subfolder, local_path, token_env_var, branch, enabled)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            source.name,
            source.r#type,
            source.repo_url,
            source.subfolder,
            source.local_path,
            source.token_env_var,
            source.branch.as_deref().unwrap_or("main"),
            if source.enabled { 1 } else { 0 }
        ],
    )?;
    Ok(())
}

pub fn list_sources(conn: &Connection) -> Result<Vec<Source>> {
    let mut stmt = conn.prepare("SELECT id, name, type, repo_url, subfolder, local_path, token_env_var, branch, enabled FROM sources ORDER BY name")?;
    let rows = stmt.query_map([], |row| {
        Ok(Source {
            id: row.get(0)?,
            name: row.get(1)?,
            r#type: row.get(2)?,
            repo_url: row.get(3)?,
            subfolder: row.get(4)?,
            local_path: row.get(5)?,
            token_env_var: row.get(6)?,
            branch: row.get(7)?,
            enabled: row.get::<_, i32>(8)? != 0,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn get_source(conn: &Connection, name: &str) -> Result<Option<Source>> {
    let mut stmt = conn.prepare("SELECT id, name, type, repo_url, subfolder, local_path, token_env_var, branch, enabled FROM sources WHERE name = ?1")?;
    let mut rows = stmt.query_map(params![name], |row| {
        Ok(Source {
            id: row.get(0)?,
            name: row.get(1)?,
            r#type: row.get(2)?,
            repo_url: row.get(3)?,
            subfolder: row.get(4)?,
            local_path: row.get(5)?,
            token_env_var: row.get(6)?,
            branch: row.get(7)?,
            enabled: row.get::<_, i32>(8)? != 0,
        })
    })?;

    if let Some(row) = rows.next() {
        Ok(Some(row?))
    } else {
        Ok(None)
    }
}

pub fn remove_source(conn: &Connection, name: &str) -> Result<Option<Source>> {
    let source = get_source(conn, name)?;
    if let Some(ref s) = source {
        // Clean up FTS before cascade delete
        conn.execute("DELETE FROM blocks_fts WHERE rowid IN (SELECT id FROM blocks WHERE source_id = ?1)", params![s.id])?;
        conn.execute("DELETE FROM sources WHERE name = ?1", params![name])?;
    }
    Ok(source)
}

pub fn is_source_indexed(conn: &Connection, source_id: i64) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM indexed_files WHERE source_id = ?1",
        params![source_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

// --- Blocks ---

#[derive(Debug)]
pub enum UpsertAction {
    Inserted,
    Updated,
    Skipped,
}

pub struct UpsertResult {
    pub id: i64,
    pub action: UpsertAction,
}

pub fn upsert_block(conn: &mut Connection, block: &Block) -> Result<UpsertResult> {
    let tx = conn.transaction()?;
    
    let existing: Option<(i64, Option<String>)> = tx.query_row(
        "SELECT id, content_hash FROM blocks WHERE source_id = ?1 AND block_name = ?2",
        params![block.source_id, block.block_name],
        |row| Ok((row.get(0)?, row.get(1)?)),
    ).ok();

    if let Some((id, hash)) = existing {
        if hash == block.content_hash {
            tx.execute(
                "UPDATE blocks SET last_seen_at = datetime('now'), status = 'active' WHERE id = ?1",
                params![id],
            )?;
            tx.commit()?;
            return Ok(UpsertResult { id, action: UpsertAction::Skipped });
        }

        tx.execute(
            "UPDATE blocks SET
                title = ?1, description = ?2, category = ?3,
                block_type = ?4, api_version = ?5,
                validation_status = ?6, confidence = ?7,
                file_path = ?8, content_hash = ?9,
                status = 'active', removed_at = NULL, last_seen_at = datetime('now')
             WHERE id = ?10",
            params![
                block.title,
                block.description,
                block.category,
                block.block_type,
                block.api_version,
                block.validation_status,
                block.confidence,
                block.file_path,
                block.content_hash,
                id
            ],
        )?;

        // Update FTS
        tx.execute("DELETE FROM blocks_fts WHERE rowid = ?1", params![id])?;
        tx.execute(
            "INSERT INTO blocks_fts(rowid, block_name, title, description, category)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, block.block_name, block.title, block.description, block.category],
        )?;

        // Clear child data for re-insertion
        tx.execute("DELETE FROM attributes WHERE block_id = ?1", params![id])?;
        tx.execute("DELETE FROM supports WHERE block_id = ?1", params![id])?;
        tx.execute("DELETE FROM markup_examples WHERE block_id = ?1", params![id])?;
        tx.execute("DELETE FROM variations WHERE block_id = ?1", params![id])?;

        tx.commit()?;
        return Ok(UpsertResult { id, action: UpsertAction::Updated });
    }

    let mut stmt = tx.prepare(
        "INSERT INTO blocks (
            source_id, block_name, title, description, category,
            block_type, api_version, validation_status, confidence,
            file_path, content_hash, status
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'active')"
    )?;
    stmt.execute(params![
        block.source_id,
        block.block_name,
        block.title,
        block.description,
        block.category,
        block.block_type,
        block.api_version,
        block.validation_status,
        block.confidence,
        block.file_path,
        block.content_hash,
    ])?;
    drop(stmt);
    let id = tx.last_insert_rowid();

    tx.execute(
        "INSERT INTO blocks_fts(rowid, block_name, title, description, category)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, block.block_name, block.title, block.description, block.category],
    )?;

    tx.commit()?;
    Ok(UpsertResult { id, action: UpsertAction::Inserted })
}

pub fn mark_blocks_removed(conn: &Connection, source_id: i64, active_block_ids: &[i64]) -> Result<i64> {
    let mut stmt = conn.prepare("SELECT id FROM blocks WHERE source_id = ?1 AND status = 'active'")?;
    let rows = stmt.query_map(params![source_id], |row| row.get::<_, i64>(0))?;
    
    let mut to_remove = Vec::new();
    let active_set: std::collections::HashSet<_> = active_block_ids.iter().cloned().collect();
    for row in rows {
        let id = row?;
        if !active_set.contains(&id) {
            to_remove.push(id);
        }
    }

    let count = to_remove.len() as i64;
    for id in to_remove {
        conn.execute(
            "UPDATE blocks SET status = 'removed', removed_at = datetime('now') WHERE id = ?1",
            params![id],
        )?;
    }
    Ok(count)
}

pub fn update_block_validation(conn: &Connection, block_id: i64, validation_status: &str) -> Result<()> {
    conn.execute(
        "UPDATE blocks SET validation_status = ?1 WHERE id = ?2",
        params![validation_status, block_id],
    )?;
    Ok(())
}

pub fn get_block_by_name(conn: &Connection, block_name: &str) -> Result<Option<Block>> {
    let mut stmt = conn.prepare(
        "SELECT id, source_id, block_name, title, description, category, block_type, api_version,
                validation_status, confidence, file_path, content_hash, status, removed_at, first_seen_at, last_seen_at
         FROM blocks
         WHERE block_name = ?1 AND status = 'active'
         ORDER BY last_seen_at DESC LIMIT 1"
    )?;
    let mut rows = stmt.query_map(params![block_name], |row| {
        Ok(Block {
            id: row.get(0)?,
            source_id: row.get(1)?,
            block_name: row.get(2)?,
            title: row.get(3)?,
            description: row.get(4)?,
            category: row.get(5)?,
            block_type: row.get(6)?,
            api_version: row.get(7)?,
            validation_status: row.get(8)?,
            confidence: row.get(9)?,
            file_path: row.get(10)?,
            content_hash: row.get(11)?,
            status: row.get(12)?,
            removed_at: row.get(13)?,
            first_seen_at: row.get(14)?,
            last_seen_at: row.get(15)?,
        })
    })?;

    if let Some(row) = rows.next() {
        Ok(Some(row?))
    } else {
        Ok(None)
    }
}

pub fn get_block_by_id(conn: &Connection, id: i64) -> Result<Option<Block>> {
    let mut stmt = conn.prepare(
        "SELECT id, source_id, block_name, title, description, category, block_type, api_version,
                validation_status, confidence, file_path, content_hash, status, removed_at, first_seen_at, last_seen_at
         FROM blocks
         WHERE id = ?1"
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Block {
            id: row.get(0)?,
            source_id: row.get(1)?,
            block_name: row.get(2)?,
            title: row.get(3)?,
            description: row.get(4)?,
            category: row.get(5)?,
            block_type: row.get(6)?,
            api_version: row.get(7)?,
            validation_status: row.get(8)?,
            confidence: row.get(9)?,
            file_path: row.get(10)?,
            content_hash: row.get(11)?,
            status: row.get(12)?,
            removed_at: row.get(13)?,
            first_seen_at: row.get(14)?,
            last_seen_at: row.get(15)?,
        })
    })?;

    if let Some(row) = rows.next() {
        Ok(Some(row?))
    } else {
        Ok(None)
    }
}

pub struct SearchOpts {
    pub source: Option<String>,
    pub category: Option<String>,
    pub block_type: Option<String>,
    pub include_removed: bool,
    pub limit: i64,
}

pub struct SearchResult {
    pub block: Block,
    pub source_name: String,
    pub rank: f64,
}

pub fn search_blocks(conn: &Connection, query: &str, opts: SearchOpts) -> Result<Vec<SearchResult>> {
    // Strip special FTS syntax to avoid parser errors
    let clean_query = query.replace(|c: char| !c.is_alphanumeric() && !c.is_whitespace(), " ");
    let terms: Vec<String> = clean_query.split_whitespace().map(|t| format!("\"{}\"*", t)).collect();
    if terms.is_empty() {
        return Ok(Vec::new());
    }
    let match_string = terms.join(" ");

    let mut sql = "
        SELECT b.id, b.source_id, b.block_name, b.title, b.description, b.category, b.block_type, b.api_version,
               b.validation_status, b.confidence, b.file_path, b.content_hash, b.status, b.removed_at, b.first_seen_at, b.last_seen_at,
               s.name AS source_name,
               bm25(blocks_fts, 10.0, 5.0, 3.0, 2.0) AS rank
        FROM blocks_fts
        JOIN blocks b ON b.id = blocks_fts.rowid
        JOIN sources s ON s.id = b.source_id
        WHERE blocks_fts MATCH ?1
    ".to_string();

    if !opts.include_removed {
        sql.push_str(" AND b.status = 'active'");
    }
    if let Some(ref source) = opts.source {
        sql.push_str(&format!(" AND s.name = '{}'", source.replace('\'', "''")));
    }
    if let Some(ref category) = opts.category {
        sql.push_str(&format!(" AND b.category = '{}'", category.replace('\'', "''")));
    }
    if let Some(ref block_type) = opts.block_type {
        sql.push_str(&format!(" AND b.block_type = '{}'", block_type.replace('\'', "''")));
    }

    sql.push_str(" ORDER BY rank LIMIT ?2");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![match_string, opts.limit], |row| {
        Ok(SearchResult {
            block: Block {
                id: row.get(0)?,
                source_id: row.get(1)?,
                block_name: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                category: row.get(5)?,
                block_type: row.get(6)?,
                api_version: row.get(7)?,
                validation_status: row.get(8)?,
                confidence: row.get(9)?,
                file_path: row.get(10)?,
                content_hash: row.get(11)?,
                status: row.get(12)?,
                removed_at: row.get(13)?,
                first_seen_at: row.get(14)?,
                last_seen_at: row.get(15)?,
            },
            source_name: row.get(16)?,
            rank: row.get(17)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

// --- Attributes ---

pub fn insert_attributes(conn: &mut Connection, block_id: i64, attributes: &serde_json::Map<String, serde_json::Value>) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO attributes (block_id, name, type, default_val, source, selector, enum_values)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;
        
        for (name, value) in attributes {
            let obj = value.as_object();
            let attr_type = obj.and_then(|o| o.get("type")).and_then(|t| {
                if t.is_array() {
                    t.as_array().map(|arr| {
                        arr.iter()
                            .filter_map(|val| val.as_str())
                            .collect::<Vec<_>>()
                            .join("|")
                    })
                } else {
                    t.as_str().map(|s| s.to_string())
                }
            });
            let default_val = obj.and_then(|o| o.get("default")).map(|d| d.to_string());
            let source = obj.and_then(|o| o.get("source")).and_then(|s| s.as_str()).map(|s| s.to_string());
            let selector = obj.and_then(|o| o.get("selector")).and_then(|s| s.as_str()).map(|s| s.to_string());
            let enum_values = obj.and_then(|o| o.get("enum")).map(|e| e.to_string());

            stmt.execute(params![
                block_id,
                name,
                attr_type,
                default_val,
                source,
                selector,
                enum_values
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn get_block_attributes(conn: &Connection, block_id: i64) -> Result<Vec<BlockAttribute>> {
    let mut stmt = conn.prepare("SELECT id, block_id, name, type, default_val, source, selector, enum_values FROM attributes WHERE block_id = ?1 ORDER BY name")?;
    let rows = stmt.query_map(params![block_id], |row| {
        Ok(BlockAttribute {
            id: row.get(0)?,
            block_id: row.get(1)?,
            name: row.get(2)?,
            r#type: row.get(3)?,
            default_val: row.get(4)?,
            source: row.get(5)?,
            selector: row.get(6)?,
            enum_values: row.get(7)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

pub fn list_block_attributes(conn: &Connection, block_name: &str) -> Result<Vec<BlockAttribute>> {
    let mut stmt = conn.prepare(
        "SELECT a.id, a.block_id, a.name, a.type, a.default_val, a.source, a.selector, a.enum_values
         FROM attributes a
         JOIN blocks b ON b.id = a.block_id
         WHERE b.block_name = ?1 AND b.status = 'active'
         ORDER BY a.name"
    )?;
    let rows = stmt.query_map(params![block_name], |row| {
        Ok(BlockAttribute {
            id: row.get(0)?,
            block_id: row.get(1)?,
            name: row.get(2)?,
            r#type: row.get(3)?,
            default_val: row.get(4)?,
            source: row.get(5)?,
            selector: row.get(6)?,
            enum_values: row.get(7)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

// --- Supports ---

pub fn insert_supports(conn: &mut Connection, block_id: i64, supports: &serde_json::Map<String, serde_json::Value>) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT OR REPLACE INTO supports (block_id, feature, config) VALUES (?1, ?2, ?3)"
        )?;
        for (feature, value) in supports {
            if value.is_null() {
                continue;
            }
            stmt.execute(params![
                block_id,
                feature,
                value.to_string()
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn get_block_supports(conn: &Connection, block_id: i64) -> Result<Vec<BlockSupport>> {
    let mut stmt = conn.prepare("SELECT id, block_id, feature, config FROM supports WHERE block_id = ?1 ORDER BY feature")?;
    let rows = stmt.query_map(params![block_id], |row| {
        Ok(BlockSupport {
            id: row.get(0)?,
            block_id: row.get(1)?,
            feature: row.get(2)?,
            config: row.get(3)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

// --- Markup Examples ---

pub fn insert_markup_example(conn: &Connection, example: &MarkupExample) -> Result<()> {
    conn.execute(
        "INSERT INTO markup_examples (block_id, title, description, markup, attributes_json, validation_status, features_used)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            example.block_id,
            example.title,
            example.description,
            example.markup,
            example.attributes_json,
            example.validation_status,
            example.features_used
        ],
    )?;
    Ok(())
}

pub struct MarkupOpts {
    pub features: Option<Vec<String>>,
    pub validated_only: bool,
}

pub fn get_block_markup(conn: &Connection, block_name: &str, opts: MarkupOpts) -> Result<Vec<MarkupExample>> {
    let mut sql = "
        SELECT me.id, me.block_id, me.title, me.description, me.markup, me.attributes_json, me.validation_status, me.features_used
        FROM markup_examples me
        JOIN blocks b ON b.id = me.block_id
        WHERE b.block_name = ?1 AND b.status = 'active'
    ".to_string();

    if opts.validated_only {
        sql.push_str(" AND me.validation_status = 'verified'");
    }

    sql.push_str(" ORDER BY me.id");

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![block_name], |row| {
        Ok(MarkupExample {
            id: row.get(0)?,
            block_id: row.get(1)?,
            title: row.get(2)?,
            description: row.get(3)?,
            markup: row.get(4)?,
            attributes_json: row.get(5)?,
            validation_status: row.get(6)?,
            features_used: row.get(7)?,
        })
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }

    if let Some(ref features) = opts.features {
        if !features.is_empty() {
            results.retain(|r| {
                if let Some(ref features_str) = r.features_used {
                    if let Ok(used_features) = serde_json::from_str::<Vec<String>>(features_str) {
                        return features.iter().any(|f| used_features.contains(f));
                    }
                }
                false
            });
        }
    }

    Ok(results)
}

// --- Variations ---

pub fn insert_variation(conn: &Connection, variation: &Variation) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO variations (block_id, name, title, description, attributes, inner_blocks, scope, markup)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            variation.block_id,
            variation.name,
            variation.title,
            variation.description,
            variation.attributes,
            variation.inner_blocks,
            variation.scope,
            variation.markup
        ],
    )?;
    Ok(())
}

pub fn get_block_variations(conn: &Connection, block_id: i64) -> Result<Vec<Variation>> {
    let mut stmt = conn.prepare("SELECT id, block_id, name, title, description, attributes, inner_blocks, scope, markup FROM variations WHERE block_id = ?1 ORDER BY name")?;
    let rows = stmt.query_map(params![block_id], |row| {
        Ok(Variation {
            id: row.get(0)?,
            block_id: row.get(1)?,
            name: row.get(2)?,
            title: row.get(3)?,
            description: row.get(4)?,
            attributes: row.get(5)?,
            inner_blocks: row.get(6)?,
            scope: row.get(7)?,
            markup: row.get(8)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariationSearchResult {
    #[serde(flatten)]
    pub variation: Variation,
    pub block_name: String,
    pub block_title: Option<String>,
    pub source_name: String,
}

pub fn search_variations(conn: &Connection, query: &str, limit: i64) -> Result<Vec<VariationSearchResult>> {
    let pattern = format!("%{}%", query);
    let mut stmt = conn.prepare(
        "SELECT v.id, v.block_id, v.name, v.title, v.description, v.attributes, v.inner_blocks, v.scope, v.markup,
                b.block_name, b.title AS block_title, s.name AS source_name
         FROM variations v
         JOIN blocks b ON b.id = v.block_id
         JOIN sources s ON s.id = b.source_id
         WHERE b.status = 'active'
           AND (v.name LIKE ?1 OR v.title LIKE ?1 OR v.description LIKE ?1)
         ORDER BY v.title
         LIMIT ?2"
    )?;
    let rows = stmt.query_map(params![pattern, limit], |row| {
        Ok(VariationSearchResult {
            variation: Variation {
                id: row.get(0)?,
                block_id: row.get(1)?,
                name: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                attributes: row.get(5)?,
                inner_blocks: row.get(6)?,
                scope: row.get(7)?,
                markup: row.get(8)?,
            },
            block_name: row.get(9)?,
            block_title: row.get(10)?,
            source_name: row.get(11)?,
        })
    })?;

    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

// --- Indexed Files ---

pub fn get_indexed_file(conn: &Connection, source_id: i64, file_path: &str) -> Result<Option<(f64, String)>> {
    let result = conn.query_row(
        "SELECT mtime_ms, content_hash FROM indexed_files WHERE source_id = ?1 AND file_path = ?2",
        params![source_id, file_path],
        |row| Ok((row.get::<_, f64>(0)?, row.get::<_, String>(1)?)),
    ).ok();
    Ok(result)
}

pub fn upsert_indexed_file(conn: &Connection, source_id: i64, file_path: &str, mtime_ms: f64, content_hash: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO indexed_files (source_id, file_path, mtime_ms, content_hash)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(source_id, file_path)
         DO UPDATE SET mtime_ms = ?3, content_hash = ?4",
        params![source_id, file_path, mtime_ms, content_hash],
    )?;
    Ok(())
}

pub fn clear_indexed_files(conn: &Connection, source_id: i64) -> Result<()> {
    conn.execute("DELETE FROM indexed_files WHERE source_id = ?1", params![source_id])?;
    Ok(())
}

// --- FTS Rebuild ---

pub fn rebuild_fts_index(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM blocks_fts", [])?;
    tx.execute(
        "INSERT INTO blocks_fts(rowid, block_name, title, description, category)
         SELECT id, block_name, title, description, category FROM blocks WHERE status = 'active'",
        [],
    )?;
    tx.commit()?;
    Ok(())
}

// --- Stats ---

pub fn get_stats(conn: &Connection) -> Result<Stats> {
    let sources_count: i64 = conn.query_row("SELECT COUNT(*) FROM sources", [], |r| r.get(0))?;
    let active_blocks: i64 = conn.query_row("SELECT COUNT(*) FROM blocks WHERE status = 'active'", [], |r| r.get(0))?;
    let removed_blocks: i64 = conn.query_row("SELECT COUNT(*) FROM blocks WHERE status = 'removed'", [], |r| r.get(0))?;
    let static_blocks: i64 = conn.query_row("SELECT COUNT(*) FROM blocks WHERE status = 'active' AND block_type = 'static'", [], |r| r.get(0))?;
    let dynamic_blocks: i64 = conn.query_row("SELECT COUNT(*) FROM blocks WHERE status = 'active' AND block_type = 'dynamic'", [], |r| r.get(0))?;
    let hybrid_blocks: i64 = conn.query_row("SELECT COUNT(*) FROM blocks WHERE status = 'active' AND block_type = 'hybrid'", [], |r| r.get(0))?;
    let verified_blocks: i64 = conn.query_row("SELECT COUNT(*) FROM blocks WHERE status = 'active' AND validation_status = 'verified'", [], |r| r.get(0))?;
    
    let total_attributes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM attributes a JOIN blocks b ON b.id = a.block_id WHERE b.status = 'active'",
        [],
        |r| r.get(0),
    )?;

    let total_variations: i64 = conn.query_row(
        "SELECT COUNT(*) FROM variations v JOIN blocks b ON b.id = v.block_id WHERE b.status = 'active'",
        [],
        |r| r.get(0),
    )?;

    let total_examples: i64 = conn.query_row(
        "SELECT COUNT(*) FROM markup_examples me JOIN blocks b ON b.id = me.block_id WHERE b.status = 'active'",
        [],
        |r| r.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT s.name,
                (SELECT COUNT(*) FROM blocks WHERE source_id = s.id AND status = 'active') AS blocks,
                (SELECT COUNT(*) FROM blocks WHERE source_id = s.id AND status = 'removed') AS removed_blocks,
                (SELECT COUNT(*) FROM blocks WHERE source_id = s.id AND status = 'active' AND block_type = 'static') AS static_blocks,
                (SELECT COUNT(*) FROM blocks WHERE source_id = s.id AND status = 'active' AND block_type = 'dynamic') AS dynamic_blocks,
                (SELECT COUNT(*) FROM blocks WHERE source_id = s.id AND status = 'active' AND validation_status = 'verified') AS verified_blocks,
                (SELECT COUNT(*) FROM attributes a JOIN blocks b ON b.id = a.block_id WHERE b.source_id = s.id AND b.status = 'active') AS attributes,
                (SELECT COUNT(*) FROM variations v JOIN blocks b ON b.id = v.block_id WHERE b.source_id = s.id AND b.status = 'active') AS variations,
                (SELECT COUNT(*) FROM markup_examples me JOIN blocks b ON b.id = me.block_id WHERE b.source_id = s.id AND b.status = 'active') AS examples,
                (SELECT COUNT(*) FROM indexed_files WHERE source_id = s.id) AS files
         FROM sources s ORDER BY s.name"
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(SourceStats {
            name: row.get(0)?,
            blocks: row.get(1)?,
            removed_blocks: row.get(2)?,
            static_blocks: row.get(3)?,
            dynamic_blocks: row.get(4)?,
            verified_blocks: row.get(5)?,
            attributes: row.get(6)?,
            variations: row.get(7)?,
            examples: row.get(8)?,
            files: row.get(9)?,
        })
    })?;

    let mut per_source = Vec::new();
    for row in rows {
        per_source.push(row?);
    }

    Ok(Stats {
        totals: StatsTotals {
            sources: sources_count,
            active_blocks,
            removed_blocks,
            static_blocks,
            dynamic_blocks,
            hybrid_blocks,
            verified_blocks,
            total_attributes,
            total_variations,
            total_examples,
        },
        per_source,
    })
}

// --- Full Block Schema ---

pub fn get_block_schema(conn: &Connection, block_name: &str) -> Result<Option<FullBlockSchema>> {
    let block = get_block_by_name(conn, block_name)?;
    if let Some(b) = block {
        let attributes = get_block_attributes(conn, b.id)?;
        let supports = get_block_supports(conn, b.id)?;
        let variations = get_block_variations(conn, b.id)?;
        let examples = get_block_markup(conn, block_name, MarkupOpts { features: None, validated_only: false })?;

        let source_name: String = conn.query_row(
            "SELECT name FROM sources WHERE id = ?1",
            params![b.source_id],
            |row| row.get(0),
        )?;

        Ok(Some(FullBlockSchema {
            block: b,
            attributes,
            supports,
            variations,
            examples,
            source_name,
        }))
    } else {
        Ok(None)
    }
}
