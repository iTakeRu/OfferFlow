use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Company {
    pub id: i64,
    pub name: String,
    pub industry: Option<String>,
    pub city: Option<String>,
    pub website: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub company_id: i64,
    pub title: String,
    pub department: Option<String>,
    pub location: Option<String>,
    pub category: Option<String>,
    pub jd_url: Option<String>,
    pub jd_text: Option<String>,
    pub salary_min: Option<i64>,
    pub salary_max: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    pub id: i64,
    pub name: String,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Application {
    pub id: i64,
    pub job_id: i64,
    pub channel: Option<String>,
    pub resume_version: Option<String>,
    pub stage_id: i64,
    pub status: String,
    pub applied_at: Option<String>,
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interview {
    pub id: i64,
    pub application_id: i64,
    pub round: i64,
    pub format: Option<String>,
    pub location: Option<String>,
    pub interviewer: Option<String>,
    pub scheduled_at: Option<String>,
    pub rating: Option<i64>,
    pub status: String,
    pub feedback: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: i64,
    pub interview_id: i64,
    pub kind: String,
    pub filename: String,
    pub filepath: String,
    pub size: i64,
    pub duration: Option<f64>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioMark {
    pub id: i64,
    pub interview_id: i64,
    pub attachment_id: i64,
    pub position: f64,
    pub label: String,
    pub note: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resume {
    pub id: i64,
    pub name: String,
    pub filename: String,
    pub filepath: String,
    pub size: i64,
    pub tags: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpcomingInterview {
    pub id: i64,
    pub round: i64,
    pub scheduled_at: String,
    pub status: String,
    pub format: Option<String>,
    pub job_title: String,
    pub company_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLog {
    pub id: i64,
    pub application_id: i64,
    pub kind: String,
    pub detail: String,
    pub created_at: String,
}
/// 岗位 JD 的网页快照，保存后不再修改。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSnapshot {
    pub id: i64,
    pub job_id: i64,
    pub source_url: Option<String>,
    /// url / file / paste，以及从旧版 jobs.jd_text 迁移来的 legacy
    pub source_kind: String,
    pub text: String,
    pub html_path: Option<String>,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewNote {
    pub id: i64,
    pub round: i64,
    pub scheduled_at: Option<String>,
    pub format: Option<String>,
    pub rating: Option<i64>,
    pub status: String,
    pub feedback: String,
    pub job_title: String,
    pub company_name: String,
    pub category: Option<String>,
}
