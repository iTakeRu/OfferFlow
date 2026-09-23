#![allow(clippy::too_many_arguments)] // Tauri command fields are mapped from IPC arguments.

use crate::db::Db;
use crate::models::*;
use rusqlite::{params, Row};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use tauri_plugin_opener::OpenerExt;

fn require_text(value: String, field: &str) -> Result<String, String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        return Err(format!("{field}不能为空"));
    }
    if value.chars().count() > 200 {
        return Err(format!("{field}过长"));
    }
    Ok(value)
}

fn validate_rating(value: Option<i64>) -> Result<(), String> {
    if value.is_some_and(|v| !(1..=5).contains(&v)) {
        return Err("评分必须在 1 到 5 之间".to_string());
    }
    Ok(())
}

fn validate_salary(min: Option<i64>, max: Option<i64>) -> Result<(), String> {
    if min.is_some_and(|v| v < 0) || max.is_some_and(|v| v < 0) {
        return Err("薪资不能为负数".to_string());
    }
    if let (Some(a), Some(b)) = (min, max) {
        if a > b {
            return Err("薪资下限不能高于上限".to_string());
        }
    }
    Ok(())
}

fn validate_application_status(value: &str) -> Result<(), String> {
    if ["推进中", "已获Offer", "未通过筛选", "无回应超时"].contains(&value) {
        Ok(())
    } else {
        Err("投递状态无效".to_string())
    }
}

fn validate_interview_status(value: &str) -> Result<(), String> {
    if ["待面试", "已完成", "已取消"].contains(&value) {
        Ok(())
    } else {
        Err("面试状态无效".to_string())
    }
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        _ => 0,
    }
}

fn validate_date(value: &Option<String>, field: &str) -> Result<(), String> {
    let Some(v) = value else { return Ok(()) };
    let b = v.as_bytes();
    let well_formed = b.len() >= 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[..4]
            .iter()
            .chain(&b[5..7])
            .chain(&b[8..10])
            .all(u8::is_ascii_digit);
    if !well_formed {
        return Err(format!("{field}格式无效"));
    }
    let num = |range: std::ops::Range<usize>| -> i64 { v[range].parse().unwrap_or(0) };
    let (year, month, day) = (num(0..4), num(5..7), num(8..10));
    if !(1900..=2999).contains(&year) || day < 1 || day > days_in_month(year, month) {
        return Err(format!("{field}不是有效日期"));
    }
    Ok(())
}

/// SQLite 的外键报错直接抛给用户是天书，这里翻成能看懂的提示。
fn friendly_db_error(err: rusqlite::Error, context: &str) -> String {
    let text = err.to_string();
    if text.contains("FOREIGN KEY") {
        return context.to_string();
    }
    text
}

pub(crate) fn remove_managed_files(paths: Vec<String>) {
    remove_files_under_root(paths, crate::db::default_db_dir());
}

fn remove_files_under_root(paths: Vec<String>, root: std::path::PathBuf) {
    let canonical_root = root.canonicalize().unwrap_or(root);
    for raw in paths {
        let path = std::path::PathBuf::from(raw);
        if let Ok(canonical) = path.canonicalize() {
            if canonical.starts_with(&canonical_root) {
                let _ = std::fs::remove_file(&canonical);
                if let Some(parent) = canonical.parent() {
                    let _ = std::fs::remove_dir(parent);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_impossible_dates() {
        assert!(validate_date(&Some("2026-02-30".into()), "日期").is_err());
        assert!(validate_date(&Some("2026-13-01".into()), "日期").is_err());
        assert!(validate_date(&Some("2026-9-01".into()), "日期").is_err());
        assert!(validate_date(&Some("2024-02-29".into()), "日期").is_ok());
        assert!(validate_date(&Some("2026-02-28T10:00".into()), "日期").is_ok());
        assert!(validate_date(&None, "日期").is_ok());
    }

    #[test]
    fn truncates_oversized_cells() {
        let long = "字".repeat(MAX_CELL_CHARS + 10);
        assert_eq!(cell(&long).chars().count(), MAX_CELL_CHARS);
        assert_eq!(cell("短文本"), "短文本");
    }

    #[test]
    fn strips_tags_and_decodes_entities() {
        assert_eq!(strip_html("<p>A&amp;B</p><li>C</li>"), "A&BC");
    }

    #[test]
    fn managed_cleanup_cannot_escape_root() {
        let base = std::env::temp_dir().join(format!("offerflow-cleanup-{}", std::process::id()));
        let root = base.join("data");
        std::fs::create_dir_all(root.join("attachments/1")).unwrap();
        let inside = root.join("attachments/1/recording.mp3");
        let outside = base.join("keep.txt");
        std::fs::write(&inside, b"audio").unwrap();
        std::fs::write(&outside, b"keep").unwrap();
        remove_files_under_root(
            vec![
                inside.to_string_lossy().to_string(),
                outside.to_string_lossy().to_string(),
            ],
            root,
        );
        assert!(!inside.exists());
        assert!(outside.exists());
        let _ = std::fs::remove_dir_all(base);
    }
}

fn validate_import_source<'a>(
    path: &'a Path,
    allowed: &[&str],
    max_bytes: u64,
) -> Result<&'a Path, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("只能导入普通文件".to_string());
    }
    if meta.len() > max_bytes {
        return Err("导入文件超过大小限制".to_string());
    }
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !allowed.contains(&ext.as_str()) {
        return Err(format!("不支持的文件类型：.{ext}"));
    }
    Ok(path)
}

fn attachment_paths(
    conn: &rusqlite::Connection,
    sql: &str,
    id: i64,
) -> Result<Vec<String>, String> {
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

fn row_company(r: &Row) -> rusqlite::Result<Company> {
    Ok(Company {
        id: r.get(0)?,
        name: r.get(1)?,
        industry: r.get(2)?,
        city: r.get(3)?,
        website: r.get(4)?,
        notes: r.get(5)?,
        created_at: r.get(6)?,
    })
}

fn row_job(r: &Row) -> rusqlite::Result<Job> {
    Ok(Job {
        id: r.get(0)?,
        company_id: r.get(1)?,
        title: r.get(2)?,
        department: r.get(3)?,
        location: r.get(4)?,
        category: r.get(5)?,
        jd_url: r.get(6)?,
        jd_text: r.get(7)?,
        salary_min: r.get(8)?,
        salary_max: r.get(9)?,
        created_at: r.get(10)?,
    })
}

fn row_stage(r: &Row) -> rusqlite::Result<Stage> {
    Ok(Stage {
        id: r.get(0)?,
        name: r.get(1)?,
        sort_order: r.get(2)?,
    })
}

fn row_application(r: &Row) -> rusqlite::Result<Application> {
    Ok(Application {
        id: r.get(0)?,
        job_id: r.get(1)?,
        channel: r.get(2)?,
        resume_version: r.get(3)?,
        stage_id: r.get(4)?,
        status: r.get(5)?,
        applied_at: r.get(6)?,
        notes: r.get(7)?,
        created_at: r.get(8)?,
        updated_at: r.get(9)?,
    })
}

fn row_interview(r: &Row) -> rusqlite::Result<Interview> {
    Ok(Interview {
        id: r.get(0)?,
        application_id: r.get(1)?,
        round: r.get(2)?,
        format: r.get(3)?,
        location: r.get(4)?,
        interviewer: r.get(5)?,
        scheduled_at: r.get(6)?,
        rating: r.get(7)?,
        status: r.get(8)?,
        feedback: r.get(9)?,
        created_at: r.get(10)?,
    })
}

fn row_attachment(r: &Row) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: r.get(0)?,
        interview_id: r.get(1)?,
        kind: r.get(2)?,
        filename: r.get(3)?,
        filepath: r.get(4)?,
        size: r.get(5)?,
        duration: r.get(6)?,
        created_at: r.get(7)?,
    })
}

