use rusqlite::{params, Connection, OpenFlags};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

pub struct RecoveringMutex<T>(Mutex<T>);
pub struct RecoveringGuard<'a, T>(MutexGuard<'a, T>);

impl<T> RecoveringMutex<T> {
    fn new(value: T) -> Self {
        Self(Mutex::new(value))
    }
    pub fn lock(&self) -> RecoveringGuard<'_, T> {
        RecoveringGuard(
            self.0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    }
}

impl<T> std::ops::Deref for RecoveringGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> std::ops::DerefMut for RecoveringGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl<T> RecoveringGuard<'_, T> {
    pub fn unwrap(self) -> Self {
        self
    }
}

pub struct Db(pub RecoveringMutex<Connection>);
impl Db {
    pub fn new(conn: Connection) -> Self {
        Self(RecoveringMutex::new(conn))
    }
}

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
CREATE TABLE IF NOT EXISTS job_snapshots (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  source_url TEXT,
  source_kind TEXT NOT NULL,
  text TEXT NOT NULL,
  html_path TEXT,
  captured_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
);
CREATE INDEX IF NOT EXISTS idx_jobs_company ON jobs(company_id);
CREATE INDEX IF NOT EXISTS idx_job_snapshots_job ON job_snapshots(job_id, id DESC);
CREATE INDEX IF NOT EXISTS idx_applications_job ON applications(job_id);
CREATE INDEX IF NOT EXISTS idx_applications_stage ON applications(stage_id);
CREATE INDEX IF NOT EXISTS idx_interviews_application ON interviews(application_id);
CREATE INDEX IF NOT EXISTS idx_interviews_scheduled ON interviews(scheduled_at);
CREATE INDEX IF NOT EXISTS idx_attachments_interview ON attachments(interview_id);
CREATE INDEX IF NOT EXISTS idx_audio_marks_interview ON audio_marks(interview_id, position);
CREATE INDEX IF NOT EXISTS idx_activity_application ON activity_logs(application_id, id DESC);
"#;

const DEFAULT_STAGES: [&str; 7] = [
    "待投递",
    "简历筛选",
    "笔试",
    "一面",
    "二面",
    "HR面",
    "Offer",
];
/// 1：初始版本；2：新增 job_snapshots，旧的 jobs.jd_text 迁为快照。
pub const SCHEMA_VERSION: i64 = 2;

fn writable_dir(path: &Path) -> bool {
    if fs::create_dir_all(path).is_err() {
        return false;
    }
    let probe = path.join(".offerflow-write-test");
    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

fn app_data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("offerflow")
}

/// 数据目录优先级：环境变量 > 可执行文件旁**已存在**的 `data/`（绿色版）> 系统应用数据目录。
///
/// 绿色版目录必须是「已经存在」才采用，不能见到可写就用：否则数据目录会跟着 exe
/// 的位置走，从 `target/release/` 和 `portable/` 各跑一次就会各自生成一份数据、
/// 之后各写各的。绿色版用法是把 `data/` 和 exe 放在一起带走，这个判断正好对应。
pub fn default_db_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("OFFERFLOW_DATA_DIR").map(PathBuf::from) {
        if writable_dir(&path) {
            return path;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let portable = parent.join("data");
            if portable.is_dir() && writable_dir(&portable) {
                return portable;
            }
        }
    }
    app_data_dir()
}

/// 0.1 版本把数据目录写死成了开发机上的 `D:/OfferFlow/data`。这个路径不该留在
/// 目录查找逻辑里（更不该留在权限白名单里），但老用户的数据还在那儿，
/// 所以降级成一次性迁移：只在启动时跑一次，跑完就再也不看这个路径。
const LEGACY_DATA_DIR: &str = "D:/OfferFlow/data";

/// 启动时调用一次：确定数据目录，并把 0.1 版遗留目录里的数据迁移过来。
pub fn prepare_data_dir() -> PathBuf {
    let target = default_db_dir();
    migrate_data_dir(Path::new(LEGACY_DATA_DIR), &target);
    target
}

