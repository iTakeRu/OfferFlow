use super::*;
use std::fs;
use std::path::PathBuf;

struct Fixture {
    root: PathBuf,
    _dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::Builder::new()
            .prefix("offerflow-backup-test-")
            .tempdir()
            .unwrap();
        Self {
            root: dir.path().to_owned(),
            _dir: dir,
        }
    }

    fn backup(&self, name: &str) -> PathBuf {
        let root = self.root.join(name);
        fs::create_dir_all(root.join("resumes")).unwrap();
        fs::write(root.join("resumes/resume.pdf"), name.as_bytes()).unwrap();
        let conn = crate::db::init(&root.join("offerflow.db")).unwrap();
        conn.execute("INSERT INTO companies(name) VALUES(?1)", [name])
            .unwrap();
        conn.execute(
            "INSERT INTO resumes(name, filename, filepath, size) VALUES(?1, 'resume.pdf', ?2, ?3)",
            params![
                name,
                root.join("resumes/resume.pdf").to_string_lossy(),
                name.len() as i64
            ],
        )
        .unwrap();
        drop(conn);
        root
    }
}

#[test]
fn repeated_restore_preserves_the_complete_pending_backup() {
    let fixture = Fixture::new();
    let a = fixture.backup("backup-a");
    let b = fixture.backup("backup-b");
    let target = fixture.root.join("target");
    fs::create_dir(&target).unwrap();
    drop(crate::db::init(&target.join("offerflow.db")).unwrap());
    prepare_restore(&a, &target).unwrap();
    let pending_before = fs::read(target.join("offerflow_restore.db")).unwrap();
    // B 的附件目录不可读取；无论 B 是否有效，都不能破坏已准备好的 A。
    fs::write(b.join("attachments"), b"not a directory").unwrap();
    assert!(prepare_restore(&b, &target)
        .unwrap_err()
        .contains("已有待恢复备份"));
    assert_eq!(
        fs::read(target.join("offerflow_restore.db")).unwrap(),
        pending_before
    );
    assert_eq!(
        fs::read(target.join("restore_resumes/resume.pdf")).unwrap(),
        b"backup-a"
    );

    let restored = crate::db::init(&target.join("offerflow.db")).unwrap();
    let company: String = restored
        .query_row("SELECT name FROM companies", [], |r| r.get(0))
        .unwrap();
    assert_eq!(company, "backup-a");
    assert_eq!(
        fs::read(target.join("resumes/resume.pdf")).unwrap(),
        b"backup-a"
    );
    validate_backup_files(&target.join("offerflow.db"), &target).unwrap();
}

#[test]
fn failed_restore_copy_does_not_publish_and_can_be_retried() {
    let fixture = Fixture::new();
    let source = fixture.backup("source");
    let target = fixture.root.join("target");
    fs::write(source.join("attachments"), b"not a directory").unwrap();
    assert!(prepare_restore(&source, &target).is_err());
    assert!(!target.join("offerflow_restore.db").exists());
    assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
    fs::remove_file(source.join("attachments")).unwrap();
    prepare_restore(&source, &target).unwrap();
    assert!(target.join("offerflow_restore.db").is_file());
}

#[test]
fn restore_without_settings_removes_stale_pending_settings() {
    let fixture = Fixture::new();
    let source = fixture.backup("source");
    let target = fixture.root.join("target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("settings.restore.json"), b"stale settings").unwrap();
    fs::write(target.join("settings.json"), b"current settings").unwrap();
    prepare_restore(&source, &target).unwrap();
    assert!(!target.join("settings.restore.json").exists());
    drop(crate::db::init(&target.join("offerflow.db")).unwrap());
    assert_eq!(
        fs::read(target.join("settings.json")).unwrap(),
        b"current settings"
    );
}

#[test]
fn restore_rejects_missing_or_truncated_referenced_files() {
    let fixture = Fixture::new();
    let source = fixture.backup("source");
    let target = fixture.root.join("target");
    fs::write(source.join("resumes/resume.pdf"), b"x").unwrap();
    assert!(prepare_restore(&source, &target)
        .unwrap_err()
        .contains("备份文件不完整"));
    assert!(!target.join("offerflow_restore.db").exists());
    fs::remove_file(source.join("resumes/resume.pdf")).unwrap();
    assert!(prepare_restore(&source, &target)
        .unwrap_err()
        .contains("备份缺少文件"));
    assert!(!target.join("offerflow_restore.db").exists());
}
