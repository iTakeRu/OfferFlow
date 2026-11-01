//! 数据保护的两道可选防线，都可在设置里单独开关：
//!
//! 1. 回收站：删除投递 / 面试 / 附件 / 简历 / 快照时，文件先移进 `trash/<时间戳>/`，
//!    保留一段时间再清理。误删或删除逻辑出 bug，都还能从这里找回录音等文件。
//! 2. 镜像备份：把数据增量同步到另一个位置（另一块硬盘、网盘同步文件夹）。
//!    托管文件只会新增、删除，从不原地修改，所以只复制新增的；镜像里的文件从不删除。
//!    镜像目录的结构和「导出完整备份」一致，可以直接用「从备份恢复」还原。

use crate::db::Db;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use tauri_plugin_opener::OpenerExt;

const SETTINGS_FILE: &str = "protect.json";
const STATUS_FILE: &str = "mirror-status.json";
const TRASH_DIR: &str = "trash";
pub const MIRROR_FOLDER: &str = "OfferFlow-Mirror";
/// 需要同步的托管文件目录，和完整备份里的一致。
const MANAGED_DIRS: [&str; 3] = ["attachments", "resumes", "snapshots"];
/// 镜像里每天另存一份数据库，保留最近这么多天。
const DB_HISTORY_KEEP: usize = 14;
const DAY_MS: u128 = 24 * 60 * 60 * 1000;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ProtectSettings {
    pub trash_enabled: bool,
    pub trash_days: u32,
    pub mirror_enabled: bool,
    pub mirror_dir: String,
}

impl Default for ProtectSettings {
    fn default() -> Self {
        // 回收站默认开：它不需要任何配置，关掉反而容易丢东西。
        // 镜像默认关：得先由用户选一个备份位置。
        Self {
            trash_enabled: true,
            trash_days: 30,
            mirror_enabled: false,
            mirror_dir: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct MirrorReport {
    pub finished_at: String,
    pub target: String,
    pub copied_files: u64,
    pub copied_bytes: u64,
    pub unchanged_files: u64,
    pub error: Option<String>,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub fn load_settings(data_dir: &Path) -> ProtectSettings {
    fs::read_to_string(data_dir.join(SETTINGS_FILE))
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_settings(data_dir: &Path, settings: &ProtectSettings) -> Result<(), String> {
    let raw = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(data_dir.join(SETTINGS_FILE), raw).map_err(|e| e.to_string())
}

// ---------- 回收站 ----------

/// 本次删除要用的回收站批次目录 `trash/<毫秒时间戳>`；回收站关闭时返回 None，文件直接删除。
pub fn trash_batch(data_dir: &Path) -> Option<PathBuf> {
    load_settings(data_dir)
        .trash_enabled
        .then(|| data_dir.join(TRASH_DIR).join(now_ms().to_string()))
}

/// 把 `root` 下的文件移进回收站批次，保留它在数据目录里的相对路径，方便对照找回。
pub fn move_to_trash(file: &Path, root: &Path, batch: &Path) -> std::io::Result<()> {
    let rel = match file.strip_prefix(root) {
        Ok(rel) => rel.to_path_buf(),
        Err(_) => PathBuf::from(file.file_name().unwrap_or_default()),
    };
    let dest = batch.join(rel);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    // 回收站在数据目录里，通常和原文件同盘，rename 即可；跨盘时退回复制 + 删除。
    if fs::rename(file, &dest).is_err() {
        fs::copy(file, &dest)?;
        fs::remove_file(file)?;
    }
    Ok(())
}

/// 清掉超过保留天数的回收站批次，返回清掉的批次数。
/// 回收站关掉以后，里面已有的内容照常到期清理；不是时间戳命名的目录一律不动。
pub fn purge_trash(data_dir: &Path, keep_days: u32, now: u128) -> usize {
    let Ok(entries) = fs::read_dir(data_dir.join(TRASH_DIR)) else {
        return 0;
    };
    let keep = keep_days as u128 * DAY_MS;
    entries
        .flatten()
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .parse::<u128>()
                .is_ok_and(|stamp| now.saturating_sub(stamp) > keep)
        })
        .filter(|e| fs::remove_dir_all(e.path()).is_ok())
        .count()
}

// ---------- 镜像备份 ----------

/// 只复制目标里还没有的文件（同名且大小相同视为已有）。
/// 先写到临时文件再改名，中途断掉也不会在镜像里留下残缺文件。
fn copy_new_files(src: &Path, dst: &Path, report: &mut MirrorReport) -> Result<(), String> {
    if !src.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_symlink() {
            continue;
        }
        if ty.is_dir() {
            copy_new_files(&from, &to, report)?;
            continue;
        }
        let len = entry.metadata().map_err(|e| e.to_string())?.len();
        if fs::metadata(&to).is_ok_and(|m| m.len() == len) {
            report.unchanged_files += 1;
            continue;
        }
        let tmp = to.with_file_name(format!(".{}.partial", entry.file_name().to_string_lossy()));
        fs::copy(&from, &tmp).map_err(|e| format!("复制 {} 失败：{e}", from.display()))?;
        fs::rename(&tmp, &to).map_err(|e| e.to_string())?;
        report.copied_files += 1;
        report.copied_bytes += len;
    }
    Ok(())
}

/// 只保留最近 `keep` 份每日数据库（文件名带日期，按名字排序即按时间排序）。
fn prune_db_history(dir: &Path, keep: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "db"))
        .collect();
    files.sort();
    let excess = files.len().saturating_sub(keep);
    for old in &files[..excess] {
        let _ = fs::remove_file(old);
    }
}

