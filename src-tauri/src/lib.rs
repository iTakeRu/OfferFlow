mod commands;
mod db;
mod models;
mod extract;

use db::Db;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = db::default_db_dir().join("offerflow.db");
    let conn = db::init(&db_path).expect("failed to initialize database");
    let db_state = Db(Mutex::new(conn));

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(db_state)
        .invoke_handler(tauri::generate_handler![
            commands::list_stages,
            commands::create_stage,
            commands::update_stage,
            commands::delete_stage,
            commands::reorder_stages,
            commands::list_companies,
            commands::create_company,
            commands::update_company,
            commands::delete_company,
            commands::list_jobs,
            commands::create_job,
            commands::update_job,
            commands::delete_job,
            commands::list_applications,
            commands::create_application,
            commands::update_application,
            commands::move_application,
            commands::delete_application,
            commands::list_interviews,
            commands::create_interview,
            commands::update_interview,
            commands::delete_interview,
            commands::list_attachments,
            commands::add_attachment,
            commands::delete_attachment,
            commands::list_audio_marks,
            commands::add_audio_mark,
            commands::update_audio_mark,
            commands::delete_audio_mark,
            commands::read_file_base64,
            commands::import_attachment,
            commands::export_excel,
            commands::list_resumes,
            commands::import_resume,
            commands::delete_resume,
            commands::update_resume,
            commands::list_upcoming_interviews,
            commands::export_backup,
            commands::import_backup,
            commands::list_activities,
            commands::add_activity_note,
            extract::get_settings,
            extract::set_settings,
            extract::fetch_jd_text,
            extract::extract_jd_info,
            extract::list_ollama_models,
            extract::fetch_mokahr_job_detail,
            extract::fetch_job_detail,
            commands::list_review_notes
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}