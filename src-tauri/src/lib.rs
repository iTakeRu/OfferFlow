mod boss;
mod chrome;
#[cfg(windows)]
mod chrome_window;
mod commands;
mod db;
mod experiences;
mod extract;
mod html;
mod job_page;
mod models;
mod page_window;
mod protect;
mod snapshot;
mod updates;

use db::Db;
use std::path::Path;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let db_path = db::prepare_data_dir().join("offerflow.db");
    let context = tauri::generate_context!();
    match db::init(&db_path) {
        Ok(conn) => {
            snapshot::clear_pending();
            start(Db::new(conn), context)
        }
        // 这里以前是 expect()：任何初始化失败都会变成无窗口、无提示的闪退，
        // 用户完全不知道发生了什么。改成弹一个能看懂的原生提示再退出。
        Err(err) => fail(&err, &db_path),
    }
}

fn start(db_state: Db, context: tauri::Context) {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 音频走 asset: 协议直接流式播放（不再把整个文件 base64 过 IPC）。
            // 静态 scope 覆盖不到 OFFERFLOW_DATA_DIR 指定的目录，这里按运行时
            // 真实的数据目录再放行一次。
            use tauri::Manager;
            app.asset_protocol_scope()
                .allow_directory(db::default_db_dir(), true)?;
            // 清理过期回收站、同步镜像备份：放到后台，不拖慢启动
            let handle = app.handle().clone();
            std::thread::spawn(move || protect::run_startup_tasks(&handle.state::<Db>()));
            Ok(())
        })
        .manage(db_state)
        // 注意：Tauri 2 里没标 async 的命令直接在主线程执行，期间窗口会冻住。
        // 凡是会访问网络、启动子进程或读写大文件的命令，都要写成
        // `#[tauri::command(async)]` 放到线程池里跑；只做一次 SQLite 查询的可以保持同步。
        .invoke_handler(tauri::generate_handler![
            updates::check_for_updates,
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
            commands::delete_attachment,
            commands::set_attachment_duration,
            commands::list_audio_marks,
            commands::add_audio_mark,
            commands::delete_audio_mark,
            commands::open_managed_path,
            commands::import_attachment,
            commands::export_excel,
            commands::list_resumes,
            experiences::list_experiences,
            experiences::save_experience,
            experiences::delete_experience,
            experiences::import_experiences,
            commands::import_resume,
            commands::delete_resume,
            commands::update_resume,
            commands::list_upcoming_interviews,
            commands::export_backup,
            commands::import_backup,
            commands::list_activities,
            commands::add_activity_note,
            commands::list_review_notes,
            extract::get_settings,
            extract::set_settings,
            extract::extract_jd_info,
            extract::list_ollama_models,
            extract::check_ollama,
            snapshot::capture_snapshot_from_url,
            snapshot::capture_snapshot_from_chrome,
            snapshot::capture_current_boss_job,
            snapshot::capture_snapshot_from_file,
            snapshot::capture_snapshot_from_text,
            snapshot::discard_snapshot_draft,
            snapshot::save_job_snapshot,
            snapshot::list_job_snapshots,
            snapshot::delete_job_snapshot,
            protect::get_protect_settings,
            protect::set_protect_settings,
            protect::sync_mirror_now,
            protect::get_mirror_status,
            protect::open_trash_dir,
            protect::get_data_dir,
            protect::open_data_dir,
        ])
        .build(context)
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                use tauri::Manager;
                app.state::<Db>().close_for_exit();
            }
        });
}

fn fail(err: &str, db_path: &Path) {
    let message = format!(
        "OfferFlow 无法启动。\n\n{err}\n\n数据文件：{}",
        db_path.display()
    );
    show_startup_error(&message);
    std::process::exit(1);
}

#[cfg(windows)]
fn show_startup_error(message: &str) {
    #[link(name = "User32")]
    extern "system" {
        fn MessageBoxW(
            hwnd: *mut std::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            kind: u32,
        ) -> i32;
    }
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
    const MB_ICONERROR: u32 = 0x10;
    let text = wide(message);
    let caption = wide("OfferFlow");
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            caption.as_ptr(),
            MB_ICONERROR,
        );
    }
}

#[cfg(not(windows))]
fn show_startup_error(message: &str) {
    eprintln!("{message}");
}