fn copy_tree(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        if ty.is_symlink() {
            continue;
        }
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_tree(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// 附件、简历、设置先搬，数据库最后搬：数据库文件的出现就是「迁移成功」的标记，
/// 中途失败下次启动会重来，不会留下半套数据。
fn migrate_data_dir(legacy: &Path, target: &Path) {
    let marker = legacy.join(".migrated");
    // 只迁一次：标记文件在，就说明数据已经搬到别处并且可能已经被改过，
    // 再搬一次就会用旧数据盖掉新数据。
    if legacy == target || marker.exists() || !legacy.join("offerflow.db").exists() {
        return;
    }
    if target.join("offerflow.db").exists() || fs::create_dir_all(target).is_err() {
        return;
    }
    for dir in ["attachments", "resumes"] {
        let from = legacy.join(dir);
        if from.exists() && copy_tree(&from, &target.join(dir)).is_err() {
            return;
        }
    }
    for file in ["settings.json", "settings.secrets.json"] {
        let from = legacy.join(file);
        if from.exists() && fs::copy(&from, target.join(file)).is_err() {
            return;
        }
    }
    // WAL / SHM 先于主库落位，SQLite 打开时才能重放出最后几笔提交。
    for suffix in ["-wal", "-shm"] {
        let from = legacy.join(format!("offerflow.db{suffix}"));
        if from.exists() {
            let _ = fs::copy(&from, target.join(format!("offerflow.db{suffix}")));
        }
    }
    if fs::copy(legacy.join("offerflow.db"), target.join("offerflow.db")).is_err() {
        // 主库没搬成功就把半套残留清掉，下次启动重试。
        for suffix in ["-wal", "-shm"] {
            let _ = fs::remove_file(target.join(format!("offerflow.db{suffix}")));
        }
        return;
    }
    let _ = fs::write(
        &marker,
        format!("migrated to {}\n", target.display()).as_bytes(),
    );
}

pub fn attachment_dir(interview_id: i64) -> PathBuf {
    default_db_dir()
        .join("attachments")
        .join(interview_id.to_string())
}

pub fn init(db_path: &Path) -> Result<Connection, String> {
    // 恢复失败时必须把 pending 文件挪走：否则下次启动会走同一条失败路径，
    // 应用将永久打不开，只能让用户手动去数据目录删文件。
    if let Err(e) = apply_pending_restore(db_path) {
        quarantine_pending_restore(db_path);
        return Err(format!(
            "{e}（待恢复文件已移到 .failed 备份，应用将使用原有数据）"
        ));
    }
    let mut conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    conn.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
        .map_err(|e| e.to_string())?;
    let current_version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if current_version > SCHEMA_VERSION {
        return Err(format!(
            "数据库版本 {current_version} 高于当前应用支持的 {SCHEMA_VERSION}，请升级应用"
        ));
    }
    conn.execute_batch(SCHEMA).map_err(|e| e.to_string())?;
    {
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        seed_default_stages(&tx)?;
        ensure_pending_apply_stage(&tx)?;
        ensure_job_category_column(&tx)?;
        ensure_resume_tags_column(&tx)?;
        if current_version < 2 {
            migrate_jd_text_to_snapshots(&tx)?;
        }
        tx.pragma_update(None, "user_version", SCHEMA_VERSION)
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    repair_managed_file_paths(&conn, db_path.parent().unwrap_or_else(|| Path::new(".")))?;
    Ok(conn)
}

/// 旧版把 JD 文本直接存在 jobs.jd_text 里；升级时各转成一份快照，
/// 抓取时间取岗位创建时间，来源标记为 legacy。原列保留不动。
fn migrate_jd_text_to_snapshots(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "INSERT INTO job_snapshots (job_id, source_url, source_kind, text, captured_at) \
         SELECT id, jd_url, 'legacy', jd_text, created_at FROM jobs \
         WHERE jd_text IS NOT NULL AND trim(jd_text) != '' \
           AND NOT EXISTS (SELECT 1 FROM job_snapshots s WHERE s.job_id = jobs.id)",
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
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
        .query_row(
            "SELECT COUNT(*) FROM stages WHERE name = '待投递'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if exists == 0 {
        let min_order: i64 = conn
            .query_row("SELECT COALESCE(MIN(sort_order), 0) FROM stages", [], |r| {
                r.get(0)
            })
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

pub fn validate_backup_database(path: &Path) -> Result<(), String> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("无法打开备份数据库：{e}"))?;
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| format!("无法检查备份完整性：{e}"))?;
    if integrity != "ok" {
        return Err(format!("备份数据库完整性检查失败：{integrity}"));
    }
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if version > SCHEMA_VERSION {
        return Err(format!(
            "备份来自更高版本（{version}），当前仅支持 {SCHEMA_VERSION}"
        ));
    }
    for table in [
        "companies",
        "jobs",
        "stages",
        "applications",
        "interviews",
        "attachments",
        "resumes",
    ] {
        let exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                params![table],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if exists == 0 {
            return Err(format!("备份数据库缺少必要数据表：{table}"));
        }
    }
    let fk_error: Option<String> = conn
        .query_row(
            "SELECT 'foreign key violation' FROM pragma_foreign_key_check LIMIT 1",
            [],
            |r| r.get(0),
        )
        .ok();
    if fk_error.is_some() {
        return Err("备份数据库存在外键错误".to_string());
    }
    Ok(())
}

fn replace_pending_dir(dir: &Path, name: &str) -> Result<bool, String> {
    let pending = dir.join(format!("restore_{name}"));
    if !pending.exists() {
        return Ok(false);
    }
    let current = dir.join(name);
    let backup = dir.join(format!("{name}.bak"));
    if backup.exists() {
        fs::remove_dir_all(&backup).map_err(|e| e.to_string())?;
    }
    if current.exists() {
        fs::rename(&current, &backup).map_err(|e| e.to_string())?;
    }
    if let Err(e) = fs::rename(&pending, &current) {
        if backup.exists() {
            let _ = fs::rename(&backup, &current);
        }
        return Err(e.to_string());
    }
    Ok(true)
}

fn rollback_dir(dir: &Path, name: &str) {
    let current = dir.join(name);
    let backup = dir.join(format!("{name}.bak"));
    if current.exists() {
        let _ = fs::remove_dir_all(&current);
    }
    if backup.exists() {
        let _ = fs::rename(backup, current);
    }
}

fn replace_pending_file(
    dir: &Path,
    pending_name: &str,
    current_name: &str,
) -> Result<bool, String> {
    let pending = dir.join(pending_name);
    if !pending.exists() {
        return Ok(false);
    }
    let current = dir.join(current_name);
    let backup = dir.join(format!("{current_name}.bak"));
    if backup.exists() {
        fs::remove_file(&backup).map_err(|e| e.to_string())?;
    }
    if current.exists() {
        fs::rename(&current, &backup).map_err(|e| e.to_string())?;
    }
    if let Err(e) = fs::rename(&pending, &current) {
        if backup.exists() {
            let _ = fs::rename(&backup, &current);
        }
        return Err(e.to_string());
    }
    Ok(true)
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
    validate_backup_database(&pending)?;
    if db_path.exists() {
        let bak = dir.join("offerflow.db.bak");
        if bak.exists() {
            fs::remove_file(&bak).map_err(|e| e.to_string())?;
        }
        fs::rename(db_path, &bak).map_err(|e| e.to_string())?;
        if let Err(e) = fs::rename(&pending, db_path) {
            let _ = fs::rename(&bak, db_path);
            return Err(e.to_string());
        }
    } else {
        fs::rename(&pending, db_path).map_err(|e| e.to_string())?;
    }
    let attachments_replaced = match replace_pending_dir(dir, "attachments") {
        Ok(v) => v,
        Err(e) => {
            rollback_database(db_path);
            return Err(e);
        }
    };
    let resumes_replaced = match replace_pending_dir(dir, "resumes") {
        Ok(v) => v,
        Err(e) => {
            if attachments_replaced {
                rollback_dir(dir, "attachments");
            }
            rollback_database(db_path);
            return Err(e);
        }
    };
    let snapshots_replaced = match replace_pending_dir(dir, "snapshots") {
        Ok(v) => v,
        Err(e) => {
            if resumes_replaced {
                rollback_dir(dir, "resumes");
            }
            if attachments_replaced {
                rollback_dir(dir, "attachments");
            }
            rollback_database(db_path);
            return Err(e);
        }
    };
    if let Err(e) = replace_pending_file(dir, "settings.restore.json", "settings.json") {
        if snapshots_replaced {
            rollback_dir(dir, "snapshots");
        }
        if resumes_replaced {
            rollback_dir(dir, "resumes");
        }
        if attachments_replaced {
            rollback_dir(dir, "attachments");
        }
        rollback_database(db_path);
        return Err(e);
    }
    Ok(())
}

/// 把失败的待恢复文件改名归档，保证下次启动能正常打开。
fn quarantine_pending_restore(db_path: &Path) {
    let Some(dir) = db_path.parent() else { return };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_millis())
        .unwrap_or_default();
    for name in [
        "offerflow_restore.db",
        "restore_attachments",
        "restore_resumes",
        "restore_snapshots",
        "settings.restore.json",
    ] {
        let pending = dir.join(name);
        if pending.exists() {
            let _ = fs::rename(&pending, dir.join(format!("{name}.failed-{stamp}")));
        }
    }
}

fn rollback_database(db_path: &Path) {
    let Some(dir) = db_path.parent() else {
        return;
    };
    let backup = dir.join("offerflow.db.bak");
    let _ = fs::remove_file(db_path);
    if backup.exists() {
        let _ = fs::rename(backup, db_path);
    }
}

/// 简历 / 附件的绝对路径随数据目录走（绿色版换机器、换盘符都会变），
/// 启动时按当前数据目录重新拼一次；路径没变的行不写库。
fn repair_managed_file_paths(conn: &Connection, dir: &Path) -> Result<(), String> {
    let mut stmt = conn
        .prepare("SELECT id, filepath FROM resumes")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(|e| e.to_string())?;
    let items: Vec<_> = rows.filter_map(Result::ok).collect();
    drop(stmt);
    for (id, old) in items {
        if let Some(name) = Path::new(&old).file_name() {
            let new = dir.join("resumes").join(name).to_string_lossy().to_string();
            if new == old {
                continue;
            }
            conn.execute(
                "UPDATE resumes SET filepath=?1 WHERE id=?2",
                params![new, id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    let mut stmt = conn
        .prepare("SELECT id, interview_id, filepath FROM attachments")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let items: Vec<_> = rows.filter_map(Result::ok).collect();
    drop(stmt);
    for (id, interview_id, old) in items {
        if let Some(name) = Path::new(&old).file_name() {
            let new = dir
                .join("attachments")
                .join(interview_id.to_string())
                .join(name)
                .to_string_lossy()
                .to_string();
            if new == old {
                continue;
            }
            conn.execute(
                "UPDATE attachments SET filepath=?1 WHERE id=?2",
                params![new, id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
    let mut stmt = conn
        .prepare("SELECT id, job_id, html_path FROM job_snapshots WHERE html_path IS NOT NULL")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?;
    let items: Vec<_> = rows.filter_map(Result::ok).collect();
    drop(stmt);
    for (id, job_id, old) in items {
        if let Some(name) = Path::new(&old).file_name() {
            let new = dir
                .join("snapshots")
                .join(job_id.to_string())
                .join(name)
                .to_string_lossy()
                .to_string();
            if new == old {
                continue;
            }
            conn.execute(
                "UPDATE job_snapshots SET html_path=?1 WHERE id=?2",
                params![new, id],
            )
            .map_err(|e| e.to_string())?;
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "offerflow-{name}-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn initialized_database_is_a_valid_backup() {
        let path = temp_db("valid");
        let conn = init(&path).unwrap();
        drop(conn);
        validate_backup_database(&path).unwrap();
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("db-wal"));
        let _ = fs::remove_file(path.with_extension("db-shm"));
    }

    #[test]
    fn migrates_legacy_data_dir_exactly_once() {
        let base = std::env::temp_dir().join(format!(
            "offerflow-migrate-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let legacy = base.join("legacy");
        let target = base.join("target");
        fs::create_dir_all(legacy.join("resumes")).unwrap();
        drop(init(&legacy.join("offerflow.db")).unwrap());
        fs::write(legacy.join("resumes/a.pdf"), b"resume").unwrap();
        fs::write(legacy.join("settings.json"), b"{}").unwrap();

        migrate_data_dir(&legacy, &target);
        assert!(target.join("offerflow.db").exists());
        assert!(target.join("resumes/a.pdf").exists());
        assert!(target.join("settings.json").exists());
        assert!(legacy.join("offerflow.db").exists(), "原目录必须原样保留");

        // 迁移后目标侧的改动不能被第二次迁移盖掉。
        fs::write(target.join("resumes/a.pdf"), b"edited").unwrap();
        fs::remove_file(target.join("offerflow.db")).unwrap();
        migrate_data_dir(&legacy, &target);
        assert!(
            !target.join("offerflow.db").exists(),
            "标记存在时不应再迁移"
        );
        assert_eq!(fs::read(target.join("resumes/a.pdf")).unwrap(), b"edited");

        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn upgrades_v1_jd_text_into_snapshots_once() {
        let path = temp_db("upgrade");
        {
            let conn = init(&path).unwrap();
            conn.execute("INSERT INTO companies(name) VALUES('Example')", [])
                .unwrap();
            conn.execute(
                "INSERT INTO jobs(company_id,title,jd_url,jd_text) VALUES(1,'A','https://a.example/1','JD 原文')",
                [],
            )
            .unwrap();
            conn.execute("INSERT INTO jobs(company_id,title) VALUES(1,'B')", [])
                .unwrap();
            // 模拟一个还停留在 v1、没有快照的旧库
            conn.execute("DELETE FROM job_snapshots", []).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
        }
        for _ in 0..2 {
            let conn = init(&path).unwrap();
            let rows: Vec<(i64, String, String)> = conn
                .prepare("SELECT job_id, source_kind, text FROM job_snapshots")
                .unwrap()
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
                .unwrap()
                .map(Result::unwrap)
                .collect();
            assert_eq!(rows, vec![(1, "legacy".to_string(), "JD 原文".to_string())]);
        }
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn rejects_corrupt_backup() {
        let path = temp_db("corrupt");
        fs::write(&path, b"not a sqlite database").unwrap();
        assert!(validate_backup_database(&path).is_err());
        let _ = fs::remove_file(path);
    }

    #[test]
    fn deleting_application_cascades_interview_metadata() {
        let path = temp_db("cascade");
        let conn = init(&path).unwrap();
        conn.execute("INSERT INTO companies(name) VALUES('Example')", [])
            .unwrap();
        let company = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO jobs(company_id,title) VALUES(?1,'Engineer')",
            params![company],
        )
        .unwrap();
        let job = conn.last_insert_rowid();
        let stage: i64 = conn
            .query_row("SELECT id FROM stages LIMIT 1", [], |r| r.get(0))
            .unwrap();
        conn.execute(
            "INSERT INTO applications(job_id,stage_id) VALUES(?1,?2)",
            params![job, stage],
        )
        .unwrap();
        let application = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO interviews(application_id) VALUES(?1)",
            params![application],
        )
        .unwrap();
        let interview = conn.last_insert_rowid();
        conn.execute("INSERT INTO attachments(interview_id,kind,filename,filepath) VALUES(?1,'audio','a.mp3','a.mp3')", params![interview]).unwrap();
        conn.execute("DELETE FROM applications WHERE id=?1", params![application])
            .unwrap();
        let interviews: i64 = conn
            .query_row("SELECT COUNT(*) FROM interviews", [], |r| r.get(0))
            .unwrap();
        let attachments: i64 = conn
            .query_row("SELECT COUNT(*) FROM attachments", [], |r| r.get(0))
            .unwrap();
        assert_eq!((interviews, attachments), (0, 0));
        drop(conn);
        let _ = fs::remove_file(path);
    }
}
