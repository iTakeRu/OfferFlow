use crate::db::Db;
use crate::models::*;
use rusqlite::{params, Row};
use base64::engine::general_purpose;
use base64::Engine;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

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
    let rows = stmt
        .query_map([], row_stage)
        .map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn create_stage(state: State<Db>, name: String) -> Result<Stage, String> {
    let conn = state.0.lock().unwrap();
    let max_order: i64 = conn
        .query_row("SELECT COALESCE(MAX(sort_order), -1) FROM stages", [], |r| r.get(0))
        .map_err(|x| x.to_string())?;
    conn.execute(
        "INSERT INTO stages (name, sort_order) VALUES (?1, ?2)",
        params![name, max_order + 1],
    )
    .map_err(|x| x.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row("SELECT id, name, sort_order FROM stages WHERE id = ?1", params![id], row_stage)
        .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn update_stage(state: State<Db>, id: i64, name: String) -> Result<Stage, String> {
    let conn = state.0.lock().unwrap();
    conn.execute("UPDATE stages SET name = ?1 WHERE id = ?2", params![name, id])
        .map_err(|x| x.to_string())?;
    conn.query_row("SELECT id, name, sort_order FROM stages WHERE id = ?1", params![id], row_stage)
        .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn delete_stage(state: State<Db>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute("DELETE FROM stages WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn reorder_stages(state: State<Db>, ids: Vec<i64>) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    for (i, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE stages SET sort_order = ?1 WHERE id = ?2",
            params![i as i64, id],
        )
        .map_err(|x| x.to_string())?;
    }
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
    let conn = state.0.lock().unwrap();
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
    conn.execute("DELETE FROM companies WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    Ok(())
}

// ---------- Job ----------
#[tauri::command]
pub fn list_jobs(state: State<Db>, company_id: Option<i64>) -> Result<Vec<Job>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, company_id, title, department, location, category, jd_url, jd_text, salary_min, salary_max, created_at FROM jobs WHERE (?1 IS NULL OR company_id = ?1) ORDER BY id DESC")
        .map_err(|x| x.to_string())?;
    let rows = stmt.query_map(params![company_id], row_job).map_err(|x| x.to_string())?;
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
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT INTO jobs (company_id, title, department, location, category, jd_url, jd_text, salary_min, salary_max) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![company_id, title, department, location, category, jd_url, jd_text, salary_min, salary_max],
    )
    .map_err(|x| x.to_string())?;
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
    conn.execute("DELETE FROM jobs WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    Ok(())
}
// ---------- Application ----------
#[tauri::command]
pub fn list_applications(state: State<Db>) -> Result<Vec<Application>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, job_id, channel, resume_version, stage_id, status, applied_at, notes, created_at, updated_at FROM applications ORDER BY id DESC")
        .map_err(|x| x.to_string())?;
    let rows = stmt.query_map([], row_application).map_err(|x| x.to_string())?;
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
    let conn = state.0.lock().unwrap();
    let status = status.unwrap_or_else(|| "推进中".to_string());
    conn.execute(
        "INSERT INTO applications (job_id, channel, resume_version, stage_id, status, applied_at, notes) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![job_id, channel, resume_version, stage_id, status, applied_at, notes],
    )
    .map_err(|x| x.to_string())?;
    let id = conn.last_insert_rowid();
    log_activity(&conn, id, "created", &format!("创建投递（阶段：{}，状态：{}）", stage_name(&conn, stage_id), status));
    conn.query_row(
        "SELECT id, job_id, channel, resume_version, stage_id, status, applied_at, notes, created_at, updated_at FROM applications WHERE id = ?1",
        params![id],
        row_application,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn update_application(
    state: State<Db>,
    id: i64,
    channel: Option<String>,
    resume_version: Option<String>,
    stage_id: i64,
    status: String,
    applied_at: Option<String>,
    notes: Option<String>,
) -> Result<Application, String> {
    let conn = state.0.lock().unwrap();
    let (old_stage, old_status): (i64, String) = conn
        .query_row(
            "SELECT stage_id, status FROM applications WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|x| x.to_string())?;
    conn.execute(
        "UPDATE applications SET channel = ?1, resume_version = ?2, stage_id = ?3, status = ?4, applied_at = ?5, notes = ?6, updated_at = datetime('now','localtime') WHERE id = ?7",
        params![channel, resume_version, stage_id, status, applied_at, notes, id],
    )
    .map_err(|x| x.to_string())?;
    if old_stage != stage_id {
        log_activity(&conn, id, "stage", &format!("阶段变更：{} → {}", stage_name(&conn, old_stage), stage_name(&conn, stage_id)));
    }
    if old_status != status {
        log_activity(&conn, id, "status", &format!("状态变更：{} → {}", old_status, status));
    }
    conn.query_row(
        "SELECT id, job_id, channel, resume_version, stage_id, status, applied_at, notes, created_at, updated_at FROM applications WHERE id = ?1",
        params![id],
        row_application,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn move_application(state: State<Db>, id: i64, stage_id: i64) -> Result<Application, String> {
    let conn = state.0.lock().unwrap();
    let old_stage: i64 = conn
        .query_row("SELECT stage_id FROM applications WHERE id = ?1", params![id], |r| r.get(0))
        .map_err(|x| x.to_string())?;
    conn.execute(
        "UPDATE applications SET stage_id = ?1, updated_at = datetime('now','localtime') WHERE id = ?2",
        params![stage_id, id],
    )
    .map_err(|x| x.to_string())?;
    if old_stage != stage_id {
        log_activity(&conn, id, "stage", &format!("阶段变更：{} → {}", stage_name(&conn, old_stage), stage_name(&conn, stage_id)));
    }
    conn.query_row(
        "SELECT id, job_id, channel, resume_version, stage_id, status, applied_at, notes, created_at, updated_at FROM applications WHERE id = ?1",
        params![id],
        row_application,
    )
    .map_err(|x| x.to_string())
}

#[tauri::command]
pub fn delete_application(state: State<Db>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().unwrap();
    conn.execute("DELETE FROM applications WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    Ok(())
}

// ---------- Interview ----------
#[tauri::command]
pub fn list_interviews(state: State<Db>, application_id: i64) -> Result<Vec<Interview>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, application_id, round, format, location, interviewer, scheduled_at, rating, status, feedback, created_at FROM interviews WHERE application_id = ?1 ORDER BY round")
        .map_err(|x| x.to_string())?;
    let rows = stmt.query_map(params![application_id], row_interview).map_err(|x| x.to_string())?;
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
    let conn = state.0.lock().unwrap();
    let status = status.unwrap_or_else(|| "待面试".to_string());
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
    conn.execute("DELETE FROM interviews WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    Ok(())
}

// ---------- Attachment ----------
#[tauri::command]
pub fn list_attachments(state: State<Db>, interview_id: i64) -> Result<Vec<Attachment>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, interview_id, kind, filename, filepath, size, duration, created_at FROM attachments WHERE interview_id = ?1 ORDER BY id")
        .map_err(|x| x.to_string())?;
    let rows = stmt.query_map(params![interview_id], row_attachment).map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn add_attachment(
    state: State<Db>,
    interview_id: i64,
    kind: String,
    filename: String,
    filepath: String,
    size: i64,
    duration: Option<f64>,
) -> Result<Attachment, String> {
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT INTO attachments (interview_id, kind, filename, filepath, size, duration) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![interview_id, kind, filename, filepath, size, duration],
    )
    .map_err(|x| x.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, interview_id, kind, filename, filepath, size, duration, created_at FROM attachments WHERE id = ?1",
        params![id],
        row_attachment,
    )
    .map_err(|x| x.to_string())
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
    if let Some(fp) = filepath {
        let _ = std::fs::remove_file(&fp);
    }
    Ok(())
}

#[tauri::command]
pub fn read_file_base64(path: String) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|x| x.to_string())?;
    Ok(general_purpose::STANDARD.encode(&bytes))
}

#[tauri::command]
pub fn import_attachment(
    state: State<Db>,
    interview_id: i64,
    kind: String,
    source_path: String,
) -> Result<Attachment, String> {
    let src = Path::new(&source_path);
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
    conn.execute(
        "INSERT INTO attachments (interview_id, kind, filename, filepath, size, duration) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![interview_id, kind, filename, filepath, size, None::<f64>],
    )
    .map_err(|x| x.to_string())?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, interview_id, kind, filename, filepath, size, duration, created_at FROM attachments WHERE id = ?1",
        params![id],
        row_attachment,
    )
    .map_err(|x| x.to_string())
}

// ---------- AudioMark ----------
#[tauri::command]
pub fn list_audio_marks(state: State<Db>, interview_id: i64) -> Result<Vec<AudioMark>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, interview_id, attachment_id, position, label, note, created_at FROM audio_marks WHERE interview_id = ?1 ORDER BY position")
        .map_err(|x| x.to_string())?;
    let rows = stmt.query_map(params![interview_id], row_mark).map_err(|x| x.to_string())?;
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
pub fn update_audio_mark(
    state: State<Db>,
    id: i64,
    position: f64,
    label: String,
    note: Option<String>,
) -> Result<AudioMark, String> {
    let conn = state.0.lock().unwrap();
    conn.execute(
        "UPDATE audio_marks SET position = ?1, label = ?2, note = ?3 WHERE id = ?4",
        params![position, label, note, id],
    )
    .map_err(|x| x.to_string())?;
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
    out
}

#[tauri::command]
pub fn export_excel(state: State<Db>, path: String) -> Result<(), String> {
    use rust_xlsxwriter::Workbook;

    let conn = state.0.lock().unwrap();
    let mut wb = Workbook::new();

    // Sheet 1: 投递记录
    {
        let ws = wb.add_worksheet();
        let headers = ["公司", "岗位", "部门", "投递渠道", "简历版本", "当前阶段", "状态", "投递日期", "备注"];
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
        let mut row = 1u32;
        for r in rows {
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
            for (c, cell) in cells.iter().enumerate() {
                ws.write(row, c as u16, cell.as_str()).map_err(|e| e.to_string())?;
            }
            row += 1;
        }
    }

    // Sheet 2: 面试记录
    {
        let ws = wb.add_worksheet();
        let headers = ["公司", "岗位", "轮次", "形式", "地点", "面试官", "面试日期", "评分", "状态", "复盘笔记"];
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
        let mut row = 1u32;
        for r in rows {
            let (company, title, round, format, location, interviewer, scheduled_at, rating, status, feedback) =
                r.map_err(|e| e.to_string())?;
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
            for (c, cell) in cells.iter().enumerate() {
                ws.write(row, c as u16, cell.as_str()).map_err(|e| e.to_string())?;
            }
            row += 1;
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

#[tauri::command]
pub fn import_resume(state: State<Db>, name: String, source_path: String) -> Result<Resume, String> {
    let src = Path::new(&source_path);
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
    let size = std::fs::metadata(&dest).map(|m| m.len() as i64).unwrap_or(0);
    let filepath = dest.to_string_lossy().to_string();
    let name = if name.trim().is_empty() { filename.clone() } else { name.trim().to_string() };
    let conn = state.0.lock().unwrap();
    conn.execute(
        "INSERT INTO resumes (name, filename, filepath, size, tags) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![name, filename, filepath, size, Option::<String>::None],
    )
    .map_err(|x| x.to_string())?;
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
        .query_row("SELECT filepath FROM resumes WHERE id = ?1", params![id], |r| r.get(0))
        .ok();
    conn.execute("DELETE FROM resumes WHERE id = ?1", params![id])
        .map_err(|x| x.to_string())?;
    if let Some(fp) = filepath {
        let _ = std::fs::remove_file(&fp);
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
pub fn list_upcoming_interviews(state: State<Db>, days: i64) -> Result<Vec<UpcomingInterview>, String> {
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

#[tauri::command]
pub fn export_backup(state: State<Db>, target_dir: String) -> Result<String, String> {
    let dir = std::path::PathBuf::from(&target_dir);
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
    let settings = data_dir.join("settings.json");
    if settings.exists() {
        std::fs::copy(&settings, dir.join("settings.json")).map_err(|e| e.to_string())?;
    }
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn import_backup(source_dir: String) -> Result<String, String> {
    let src = std::path::PathBuf::from(&source_dir);
    let src_db = src.join("offerflow.db");
    if !src_db.exists() {
        return Err("所选目录中未找到 offerflow.db".to_string());
    }
    let data_dir = crate::db::default_db_dir();
    let src_att = src.join("attachments");
    if src_att.exists() {
        copy_dir_recursive(&src_att, &data_dir.join("attachments"))?;
    }
    let src_resumes = src.join("resumes");
    if src_resumes.exists() {
        copy_dir_recursive(&src_resumes, &data_dir.join("resumes"))?;
    }
    let settings = src.join("settings.json");
    if settings.exists() {
        std::fs::copy(&settings, data_dir.join("settings.json")).map_err(|e| e.to_string())?;
    }
    std::fs::copy(&src_db, data_dir.join("offerflow_restore.db")).map_err(|e| e.to_string())?;
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

fn stage_name(conn: &rusqlite::Connection, id: i64) -> String {
    conn.query_row("SELECT name FROM stages WHERE id = ?1", params![id], |r| r.get(0))
        .unwrap_or_default()
}

fn log_activity(conn: &rusqlite::Connection, application_id: i64, kind: &str, detail: &str) {
    let _ = conn.execute(
        "INSERT INTO activity_logs (application_id, kind, detail) VALUES (?1, ?2, ?3)",
        params![application_id, kind, detail],
    );
}

#[tauri::command]
pub fn list_activities(state: State<Db>, application_id: i64) -> Result<Vec<ActivityLog>, String> {
    let conn = state.0.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, application_id, kind, detail, created_at FROM activity_logs WHERE application_id = ?1 ORDER BY id DESC")
        .map_err(|x| x.to_string())?;
    let rows = stmt.query_map(params![application_id], row_activity).map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn add_activity_note(state: State<Db>, application_id: i64, detail: String) -> Result<ActivityLog, String> {
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
    let rows = stmt.query_map([], row_review_note).map_err(|x| x.to_string())?;
    let mut v = Vec::new();
    for r in rows {
        v.push(r.map_err(|x| x.to_string())?);
    }
    Ok(v)
}

#[tauri::command]
pub fn update_resume(state: State<Db>, id: i64, name: String, tags: Option<String>) -> Result<Resume, String> {
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
