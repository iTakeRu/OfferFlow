use rusqlite::{params, Connection};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Db(pub Mutex<Connection>);

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS companies (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  industry TEXT,
  city TEXT,
  website TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS jobs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  company_id INTEGER NOT NULL REFERENCES companies(id) ON DELETE CASCADE,
  title TEXT NOT NULL,
  department TEXT,
  location TEXT,
  category TEXT,
  jd_url TEXT,
  jd_text TEXT,
  salary_min INTEGER,
  salary_max INTEGER,
  created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS stages (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  sort_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS applications (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  channel TEXT,
  resume_version TEXT,
  stage_id INTEGER NOT NULL REFERENCES stages(id),
  status TEXT NOT NULL DEFAULT '推进中',
  applied_at TEXT,
  notes TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS interviews (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  application_id INTEGER NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
  round INTEGER NOT NULL DEFAULT 1,
  format TEXT,
  location TEXT,
  interviewer TEXT,
  scheduled_at TEXT,
  rating INTEGER,
  status TEXT NOT NULL DEFAULT '待面试',
  feedback TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS attachments (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  interview_id INTEGER NOT NULL REFERENCES interviews(id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  filename TEXT NOT NULL,
  filepath TEXT NOT NULL,
  size INTEGER NOT NULL DEFAULT 0,
  duration REAL,
  created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS audio_marks (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  interview_id INTEGER NOT NULL REFERENCES interviews(id) ON DELETE CASCADE,
  attachment_id INTEGER NOT NULL REFERENCES attachments(id) ON DELETE CASCADE,
  position REAL NOT NULL,
  label TEXT NOT NULL,
  note TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
CREATE TABLE IF NOT EXISTS resumes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  filename TEXT NOT NULL,
  filepath TEXT NOT NULL,
  size INTEGER NOT NULL DEFAULT 0,
  tags TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);

CREATE TABLE IF NOT EXISTS activity_logs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  application_id INTEGER NOT NULL REFERENCES applications(id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  detail TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
"#;

const DEFAULT_STAGES: [&str; 7] = ["待投递", "简历筛选", "笔试", "一面", "二面", "HR面", "Offer"];

pub fn default_db_dir() -> PathBuf {
    let preferred = PathBuf::from("D:/OfferFlow/data");
    if preferred.exists() || fs::create_dir_all(&preferred).is_ok() {
        return preferred;
    }
    if let Some(appdata) = dirs::data_dir() {
        let alt = appdata.join("offerflow");
        let _ = fs::create_dir_all(&alt);
        return alt;
    }
    preferred
}

pub fn attachment_dir(interview_id: i64) -> PathBuf {
    default_db_dir()
        .join("attachments")
        .join(interview_id.to_string())
}

pub fn init(db_path: &Path) -> Result<Connection, String> {
    apply_pending_restore(db_path)?;
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA foreign_keys = ON;").map_err(|e| e.to_string())?;
    conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
    seed_default_stages(&conn)?;
    ensure_pending_apply_stage(&conn)?;
    ensure_job_category_column(&conn)?;
    ensure_resume_tags_column(&conn)?;
    Ok(conn)
}

fn seed_default_stages(conn: &Connection) -> Result<(), String> {
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM stages", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if count == 0 {
        for (i, name) in DEFAULT_STAGES.iter().enumerate() {
            conn.execute(
                "INSERT INTO stages (name, sort_order) VALUES (?1, ?2)",
                params![name, i as i64],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn ensure_pending_apply_stage(conn: &Connection) -> Result<(), String> {
    let exists: i64 = conn
        .query_row("SELECT COUNT(*) FROM stages WHERE name = '待投递'", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if exists == 0 {
        let min_order: i64 = conn
            .query_row("SELECT COALESCE(MIN(sort_order), 0) FROM stages", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO stages (name, sort_order) VALUES ('待投递', ?1)",
            params![min_order - 1],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}


fn ensure_job_category_column(conn: &Connection) -> Result<(), String> {
    let has: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('jobs') WHERE name = 'category'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if has == 0 {
        conn.execute("ALTER TABLE jobs ADD COLUMN category TEXT", [])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn apply_pending_restore(db_path: &Path) -> Result<(), String> {
    let dir = match db_path.parent() {
        Some(d) => d,
        None => return Ok(()),
    };
    let pending = dir.join("offerflow_restore.db");
    if !pending.exists() {
        return Ok(());
    }
    if db_path.exists() {
        let bak = dir.join("offerflow.db.bak");
        let _ = std::fs::remove_file(&bak);
        std::fs::rename(db_path, &bak).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&pending, db_path).map_err(|e| e.to_string())?;
    Ok(())
}
fn ensure_resume_tags_column(conn: &Connection) -> Result<(), String> {
    let has: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('resumes') WHERE name = 'tags'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if has == 0 {
        conn.execute("ALTER TABLE resumes ADD COLUMN tags TEXT", [])
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