/// 把数据同步到 `target_root/OfferFlow-Mirror`。
///
/// 先导出数据库，再复制文件并核对引用。文件复制不持有数据库锁。
pub fn sync_mirror(db: &Db, data_dir: &Path, target_root: &Path) -> Result<MirrorReport, String> {
    static SYNC_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = SYNC_LOCK.lock().map_err(|e| e.to_string())?;
    if !target_root.is_dir() {
        return Err(format!("备份位置不存在：{}", target_root.display()));
    }
    crate::commands::ensure_outside_data_dir(target_root, data_dir)?;
    let dest = target_root.join(MIRROR_FOLDER);
    fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    // 数据库每次都完整导出一份最新的：先导到临时文件，成功后再替换
    let tmp = dest.join("offerflow.db.partial");
    let _ = fs::remove_file(&tmp);
    let (today, finished_at) = {
        let conn = db.0.lock();
        crate::commands::vacuum_to(&conn, &tmp)?;
        conn.query_row(
            "SELECT date('now','localtime'), datetime('now','localtime')",
            [],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .map_err(|e| e.to_string())?
    };
    finish_mirror(data_dir, &dest, &tmp, &today, finished_at)
}

/// 此时 tmp 是固定的数据库快照；后续新增记录不应混进这一份备份。
fn finish_mirror(
    data_dir: &Path,
    dest: &Path,
    tmp: &Path,
    today: &str,
    finished_at: String,
) -> Result<MirrorReport, String> {
    let mut report = MirrorReport {
        target: dest.to_string_lossy().to_string(),
        ..Default::default()
    };
    for sub in MANAGED_DIRS {
        copy_new_files(&data_dir.join(sub), &dest.join(sub), &mut report)?;
    }
    crate::commands::validate_backup_files(tmp, dest)?;
    crate::commands::write_sanitized_settings(data_dir, dest)?;
    let latest = dest.join("offerflow.db");
    fs::rename(tmp, &latest).map_err(|e| e.to_string())?;

    // 另按天留档：万一 bug 把数据改坏又被同步过来，最新那份会被覆盖，还能退回前几天
    let history = dest.join("db-history");
    fs::create_dir_all(&history).map_err(|e| e.to_string())?;
    let daily = history.join(format!("offerflow-{today}.db"));
    fs::copy(&latest, &daily).map_err(|e| e.to_string())?;
    prune_db_history(&history, DB_HISTORY_KEEP);

    report.finished_at = finished_at;
    Ok(report)
}

fn write_status(data_dir: &Path, report: &MirrorReport) {
    if let Ok(raw) = serde_json::to_vec_pretty(report) {
        let _ = fs::write(data_dir.join(STATUS_FILE), raw);
    }
}

fn sync_and_record(db: &Db, data_dir: &Path, target: &Path) -> Result<MirrorReport, String> {
    let result = sync_mirror(db, data_dir, target);
    let report = match &result {
        Ok(report) => report.clone(),
        Err(e) => MirrorReport {
            target: target.join(MIRROR_FOLDER).to_string_lossy().to_string(),
            finished_at: db
                .0
                .lock()
                .query_row("SELECT datetime('now','localtime')", [], |r| r.get(0))
                .unwrap_or_default(),
            error: Some(e.clone()),
            ..Default::default()
        },
    };
    write_status(data_dir, &report);
    result
}

/// 启动后在后台跑：清理过期回收站；开了镜像就同步一次。失败只记到状态文件里，不打扰启动。
pub fn run_startup_tasks(db: &Db) {
    let data_dir = crate::db::default_db_dir();
    let settings = load_settings(&data_dir);
    purge_trash(&data_dir, settings.trash_days, now_ms());
    if settings.mirror_enabled && !settings.mirror_dir.trim().is_empty() {
        let _ = sync_and_record(db, &data_dir, Path::new(settings.mirror_dir.trim()));
    }
}

// ---------- 命令 ----------

#[tauri::command]
pub fn get_protect_settings() -> ProtectSettings {
    load_settings(&crate::db::default_db_dir())
}

#[tauri::command]
pub fn set_protect_settings(settings: ProtectSettings) -> Result<(), String> {
    let data_dir = crate::db::default_db_dir();
    if !(1..=365).contains(&settings.trash_days) {
        return Err("回收站保留天数需在 1 到 365 之间".to_string());
    }
    let dir = settings.mirror_dir.trim();
    if settings.mirror_enabled && dir.is_empty() {
        return Err("开启自动同步前，请先选择备份位置".to_string());
    }
    if !dir.is_empty() {
        let path = Path::new(dir);
        if !path.is_absolute() || !path.is_dir() {
            return Err(format!("备份位置不存在：{dir}"));
        }
        crate::commands::ensure_outside_data_dir(path, &data_dir)?;
    }
    save_settings(
        &data_dir,
        &ProtectSettings {
            mirror_dir: dir.to_string(),
            ..settings
        },
    )
}

#[tauri::command(async)]
pub fn sync_mirror_now(state: State<Db>) -> Result<MirrorReport, String> {
    let data_dir = crate::db::default_db_dir();
    let settings = load_settings(&data_dir);
    let dir = settings.mirror_dir.trim();
    if dir.is_empty() {
        return Err("请先选择备份位置并保存设置".to_string());
    }
    sync_and_record(&state, &data_dir, Path::new(dir))
}

#[tauri::command]
pub fn get_mirror_status() -> Option<MirrorReport> {
    let raw = fs::read_to_string(crate::db::default_db_dir().join(STATUS_FILE)).ok()?;
    serde_json::from_str(&raw).ok()
}

#[tauri::command]
pub fn open_trash_dir(app: tauri::AppHandle) -> Result<(), String> {
    let dir = crate::db::default_db_dir().join(TRASH_DIR);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataDirInfo {
    pub path: String,
    pub source: crate::db::DataDirSource,
}

#[tauri::command]
pub fn get_data_dir() -> DataDirInfo {
    let (dir, source) = crate::db::resolve_data_dir();
    DataDirInfo {
        path: dir.to_string_lossy().to_string(),
        source,
    }
}

/// 默认数据目录在 AppData 里，资源管理器默认不显示，给用户一个直达入口。
#[tauri::command]
pub fn open_data_dir(app: tauri::AppHandle) -> Result<(), String> {
    let dir = crate::db::default_db_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(dir.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "offerflow-protect-{name}-{}-{}",
            std::process::id(),
            now_ms()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn trash_keeps_relative_path_and_removes_original() {
        let root = temp_dir("trash");
        let file = root.join("attachments/7/rec.m4a");
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, b"audio").unwrap();
        let batch = root.join("trash/123");
        move_to_trash(&file, &root, &batch).unwrap();
        assert!(!file.exists());
        assert_eq!(
            fs::read(batch.join("attachments/7/rec.m4a")).unwrap(),
            b"audio"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn purge_only_removes_expired_timestamp_batches() {
        let root = temp_dir("purge");
        let now = 100 * DAY_MS;
        let old = (now - 31 * DAY_MS).to_string();
        let recent = (now - 2 * DAY_MS).to_string();
        for name in [old.as_str(), recent.as_str(), "not-a-batch"] {
            fs::create_dir_all(root.join("trash").join(name)).unwrap();
        }
        assert_eq!(purge_trash(&root, 30, now), 1);
        assert!(!root.join("trash").join(&old).exists());
        assert!(root.join("trash").join(&recent).exists());
        assert!(
            root.join("trash/not-a-batch").exists(),
            "不认识的目录不能删"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn settings_default_to_trash_on_mirror_off() {
        let root = temp_dir("settings");
        let s = load_settings(&root);
        assert!(s.trash_enabled && !s.mirror_enabled && s.trash_days == 30);
        fs::write(root.join(SETTINGS_FILE), br#"{"mirrorEnabled":true}"#).unwrap();
        let s = load_settings(&root);
        assert!(s.mirror_enabled && s.trash_enabled, "缺的字段应取默认值");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn mirror_copies_only_new_files_and_is_restorable() {
        let base = temp_dir("mirror");
        let data = base.join("data");
        let target = base.join("usb");
        fs::create_dir_all(data.join("attachments/1")).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(data.join("attachments/1/a.m4a"), b"first").unwrap();
        fs::write(
            data.join("settings.json"),
            br#"{"openaiKey":"sk-secret","cookie":"c=1","provider":"none"}"#,
        )
        .unwrap();
        let db = Db::new(crate::db::init(&data.join("offerflow.db")).unwrap());
        db.0.lock()
            .execute("INSERT INTO companies(name) VALUES('镜像测试')", [])
            .unwrap();

        let first = sync_mirror(&db, &data, &target).unwrap();
        assert_eq!((first.copied_files, first.unchanged_files), (1, 0));

        // 第二次：旧文件跳过，只复制新增的；本地删掉的文件镜像里仍保留
        fs::write(data.join("attachments/1/b.m4a"), b"second").unwrap();
        fs::remove_file(data.join("attachments/1/a.m4a")).unwrap();
        let second = sync_mirror(&db, &data, &target).unwrap();
        assert_eq!((second.copied_files, second.unchanged_files), (1, 0));
        let mirror = target.join(MIRROR_FOLDER);
        assert!(
            mirror.join("attachments/1/a.m4a").exists(),
            "镜像不应跟着本地删除"
        );
        assert!(mirror.join("attachments/1/b.m4a").exists());

        // 镜像能通过完整备份的校验，数据库里有刚写入的数据，且不含密钥
        crate::db::validate_backup_database(&mirror.join("offerflow.db")).unwrap();
        let conn = rusqlite::Connection::open(mirror.join("offerflow.db")).unwrap();
        let n: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM companies WHERE name = '镜像测试'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
        drop(conn);
        assert_eq!(fs::read_dir(mirror.join("db-history")).unwrap().count(), 1);
        let settings = fs::read_to_string(mirror.join("settings.json")).unwrap();
        assert!(!settings.contains("sk-secret") && !settings.contains("c=1"));

        db.close_for_exit();
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn mirror_refuses_target_inside_data_dir() {
        let base = temp_dir("inside");
        let data = base.join("data");
        fs::create_dir_all(data.join("sub")).unwrap();
        let db = Db::new(crate::db::init(&data.join("offerflow.db")).unwrap());
        assert!(sync_mirror(&db, &data, &data.join("sub")).is_err());
        assert!(
            sync_mirror(&db, &data, &base).is_err(),
            "数据目录的上级也不行"
        );
        db.close_for_exit();
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn mirror_does_not_include_records_added_after_database_snapshot() {
        let base = temp_dir("mirror-concurrent-insert");
        let data = base.join("data");
        let dest = base.join("mirror");
        fs::create_dir_all(&data).unwrap();
        fs::create_dir_all(&dest).unwrap();
        let db = Db::new(crate::db::init(&data.join("offerflow.db")).unwrap());
        let tmp = dest.join("offerflow.db.partial");
        crate::commands::vacuum_to(&db.0.lock(), &tmp).unwrap();
        // 模拟数据库快照完成后，用户导入了一份简历。
        fs::create_dir_all(data.join("resumes")).unwrap();
        let file = data.join("resumes/new.pdf");
        fs::write(&file, b"new").unwrap();
        db.0.lock()
            .execute(
                "INSERT INTO resumes(name,filename,filepath,size) VALUES('new','new.pdf',?1,3)",
                [file.to_string_lossy().as_ref()],
            )
            .unwrap();
        finish_mirror(&data, &dest, &tmp, "2026-10-15", "finished".into()).unwrap();
        let snapshot = rusqlite::Connection::open(dest.join("offerflow.db")).unwrap();
        let count: i64 = snapshot
            .query_row("SELECT COUNT(*) FROM resumes", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0, "新记录不能混进先前固定的数据库快照");
        crate::commands::validate_backup_files(&dest.join("offerflow.db"), &dest).unwrap();
        drop(snapshot);
        db.close_for_exit();
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn mirror_missing_file_keeps_previous_published_database() {
        let base = temp_dir("mirror-concurrent-delete");
        let data = base.join("data");
        let target = base.join("usb");
        fs::create_dir_all(data.join("resumes")).unwrap();
        fs::create_dir_all(&target).unwrap();
        let db = Db::new(crate::db::init(&data.join("offerflow.db")).unwrap());
        sync_mirror(&db, &data, &target).unwrap();
        let dest = target.join(MIRROR_FOLDER);
        let previous = fs::read(dest.join("offerflow.db")).unwrap();
        let file = data.join("resumes/deleted.pdf");
        fs::write(&file, b"resume").unwrap();
        db.0.lock().execute(
            "INSERT INTO resumes(name,filename,filepath,size) VALUES('deleted','deleted.pdf',?1,6)",
            [file.to_string_lossy().as_ref()],
        ).unwrap();
        let tmp = dest.join("offerflow.db.partial");
        crate::commands::vacuum_to(&db.0.lock(), &tmp).unwrap();
        // 模拟在数据库导出后、文件复制前删除简历。
        db.0.lock().execute("DELETE FROM resumes", []).unwrap();
        fs::remove_file(file).unwrap();
        assert!(
            finish_mirror(&data, &dest, &tmp, "2026-10-15", "finished".into())
                .unwrap_err()
                .contains("备份缺少文件")
        );
        assert_eq!(fs::read(dest.join("offerflow.db")).unwrap(), previous);
        crate::commands::validate_backup_files(&dest.join("offerflow.db"), &dest).unwrap();
        db.close_for_exit();
        let _ = fs::remove_dir_all(base);
    }

    #[test]
    fn prunes_db_history_to_newest() {
        let dir = temp_dir("history");
        for d in 1..=16 {
            fs::write(dir.join(format!("offerflow-2026-09-{d:02}.db")), b"x").unwrap();
        }
        prune_db_history(&dir, DB_HISTORY_KEEP);
        let mut left: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        left.sort();
        assert_eq!(left.len(), 14);
        assert_eq!(left[0], "offerflow-2026-09-03.db");
        let _ = fs::remove_dir_all(dir);
    }
}