fn row_mark(r: &Row) -> rusqlite::Result<AudioMark> {
    Ok(AudioMark {
        id: r.get(0)?,
        interview_id: r.get(1)?,
        attachment_id: r.get(2)?,
        position: r.get(3)?,
        label: r.get(4)?,
        note: r.get(5)?,
        created_at: r.get(6)?,
    })
}

// ---------- Stage ----------
#[tauri::command]
pub fn list_stages(state: State<Db>) -> Result<Vec<Stage>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, sort_order FROM stages ORDER BY sort_order, id")
        .map_err(|x| x.to_string())?;
    let rows = stmt.query_map([], row_stage).map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn create_stage(state: State<Db>, name: String) -> Result<Stage, String> {
    let name = require_text(name, "阶段名称")?;
    let conn = state.0.lock().unwrap();
    let max_order: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) FROM stages",
            [],
            |r| r.get(0),
        )
        .map_err(|x| x.to_string())?;
    conn.execute(
        "INSERT INTO stages (name, sort_order) VALUES (?1, ?2)",
        params![name, max_order + 1],
    )
    .map_err(|x| x.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, sort_order FROM stages WHERE id = ?1",
        params![id],
        row_stage,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn update_stage(state: State<Db>, id: i64, name: String) -> Result<Stage, String> {
    let name = require_text(name, "阶段名称")?;
    let conn = state.0.lock().unwrap();
    conn.execute(
        "UPDATE stages SET name = ?1 WHERE id = ?2",
        params![name, id],
    )
    .map_err(|x| x.to_string())?;
    conn.query_row(
        "SELECT id, name, sort_order FROM stages WHERE id = ?1",
        params![id],
        row_stage,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn delete_stage(state: State<Db>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let in_use: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM applications WHERE stage_id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|x| x.to_string())?;
    if in_use > 0 {
        return Err(format!("该阶段下还有 {in_use} 条投递，请先移走再删除"));
    }
    let remaining: i64 = conn
        .query_row("SELECT COUNT(*) FROM stages", [], |r| r.get(0))
        .map_err(|x| x.to_string())?;
    if remaining <= 1 {
        return Err("至少需要保留一个阶段".to_string());
    }
    conn.execute("DELETE FROM stages WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn reorder_stages(state: State<Db>, ids: Vec<i64>) -> Result<(), String> {
    let mut conn = state.0.lock().unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM stages", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if ids.len() as i64 != count {
        return Err("阶段排序列表不完整".to_string());
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    for (i, id) in ids.iter().enumerate() {
        if tx
            .execute(
                "UPDATE stages SET sort_order = ?1 WHERE id = ?2",
                params![i as i64, id],
            )
            .map_err(|x| x.to_string())?
            != 1
        {
            return Err("阶段不存在".to_string());
        }
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- Company ----------
#[tauri::command]
pub fn list_companies(state: State<Db>) -> Result<Vec<Company>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, industry, city, website, notes, created_at FROM companies ORDER BY name")
        .map_err(|x| x.to_string())?;
    let rows = stmt.query_map([], row_company).map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn create_company(
    state: State<Db>,
    name: String,
    industry: Option<String>,
    city: Option<String>,
    website: Option<String>,
    notes: Option<String>,
) -> Result<Company, String> {
    let name = require_text(name, "公司名称")?;
    let conn = state.0.lock().unwrap();
    let duplicate: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM companies WHERE lower(trim(name))=lower(trim(?1))",
            params![name],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if duplicate > 0 {
        return Err("已存在同名公司".to_string());
    }
    conn.execute(
        "INSERT INTO companies (name, industry, city, website, notes) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, industry, city, website, notes],
    )
    .map_err(|x| x.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, industry, city, website, notes, created_at FROM companies WHERE id = ?1",
        params![id],
        row_company,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn update_company(
    state: State<Db>,
    id: i64,
    name: String,
    industry: Option<String>,
    city: Option<String>,
    website: Option<String>,
    notes: Option<String>,
) -> Result<Company, String> {
    let name = require_text(name, "公司名称")?;
    let conn = state.0.lock().unwrap();
    conn.execute(
        "UPDATE companies SET name = ?1, industry = ?2, city = ?3, website = ?4, notes = ?5 WHERE id = ?6",
        params![name, industry, city, website, notes, id],
    )
    .map_err(|x| x.to_string())?;
    conn.query_row(
        "SELECT id, name, industry, city, website, notes, created_at FROM companies WHERE id = ?1",
        params![id],
        row_company,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn delete_company(state: State<Db>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let mut paths = attachment_paths(&conn, "SELECT at.filepath FROM attachments at JOIN interviews i ON i.id=at.interview_id JOIN applications a ON a.id=i.application_id JOIN jobs j ON j.id=a.job_id WHERE j.company_id=?1", id)?;
    paths.extend(attachment_paths(&conn, "SELECT s.html_path FROM job_snapshots s JOIN jobs j ON j.id=s.job_id WHERE j.company_id=?1 AND s.html_path IS NOT NULL", id)?);
    conn.execute("DELETE FROM companies WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    drop(conn);
    remove_managed_files(paths);
    Ok(())
}

// ---------- Job ----------
#[tauri::command]
pub fn list_jobs(state: State<Db>, company_id: Option<i64>) -> Result<Vec<Job>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, company_id, title, department, location, category, jd_url, jd_text, salary_min, salary_max, created_at FROM jobs WHERE (?1 IS NULL OR company_id = ?1) ORDER BY id DESC")
        .map_err(|x| x.to_string())?;
    let rows = stmt
        .query_map(params![company_id], row_job)
        .map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn create_job(
    state: State<Db>,
    company_id: i64,
    title: String,
    department: Option<String>,
    location: Option<String>,
    category: Option<String>,
    jd_url: Option<String>,
    jd_text: Option<String>,
    salary_min: Option<i64>,
    salary_max: Option<i64>,
) -> Result<Job, String> {
    let title = require_text(title, "岗位名称")?;
    validate_salary(salary_min, salary_max)?;
    let conn = state.0.lock().unwrap();
    let duplicate: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM jobs WHERE company_id=?1 AND lower(trim(title))=lower(trim(?2))",
            params![company_id, title],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if duplicate > 0 {
        return Err("该公司下已存在同名岗位".to_string());
    }
    conn.execute(
        "INSERT INTO jobs (company_id, title, department, location, category, jd_url, jd_text, salary_min, salary_max) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![company_id, title, department, location, category, jd_url, jd_text, salary_min, salary_max],
    )
    .map_err(|x| friendly_db_error(x, "所选公司不存在"))?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, company_id, title, department, location, category, jd_url, jd_text, salary_min, salary_max, created_at FROM jobs WHERE id = ?1",
        params![id],
        row_job,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn update_job(
    state: State<Db>,
    id: i64,
    company_id: i64,
    title: String,
    department: Option<String>,
    location: Option<String>,
    category: Option<String>,
    jd_url: Option<String>,
    jd_text: Option<String>,
    salary_min: Option<i64>,
    salary_max: Option<i64>,
) -> Result<Job, String> {
    let title = require_text(title, "岗位名称")?;
    validate_salary(salary_min, salary_max)?;
    let conn = state.0.lock().unwrap();
    conn.execute(
        "UPDATE jobs SET company_id = ?1, title = ?2, department = ?3, location = ?4, category = ?5, jd_url = ?6, jd_text = ?7, salary_min = ?8, salary_max = ?9 WHERE id = ?10",
        params![company_id, title, department, location, category, jd_url, jd_text, salary_min, salary_max, id],
    )
    .map_err(|x| x.to_string())?;
    conn.query_row(
        "SELECT id, company_id, title, department, location, category, jd_url, jd_text, salary_min, salary_max, created_at FROM jobs WHERE id = ?1",
        params![id],
        row_job,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn delete_job(state: State<Db>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let mut paths = attachment_paths(&conn, "SELECT at.filepath FROM attachments at JOIN interviews i ON i.id=at.interview_id JOIN applications a ON a.id=i.application_id WHERE a.job_id=?1", id)?;
    paths.extend(attachment_paths(
        &conn,
        "SELECT html_path FROM job_snapshots WHERE job_id=?1 AND html_path IS NOT NULL",
        id,
    )?);
    conn.execute("DELETE FROM jobs WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    drop(conn);
    remove_managed_files(paths);
    Ok(())
}
// ---------- Application ----------
#[tauri::command]
pub fn list_applications(state: State<Db>) -> Result<Vec<Application>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, job_id, channel, resume_version, stage_id, status, applied_at, notes, created_at, updated_at FROM applications ORDER BY id DESC")
        .map_err(|x| x.to_string())?;
    let rows = stmt
        .query_map([], row_application)
        .map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn create_application(
    state: State<Db>,
    job_id: i64,
    channel: Option<String>,
    resume_version: Option<String>,
    stage_id: i64,
    status: Option<String>,
    applied_at: Option<String>,
    notes: Option<String>,
) -> Result<Application, String> {
    validate_date(&applied_at, "投递日期")?;
    let status = status.unwrap_or_else(|| "推进中".to_string());
    validate_application_status(&status)?;
    let mut conn = state.0.lock().unwrap();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO applications (job_id, channel, resume_version, stage_id, status, applied_at, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![job_id, channel, resume_version, stage_id, status, applied_at, notes],
    )
    .map_err(|x| friendly_db_error(x, "所选岗位或阶段不存在，请重新选择"))?;
    let id = tx.last_insert_rowid();
    log_activity(
        &tx,
        id,
        "created",
        &format!(
            "创建投递（阶段：{}，状态：{}）",
            stage_name(&tx, stage_id),
            status
        ),
    )?;
    let result = tx.query_row(
        "SELECT id, job_id, channel, resume_version, stage_id, status, applied_at, notes, created_at, updated_at FROM applications WHERE id = ?1",
        params![id],
        row_application,
    )
    .map_err(|x| x.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub fn update_application(
    state: State<Db>,
    id: i64,
    job_id: i64,
    channel: Option<String>,
    resume_version: Option<String>,
    stage_id: i64,
    status: String,
    applied_at: Option<String>,
    notes: Option<String>,
) -> Result<Application, String> {
    validate_application_status(&status)?;
    validate_date(&applied_at, "投递日期")?;
    let mut conn = state.0.lock().unwrap();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let (old_job, old_stage, old_status): (i64, i64, String) = tx
        .query_row(
            "SELECT job_id, stage_id, status FROM applications WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|x| x.to_string())?;
    tx.execute(
        "UPDATE applications SET job_id = ?1, channel = ?2, resume_version = ?3, stage_id = ?4, status = ?5, applied_at = ?6, notes = ?7, updated_at = datetime('now','localtime') WHERE id = ?8",
        params![job_id, channel, resume_version, stage_id, status, applied_at, notes, id],
    )
    .map_err(|x| friendly_db_error(x, "所选岗位或阶段不存在，请重新选择"))?;
    if old_job != job_id {
        log_activity(
            &tx,
            id,
            "job",
            &format!(
                "岗位变更：{} → {}",
                job_label(&tx, old_job),
                job_label(&tx, job_id)
            ),
        )?;
    }
    if old_stage != stage_id {
        log_activity(
            &tx,
            id,
            "stage",
            &format!(
                "阶段变更：{} → {}",
                stage_name(&tx, old_stage),
                stage_name(&tx, stage_id)
            ),
        )?;
    }
    if old_status != status {
        log_activity(
            &tx,
            id,
            "status",
            &format!("状态变更：{} → {}", old_status, status),
        )?;
    }
    let result = tx.query_row(
        "SELECT id, job_id, channel, resume_version, stage_id, status, applied_at, notes, created_at, updated_at FROM applications WHERE id = ?1",
        params![id],
        row_application,
    )
    .map_err(|x| x.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub fn move_application(state: State<Db>, id: i64, stage_id: i64) -> Result<Application, String> {
    let mut conn = state.0.lock().unwrap();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let old_stage: i64 = tx
        .query_row(
            "SELECT stage_id FROM applications WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .map_err(|x| x.to_string())?;
    tx.execute(
        "UPDATE applications SET stage_id = ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
        params![stage_id, id],
    )
    .map_err(|x| x.to_string())?;
    if old_stage != stage_id {
        log_activity(
            &tx,
            id,
            "stage",
            &format!(
                "阶段变更：{} → {}",
                stage_name(&tx, old_stage),
                stage_name(&tx, stage_id)
            ),
        )?;
    }
    let result = tx.query_row(
        "SELECT id, job_id, channel, resume_version, stage_id, status, applied_at, notes, created_at, updated_at FROM applications WHERE id = ?1",
        params![id],
        row_application,
    )
    .map_err(|x| x.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(result)
}

#[tauri::command]
pub fn delete_application(state: State<Db>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let paths = attachment_paths(&conn, "SELECT at.filepath FROM attachments at JOIN interviews i ON i.id=at.interview_id WHERE i.application_id=?1", id)?;
    conn.execute("DELETE FROM applications WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    drop(conn);
    remove_managed_files(paths);
    Ok(())
}

// ---------- Interview ----------
#[tauri::command]
pub fn list_interviews(state: State<Db>, application_id: i64) -> Result<Vec<Interview>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, application_id, round, format, location, interviewer, scheduled_at, rating, status, feedback, created_at FROM interviews WHERE application_id = ?1 ORDER BY round")
        .map_err(|x| x.to_string())?;
    let rows = stmt
        .query_map(params![application_id], row_interview)
        .map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn create_interview(
    state: State<Db>,
    application_id: i64,
    round: i64,
    format: Option<String>,
    location: Option<String>,
    interviewer: Option<String>,
    scheduled_at: Option<String>,
    rating: Option<i64>,
    status: Option<String>,
    feedback: Option<String>,
) -> Result<Interview, String> {
    if round < 1 {
        return Err("面试轮次必须大于 0".to_string());
    }
    validate_rating(rating)?;
    validate_date(&scheduled_at, "面试日期")?;
    let conn = state.0.lock().unwrap();
    let status = status.unwrap_or_else(|| "待面试".to_string());
    validate_interview_status(&status)?;
    conn.execute(
        "INSERT INTO interviews (application_id, round, format, location, interviewer, scheduled_at, rating, status, feedback) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![application_id, round, format, location, interviewer, scheduled_at, rating, status, feedback],
    )
    .map_err(|x| x.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, application_id, round, format, location, interviewer, scheduled_at, rating, status, feedback, created_at FROM interviews WHERE id = ?1",
        params![id],
        row_interview,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn update_interview(
    state: State<Db>,
    id: i64,
    round: i64,
    format: Option<String>,
    location: Option<String>,
    interviewer: Option<String>,
    scheduled_at: Option<String>,
    rating: Option<i64>,
    status: String,
    feedback: Option<String>,
) -> Result<Interview, String> {
    if round < 1 {
        return Err("面试轮次必须大于 0".to_string());
    }
    validate_rating(rating)?;
    validate_date(&scheduled_at, "面试日期")?;
    validate_interview_status(&status)?;
    let conn = state.0.lock().unwrap();
    conn.execute(
        "UPDATE interviews SET round = ?1, format = ?2, location = ?3, interviewer = ?4, scheduled_at = ?5, rating = ?6, status = ?7, feedback = ?8 WHERE id = ?9",
        params![round, format, location, interviewer, scheduled_at, rating, status, feedback, id],
    )
    .map_err(|x| x.to_string())?;
    conn.query_row(
        "SELECT id, application_id, round, format, location, interviewer, scheduled_at, rating, status, feedback, created_at FROM interviews WHERE id = ?1",
        params![id],
        row_interview,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn delete_interview(state: State<Db>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let paths = attachment_paths(
        &conn,
        "SELECT filepath FROM attachments WHERE interview_id=?1",
        id,
    )?;
    conn.execute("DELETE FROM interviews WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    drop(conn);
    remove_managed_files(paths);
    Ok(())
}

// ---------- Attachment ----------
#[tauri::command]
pub fn list_attachments(state: State<Db>, interview_id: i64) -> Result<Vec<Attachment>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, interview_id, kind, filename, filepath, size, duration, created_at FROM attachments WHERE interview_id = ?1 ORDER BY id")
        .map_err(|x| x.to_string())?;
    let rows = stmt
        .query_map(params![interview_id], row_attachment)
        .map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn delete_attachment(state: State<Db>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let filepath: Option<String> = conn
        .query_row(
            "SELECT filepath FROM attachments WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .ok();
    conn.execute("DELETE FROM attachments WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    drop(conn);
    if let Some(fp) = filepath {
        remove_managed_files(vec![fp]);
    }
    Ok(())
}

/// 用系统默认程序打开简历 / 附件。
///
/// 走后端而不是前端的 `openPath`，是因为 opener 的能力白名单只能写死路径，
/// 一旦用户用 `OFFERFLOW_DATA_DIR` 换了数据目录，前端调用就会被权限拒掉。
/// 这里按运行时真实的数据目录做包含性校验，语义更准也更严。
#[tauri::command(async)]
pub fn open_managed_path(app: tauri::AppHandle, path: String) -> Result<(), String> {
    let requested = std::path::PathBuf::from(&path)
        .canonicalize()
        .map_err(|x| x.to_string())?;
    let root = crate::db::default_db_dir()
        .canonicalize()
        .map_err(|x| x.to_string())?;
    if !requested.starts_with(&root) {
        return Err("拒绝打开应用数据目录之外的文件".to_string());
    }
    if !requested.is_file() {
        return Err("文件不存在或已被移动".to_string());
    }
    // canonicalize 在 Windows 上会带 UNC 前缀，ShellExecute 不认，所以打开原始路径。
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn import_attachment(
    state: State<Db>,
    interview_id: i64,
    kind: String,
    source_path: String,
) -> Result<Attachment, String> {
    let src = validate_import_source(
        Path::new(&source_path),
        &[
            "mp3", "wav", "m4a", "aac", "ogg", "flac", "mp4", "webm", "pdf", "doc", "docx", "txt",
            "png", "jpg", "jpeg", "webp",
        ],
        500 * 1024 * 1024,
    )?;
    let filename = src
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "attachment".to_string());
    let dir = crate::db::attachment_dir(interview_id);
    std::fs::create_dir_all(&dir).map_err(|x| x.to_string())?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|x| x.to_string())?
        .as_millis();
    let dest = dir.join(format!("{}_{}", ts, filename));
    std::fs::copy(src, &dest).map_err(|x| x.to_string())?;
    let size = std::fs::metadata(&dest)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    let filepath = dest.to_string_lossy().to_string();
    let conn = state.0.lock().unwrap();
    if let Err(e) = conn.execute(
        "INSERT INTO attachments (interview_id, kind, filename, filepath, size, duration) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![interview_id, kind, filename, filepath, size, None::<f64>],
    ) {
        let _ = std::fs::remove_file(&dest);
        return Err(e.to_string());
    }
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, interview_id, kind, filename, filepath, size, duration, created_at FROM attachments WHERE id = ?1",
        params![id],
        row_attachment,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn set_attachment_duration(state: State<Db>, id: i64, duration: f64) -> Result<(), String> {
    if !duration.is_finite() || duration < 0.0 {
        return Err("音频时长无效".to_string());
    }
    let conn = state.0.lock().unwrap();
    conn.execute(
        "UPDATE attachments SET duration = ?1 WHERE id = ?2",
        params![duration, id],
    )
    .map_err(|x| x.to_string())?;
    Ok(())
}

// ---------- AudioMark ----------
#[tauri::command]
pub fn list_audio_marks(state: State<Db>, interview_id: i64) -> Result<Vec<AudioMark>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, interview_id, attachment_id, position, label, note, created_at FROM audio_marks WHERE interview_id = ?1 ORDER BY position")
        .map_err(|x| x.to_string())?;
    let rows = stmt
        .query_map(params![interview_id], row_mark)
        .map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn add_audio_mark(
    state: State<Db>,
    interview_id: i64,
    attachment_id: i64,
    position: f64,
    label: String,
    note: Option<String>,
) -> Result<AudioMark, String> {
    if !position.is_finite() || position < 0.0 {
        return Err("音频标记位置无效".to_string());
    }
    let label = require_text(label, "标记标签")?;
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT INTO audio_marks (interview_id, attachment_id, position, label, note) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![interview_id, attachment_id, position, label, note],
    )
    .map_err(|x| x.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, interview_id, attachment_id, position, label, note, created_at FROM audio_marks WHERE id = ?1",
        params![id],
        row_mark,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn delete_audio_mark(state: State<Db>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute("DELETE FROM audio_marks WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    Ok(())
}

// ---------- Export ----------
/// xlsx 单元格上限 32767 字符，超了整个导出会失败，这里截断保平安。
const MAX_CELL_CHARS: usize = 32_767;

fn cell(value: &str) -> String {
    if value.chars().count() <= MAX_CELL_CHARS {
        return value.to_string();
    }
    let mut out: String = value.chars().take(MAX_CELL_CHARS - 1).collect();
    out.push('…');
    out
}

fn strip_html(s: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    crate::extract::decode_entities(&out)
}

#[tauri::command(async)]
pub fn export_excel(state: State<Db>, path: String) -> Result<(), String> {
    use rust_xlsxwriter::Workbook;

    if !path.to_ascii_lowercase().ends_with(".xlsx") {
        return Err("导出文件必须使用 .xlsx 扩展名".to_string());
    }
    let conn = state.0.lock().unwrap();
    let mut wb = Workbook::new();

    // Sheet 1: 投递记录
    {
        let ws = wb.add_worksheet();
        let headers = [
            "公司",
            "岗位",
            "部门",
            "投递渠道",
            "简历版本",
            "当前阶段",
            "状态",
            "投递日期",
            "备注",
        ];
        for (c, h) in headers.iter().enumerate() {
            ws.write(0, c as u16, *h).map_err(|e| e.to_string())?;
        }
        let mut stmt = conn
            .prepare(
                "SELECT c.name, j.title, j.department, a.channel, a.resume_version, s.name, a.status, a.applied_at, a.notes \
                 FROM applications a \
                 JOIN jobs j ON j.id = a.job_id \
                 JOIN companies c ON c.id = j.company_id \
                 JOIN stages s ON s.id = a.stage_id \
                 ORDER BY c.name, j.title, a.id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, String>(6)?,
                    r.get::<_, Option<String>>(7)?,
                    r.get::<_, Option<String>>(8)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for (row, r) in (1u32..).zip(rows) {
            let (company, title, department, channel, resume, stage, status, applied_at, notes) =
                r.map_err(|e| e.to_string())?;
            let cells = [
                company.unwrap_or_default(),
                title,
                department.unwrap_or_default(),
                channel.unwrap_or_default(),
                resume.unwrap_or_default(),
                stage.unwrap_or_default(),
                status,
                applied_at.unwrap_or_default(),
                notes.unwrap_or_default(),
            ];
            for (c, value) in cells.iter().enumerate() {
                ws.write(row, c as u16, cell(value).as_str())
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    // Sheet 2: 面试记录
    {
        let ws = wb.add_worksheet();
        let headers = [
            "公司",
            "岗位",
            "轮次",
            "形式",
            "地点",
            "面试官",
            "面试日期",
            "评分",
            "状态",
            "复盘笔记",
        ];
        for (c, h) in headers.iter().enumerate() {
            ws.write(0, c as u16, *h).map_err(|e| e.to_string())?;
        }
        let mut stmt = conn
            .prepare(
                "SELECT c.name, j.title, i.round, i.format, i.location, i.interviewer, i.scheduled_at, i.rating, i.status, i.feedback \
                 FROM interviews i \
                 JOIN applications a ON a.id = i.application_id \
                 JOIN jobs j ON j.id = a.job_id \
                 JOIN companies c ON c.id = j.company_id \
                 ORDER BY c.name, j.title, i.round",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, Option<String>>(5)?,
                    r.get::<_, Option<String>>(6)?,
                    r.get::<_, Option<i64>>(7)?,
                    r.get::<_, String>(8)?,
                    r.get::<_, Option<String>>(9)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        for (row, r) in (1u32..).zip(rows) {
            let (
                company,
                title,
                round,
                format,
                location,
                interviewer,
                scheduled_at,
                rating,
                status,
                feedback,
            ) = r.map_err(|e| e.to_string())?;
            let cells = [
                company.unwrap_or_default(),
                title,
                round.to_string(),
                format.unwrap_or_default(),
                location.unwrap_or_default(),
                interviewer.unwrap_or_default(),
                scheduled_at.unwrap_or_default(),
                rating.map(|x| x.to_string()).unwrap_or_default(),
                status,
                strip_html(&feedback.unwrap_or_default()),
            ];
            for (c, value) in cells.iter().enumerate() {
                ws.write(row, c as u16, cell(value).as_str())
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    drop(conn);
    wb.save(&path).map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- Resume ----------
fn row_resume(r: &Row) -> rusqlite::Result<Resume> {
    Ok(Resume {
        id: r.get(0)?,
        name: r.get(1)?,
        filename: r.get(2)?,
        filepath: r.get(3)?,
        size: r.get(4)?,
        tags: r.get(5)?,
        created_at: r.get(6)?,
    })
}

#[tauri::command]
pub fn list_resumes(state: State<Db>) -> Result<Vec<Resume>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, name, filename, filepath, size, tags, created_at FROM resumes ORDER BY id DESC")
        .map_err(|x| x.to_string())?;
    let rows = stmt.query_map([], row_resume).map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command(async)]
pub fn import_resume(
    state: State<Db>,
    name: String,
    source_path: String,
) -> Result<Resume, String> {
    let src = validate_import_source(
        Path::new(&source_path),
        &["pdf", "doc", "docx", "txt", "rtf"],
        50 * 1024 * 1024,
    )?;
    let filename = src
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "resume".to_string());
    let dir = crate::db::default_db_dir().join("resumes");
    std::fs::create_dir_all(&dir).map_err(|x| x.to_string())?;
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|x| x.to_string())?
        .as_millis();
    let dest = dir.join(format!("{}_{}", ts, filename));
    std::fs::copy(src, &dest).map_err(|x| x.to_string())?;
    let size = std::fs::metadata(&dest)
        .map(|m| m.len() as i64)
        .unwrap_or(0);
    let filepath = dest.to_string_lossy().to_string();
    let name = if name.trim().is_empty() {
        filename.clone()
    } else {
        name.trim().to_string()
    };
    let conn = state.0.lock().unwrap();
    if let Err(e) = conn.execute(
        "INSERT INTO resumes (name, filename, filepath, size, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, filename, filepath, size, Option::<String>::None],
    ) {
        let _ = std::fs::remove_file(&dest);
        return Err(e.to_string());
    }
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, name, filename, filepath, size, tags, created_at FROM resumes WHERE id = ?1",
        params![id],
        row_resume,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn delete_resume(state: State<Db>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    let filepath: Option<String> = conn
        .query_row(
            "SELECT filepath FROM resumes WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .ok();
    conn.execute("DELETE FROM resumes WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    drop(conn);
    if let Some(fp) = filepath {
        remove_managed_files(vec![fp]);
    }
    Ok(())
}

// ---------- Upcoming interviews ----------
fn row_upcoming(r: &Row) -> rusqlite::Result<UpcomingInterview> {
    Ok(UpcomingInterview {
        id: r.get(0)?,
        round: r.get(1)?,
        scheduled_at: r.get(2)?,
        status: r.get(3)?,
        format: r.get(4)?,
        job_title: r.get(5)?,
        company_name: r.get(6)?,
    })
}

#[tauri::command]
pub fn list_upcoming_interviews(
    state: State<Db>,
    days: i64,
) -> Result<Vec<UpcomingInterview>, String> {
    if !(0..=365).contains(&days) {
        return Err("查询天数必须在 0 到 365 之间".to_string());
    }
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT i.id, i.round, i.scheduled_at, i.status, i.format, j.title, c.name \
             FROM interviews i \
             JOIN applications a ON a.id = i.application_id \
             JOIN jobs j ON j.id = a.job_id \
             JOIN companies c ON c.id = j.company_id \
             WHERE i.scheduled_at IS NOT NULL \
               AND date(i.scheduled_at) >= date('now','localtime') \
               AND date(i.scheduled_at) <= date('now','localtime', ?1) \
             ORDER BY i.scheduled_at, i.id",
        )
        .map_err(|x| x.to_string())?;
    let rows = stmt
        .query_map(params![format!("+{} days", days)], row_upcoming)
        .map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

// ---------- Backup / Restore ----------
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        if ty.is_symlink() {
            return Err("备份中不允许包含符号链接".to_string());
        }
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn vacuum_to(conn: &rusqlite::Connection, dest: &Path) -> Result<(), String> {
    let escaped = dest.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{}'", escaped))
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn export_backup(state: State<Db>, target_dir: String) -> Result<String, String> {
    let base = std::path::PathBuf::from(&target_dir);
    let data_dir = crate::db::default_db_dir();
    std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let base_canonical = base.canonicalize().map_err(|e| e.to_string())?;
    let data_canonical = data_dir.canonicalize().unwrap_or(data_dir.clone());
    if base_canonical.starts_with(&data_canonical) || data_canonical.starts_with(&base_canonical) {
        return Err("备份目标不能是应用数据目录或其父目录".to_string());
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_millis();
    let dir = base.join(format!("OfferFlow-Backup-{stamp}"));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    {
        let conn = state.0.lock().unwrap();
        let dest = dir.join("offerflow.db");
        let _ = std::fs::remove_file(&dest);
        vacuum_to(&conn, &dest)?;
    }
    let data_dir = crate::db::default_db_dir();
    let src_att = data_dir.join("attachments");
    if src_att.exists() {
        copy_dir_recursive(&src_att, &dir.join("attachments"))?;
    }
    let src_resumes = data_dir.join("resumes");
    if src_resumes.exists() {
        copy_dir_recursive(&src_resumes, &dir.join("resumes"))?;
    }
    let src_snapshots = data_dir.join("snapshots");
    if src_snapshots.exists() {
        copy_dir_recursive(&src_snapshots, &dir.join("snapshots"))?;
    }
    let settings = data_dir.join("settings.json");
    if settings.exists() {
        let raw = std::fs::read_to_string(&settings).map_err(|e| e.to_string())?;
        let mut value: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                "openaiKey".to_string(),
                serde_json::Value::String(String::new()),
            );
            obj.insert(
                "cookie".to_string(),
                serde_json::Value::String(String::new()),
            );
        }
        std::fs::write(
            dir.join("settings.json"),
            serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command(async)]
pub fn import_backup(source_dir: String) -> Result<String, String> {
    let src = std::path::PathBuf::from(&source_dir);
    let src_db = src.join("offerflow.db");
    if !src_db.exists() {
        return Err("所选目录中未找到 offerflow.db".to_string());
    }
    crate::db::validate_backup_database(&src_db)?;
    let data_dir = crate::db::default_db_dir();
    std::fs::create_dir_all(&data_dir).map_err(|e| e.to_string())?;
    let src_att = src.join("attachments");
    let pending_att = data_dir.join("restore_attachments");
    if pending_att.exists() {
        std::fs::remove_dir_all(&pending_att).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&pending_att).map_err(|e| e.to_string())?;
    if src_att.exists() {
        copy_dir_recursive(&src_att, &pending_att)?;
    }
    let src_resumes = src.join("resumes");
    let pending_resumes = data_dir.join("restore_resumes");
    if pending_resumes.exists() {
        std::fs::remove_dir_all(&pending_resumes).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&pending_resumes).map_err(|e| e.to_string())?;
    if src_resumes.exists() {
        copy_dir_recursive(&src_resumes, &pending_resumes)?;
    }
    let src_snapshots = src.join("snapshots");
    let pending_snapshots = data_dir.join("restore_snapshots");
    if pending_snapshots.exists() {
        std::fs::remove_dir_all(&pending_snapshots).map_err(|e| e.to_string())?;
    }
    std::fs::create_dir_all(&pending_snapshots).map_err(|e| e.to_string())?;
    if src_snapshots.exists() {
        copy_dir_recursive(&src_snapshots, &pending_snapshots)?;
    }
    let settings = src.join("settings.json");
    if settings.exists() {
        std::fs::copy(&settings, data_dir.join("settings.restore.json"))
            .map_err(|e| e.to_string())?;
    }
    let pending_db = data_dir.join("offerflow_restore.db");
    if pending_db.exists() {
        std::fs::remove_file(&pending_db).map_err(|e| e.to_string())?;
    }
    std::fs::copy(&src_db, &pending_db).map_err(|e| e.to_string())?;
    crate::db::validate_backup_database(&pending_db)?;
    Ok("数据已导入，关闭并重新打开应用后完成恢复。".to_string())
}

// ---------- Activity log ----------
fn row_activity(r: &Row) -> rusqlite::Result<ActivityLog> {
    Ok(ActivityLog {
        id: r.get(0)?,
        application_id: r.get(1)?,
        kind: r.get(2)?,
        detail: r.get(3)?,
        created_at: r.get(4)?,
    })
}

fn job_label(conn: &rusqlite::Connection, id: i64) -> String {
    conn.query_row(
        "SELECT c.name || ' · ' || j.title FROM jobs j JOIN companies c ON c.id = j.company_id WHERE j.id = ?1",
        params![id],
        |r| r.get(0),
    )
    .unwrap_or_default()
}

fn stage_name(conn: &rusqlite::Connection, id: i64) -> String {
    conn.query_row("SELECT name FROM stages WHERE id = ?1", params![id], |r| {
        r.get(0)
    })
    .unwrap_or_default()
}

fn log_activity(
    conn: &rusqlite::Connection,
    application_id: i64,
    kind: &str,
    detail: &str,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO activity_logs (application_id, kind, detail) VALUES (?1, ?2, ?3)",
        params![application_id, kind, detail],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_activities(state: State<Db>, application_id: i64) -> Result<Vec<ActivityLog>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, application_id, kind, detail, created_at FROM activity_logs WHERE application_id = ?1 ORDER BY id DESC")
        .map_err(|x| x.to_string())?;
    let rows = stmt
        .query_map(params![application_id], row_activity)
        .map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn add_activity_note(
    state: State<Db>,
    application_id: i64,
    detail: String,
) -> Result<ActivityLog, String> {
    let detail = detail.trim().to_string();
    if detail.is_empty() {
        return Err("备注不能为空".to_string());
    }
    if detail.chars().count() > 5000 {
        return Err("备注过长".to_string());
    }
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT INTO activity_logs (application_id, kind, detail) VALUES (?1, 'note', ?2)",
        params![application_id, detail],
    )
    .map_err(|x| x.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, application_id, kind, detail, created_at FROM activity_logs WHERE id = ?1",
        params![id],
        row_activity,
    )
    .map_err(|x| x.to_string())
}
fn row_review_note(r: &Row) -> rusqlite::Result<ReviewNote> {
    Ok(ReviewNote {
        id: r.get(0)?,
        round: r.get(1)?,
        scheduled_at: r.get(2)?,
        format: r.get(3)?,
        rating: r.get(4)?,
        status: r.get(5)?,
        feedback: r.get(6)?,
        job_title: r.get(7)?,
        company_name: r.get(8)?,
        category: r.get(9)?,
    })
}

#[tauri::command]
pub fn list_review_notes(state: State<Db>) -> Result<Vec<ReviewNote>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT i.id, i.round, i.scheduled_at, i.format, i.rating, i.status, COALESCE(i.feedback, ''), j.title, c.name, j.category FROM interviews i JOIN applications a ON a.id = i.application_id JOIN jobs j ON j.id = a.job_id JOIN companies c ON c.id = j.company_id WHERE COALESCE(i.feedback, '') != '' ORDER BY i.scheduled_at DESC, i.id DESC")
        .map_err(|x| x.to_string())?;
    let rows = stmt
        .query_map([], row_review_note)
        .map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn update_resume(
    state: State<Db>,
    id: i64,
    name: String,
    tags: Option<String>,
) -> Result<Resume, String> {
    let name = require_text(name, "简历名称")?;
    let tags = match tags {
        Some(v) if v.chars().count() > 200 => return Err("标签过长".to_string()),
        other => other,
    };
    let conn = state.0.lock().unwrap();
    conn.execute(
        "UPDATE resumes SET name = ?1, tags = ?2 WHERE id = ?3",
        params![name, tags, id],
    )
    .map_err(|x| x.to_string())?;
    conn.query_row(
        "SELECT id, name, filename, filepath, size, tags, created_at FROM resumes WHERE id = ?1",
        params![id],
        row_resume,
    )
    .map_err(|x| x.to_string())
}
