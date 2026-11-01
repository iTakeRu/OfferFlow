use crate::db::Db;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tauri::State;

const KINDS: [&str; 7] = [
    "work",
    "education",
    "project",
    "research",
    "portfolio",
    "text",
    "other",
];

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS experiences (
        id TEXT PRIMARY KEY NOT NULL,
        title TEXT NOT NULL,
        kind TEXT NOT NULL,
        fields TEXT NOT NULL DEFAULT '[]',
        versions TEXT NOT NULL,
        revision INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')),
        updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime'))
    );",
    )?;
    let has_fields: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pragma_table_info('experiences') WHERE name = 'fields'",
        [],
        |r| r.get(0),
    )?;
    if has_fields > 0 {
        return Ok(());
    }
    // 旧版每个版本各存一套字段；改为整段经历共用一套，版本只保留描述。
    conn.execute(
        "ALTER TABLE experiences ADD COLUMN fields TEXT NOT NULL DEFAULT '[]'",
        [],
    )?;
    let rows: Vec<(String, String, String)> = {
        let mut stmt = conn.prepare("SELECT id,kind,versions FROM experiences")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    let json_err = |e: serde_json::Error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    };
    for (id, kind, json) in rows {
        let mut item = Experience {
            id,
            title: String::new(),
            kind,
            fields: vec![],
            versions: serde_json::from_str(&json).map_err(json_err)?,
            revision: 0,
        };
        normalize(&mut item);
        conn.execute(
            "UPDATE experiences SET kind=?2,fields=?3,versions=?4 WHERE id=?1",
            params![
                item.id,
                item.kind,
                serde_json::to_string(&item.fields).map_err(json_err)?,
                serde_json::to_string(&item.versions).map_err(json_err)?
            ],
        )?;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    pub label: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub source: String,
    /// 仅用于读取旧格式（字段存在版本里）的数据，保存时会并入经历本身。
    #[serde(default, skip_serializing)]
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experience {
    pub id: String,
    pub title: String,
    pub kind: String,
    #[serde(default)]
    pub fields: Vec<Field>,
    pub versions: Vec<Version>,
    #[serde(default)]
    pub revision: i64,
}

/// 把旧格式转换成当前格式：合并已取消的分类；字段取第一个版本的，
/// 其他版本里不同的字段值写到该版本描述开头，不丢内容。
fn normalize(item: &mut Experience) {
    if matches!(item.kind.as_str(), "summary" | "skills" | "game") {
        item.kind = "text".into();
    }
    if item.fields.is_empty() {
        if let Some(first) = item.versions.first() {
            item.fields = first.fields.clone();
        }
    }
    for v in &mut item.versions {
        let extra: Vec<String> = v
            .fields
            .drain(..)
            .filter(|f| !f.value.trim().is_empty() && !item.fields.contains(f))
            .map(|f| format!("{}：{}", f.label, f.value))
            .collect();
        if !extra.is_empty() {
            let mut text = extra.join("\n");
            if !v.description.is_empty() {
                text.push_str("\n\n");
                text.push_str(&v.description);
            }
            v.description = text;
        }
    }
}

fn validate(item: &Experience) -> Result<(), String> {
    if item.id.is_empty()
        || item.id.len() > 100
        || item.title.trim().is_empty()
        || item.title.chars().count() > 200
    {
        return Err("经历名称不能为空且不能超过 200 字".into());
    }
    if !KINDS.contains(&item.kind.as_str()) {
        return Err("经历类型无效".into());
    }
    if item.fields.len() > 30 {
        return Err("字段不能超过 30 个".into());
    }
    let mut labels = HashSet::new();
    for f in &item.fields {
        if f.label.trim().is_empty()
            || f.label.chars().count() > 100
            || f.value.chars().count() > 10000
            || !labels.insert(f.label.trim().to_lowercase())
        {
            return Err("字段名称不能为空或重复，内容不能过长".into());
        }
    }
    for label in ["开始时间", "结束时间"] {
        if let Some(f) = item
            .fields
            .iter()
            .find(|f| f.label == label && !f.value.is_empty() && f.value != "至今")
        {
            let b = f.value.as_bytes();
            if b.len() != 7
                || b[4] != b'-'
                || !b[..4].iter().all(u8::is_ascii_digit)
                || !b[5..].iter().all(u8::is_ascii_digit)
                || !("01"..="12").contains(&&f.value[5..])
            {
                return Err("时间请使用 YYYY-MM 格式，结束时间也可填写至今".into());
            }
        }
    }
    let value = |label: &str| {
        item.fields
            .iter()
            .find(|f| f.label == label)
            .map(|f| f.value.as_str())
            .unwrap_or("")
    };
    let (start, end) = (value("开始时间"), value("结束时间"));
    if start == "至今" || (!start.is_empty() && !end.is_empty() && end != "至今" && start > end)
    {
        return Err("开始时间不能晚于结束时间".into());
    }
    if item.versions.is_empty() || item.versions.len() > 50 {
        return Err("每段经历需要 1 至 50 个版本".into());
    }
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for v in &item.versions {
        if v.id.is_empty() || v.id.len() > 100 || !ids.insert(&v.id) {
            return Err("版本标识无效或重复".into());
        }
        if v.name.trim().is_empty()
            || v.name.chars().count() > 100
            || !names.insert(v.name.trim().to_lowercase())
        {
            return Err("版本名称不能为空或重复，且不能超过 100 字".into());
        }
        if v.description.chars().count() > 50000 || v.source.chars().count() > 1000 {
            return Err("版本内容过长".into());
        }
    }
    Ok(())
}

fn from_row(row: &Row) -> rusqlite::Result<Experience> {
    let json = |i: usize| -> rusqlite::Result<String> { row.get(i) };
    let parse_err = |i: usize, e: serde_json::Error| {
        rusqlite::Error::FromSqlConversionFailure(i, rusqlite::types::Type::Text, Box::new(e))
    };
    Ok(Experience {
        id: row.get(0)?,
        title: row.get(1)?,
        kind: row.get(2)?,
        fields: serde_json::from_str(&json(3)?).map_err(|e| parse_err(3, e))?,
        versions: serde_json::from_str(&json(4)?).map_err(|e| parse_err(4, e))?,
        revision: row.get(5)?,
    })
}

fn list(conn: &Connection) -> Result<Vec<Experience>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id,title,kind,fields,versions,revision FROM experiences ORDER BY created_at,rowid",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], from_row).map_err(|e| e.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|e| e.to_string())
}

fn save(conn: &Connection, mut item: Experience) -> Result<Experience, String> {
    normalize(&mut item);
    validate(&item)?;
    item.title = item.title.trim().to_string();
    for v in &mut item.versions {
        v.name = v.name.trim().to_string();
    }
    for f in &mut item.fields {
        f.label = f.label.trim().to_string();
    }
    let fields = serde_json::to_string(&item.fields).map_err(|e| e.to_string())?;
    let versions = serde_json::to_string(&item.versions).map_err(|e| e.to_string())?;
    if item.revision == 0 {
        conn.execute(
            "INSERT INTO experiences(id,title,kind,fields,versions) VALUES(?1,?2,?3,?4,?5)",
            params![item.id, item.title, item.kind, fields, versions],
        )
        .map_err(|e| e.to_string())?;
    } else {
        let changed = conn.execute("UPDATE experiences SET title=?2,kind=?3,fields=?4,versions=?5,revision=revision+1,updated_at=datetime('now','localtime') WHERE id=?1 AND revision=?6", params![item.id, item.title, item.kind, fields, versions, item.revision]).map_err(|e| e.to_string())?;
        if changed == 0 {
            return Err("经历已被修改或删除，请关闭编辑并刷新后重试".into());
        }
    }
    item.revision += 1;
    Ok(item)
}

fn remove(conn: &Connection, id: &str, revision: i64) -> Result<(), String> {
    let changed = conn
        .execute(
            "DELETE FROM experiences WHERE id=?1 AND revision=?2",
            params![id, revision],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("经历已被修改或删除，请刷新后重试".into());
    }
    Ok(())
}

fn import(conn: &mut Connection, mut items: Vec<Experience>) -> Result<usize, String> {
    if items.len() > 500 {
        return Err("一次最多导入 500 段经历".into());
    }
    for item in &mut items {
        normalize(item);
        validate(item)?;
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    let mut count = 0;
    for mut item in items {
        let exists: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM experiences WHERE id=?1)",
                [&item.id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        if exists {
            continue;
        }
        item.revision = 0;
        save(&tx, item)?;
        count += 1;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
pub fn list_experiences(state: State<Db>) -> Result<Vec<Experience>, String> {
    list(&state.0.lock())
}

#[tauri::command]
pub fn save_experience(state: State<Db>, item: Experience) -> Result<Experience, String> {
    save(&state.0.lock(), item)
}

#[tauri::command]
pub fn delete_experience(state: State<Db>, id: String, revision: i64) -> Result<(), String> {
    remove(&state.0.lock(), &id, revision)
}

#[tauri::command]
pub fn import_experiences(state: State<Db>, items: Vec<Experience>) -> Result<usize, String> {
    import(&mut state.0.lock(), items)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn field(label: &str, value: &str) -> Field {
        Field {
            label: label.into(),
            value: value.into(),
        }
    }
    fn version(id: &str, name: &str, description: &str) -> Version {
        Version {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            source: "local.pdf".into(),
            fields: vec![],
        }
    }
    fn sample() -> Experience {
        Experience {
            id: "example".into(),
            title: "Internship".into(),
            kind: "work".into(),
            revision: 0,
            fields: vec![field("公司名称", "Example")],
            versions: vec![version("v1", "General", "Original\nSecond line")],
        }
    }
    fn database() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        migrate(&c).unwrap();
        c
    }
    #[test]
    fn independent_versions_roundtrip_and_stale_writes_are_rejected() {
        let c = database();
        let original = save(&c, sample()).unwrap();
        let mut changed = original.clone();
        changed.versions.push(version("v2", "Game", "Different"));
        let saved = save(&c, changed).unwrap();
        let loaded = list(&c).unwrap().remove(0);
        assert_eq!(loaded.fields, vec![field("公司名称", "Example")]);
        assert_eq!(loaded.versions[0].description, "Original\nSecond line");
        assert_eq!(loaded.versions[1].description, "Different");
        assert!(save(&c, original.clone()).is_err());
        assert!(remove(&c, &saved.id, original.revision).is_err());
        remove(&c, &saved.id, saved.revision).unwrap();
        assert!(list(&c).unwrap().is_empty());
    }
    #[test]
    fn import_is_atomic_and_repeated_import_preserves_edits() {
        let mut c = database();
        let mut bad = sample();
        bad.id = "bad".into();
        bad.versions.clear();
        assert!(import(&mut c, vec![sample(), bad]).is_err());
        assert!(list(&c).unwrap().is_empty());
        assert_eq!(import(&mut c, vec![sample()]).unwrap(), 1);
        let mut edited = list(&c).unwrap().remove(0);
        edited.title = "Edited".into();
        save(&c, edited).unwrap();
        assert_eq!(import(&mut c, vec![sample()]).unwrap(), 0);
        assert_eq!(list(&c).unwrap()[0].title, "Edited");
    }
    #[test]
    fn rejects_duplicate_versions_and_bad_dates() {
        let mut e = sample();
        e.versions.push(e.versions[0].clone());
        assert!(validate(&e).is_err());
        e.versions.pop();
        e.fields = vec![field("开始时间", "2026-13")];
        assert!(validate(&e).is_err());
        e.fields[0].value = "2026-06".into();
        e.fields.push(field("结束时间", "2026-02"));
        assert!(validate(&e).is_err());
        e.fields[1].value = "至今".into();
        assert!(validate(&e).is_ok());
    }
    #[test]
    fn legacy_per_version_fields_are_lifted_without_losing_values() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch(
            "CREATE TABLE experiences (id TEXT PRIMARY KEY NOT NULL, title TEXT NOT NULL, kind TEXT NOT NULL, versions TEXT NOT NULL, revision INTEGER NOT NULL DEFAULT 1, created_at TEXT NOT NULL DEFAULT (datetime('now','localtime')), updated_at TEXT NOT NULL DEFAULT (datetime('now','localtime')));",
        )
        .unwrap();
        let versions = r#"[{"id":"v1","name":"General","fields":[{"label":"公司名称","value":"A"},{"label":"部门","value":"X"}],"description":"one","source":""},{"id":"v2","name":"Game","fields":[{"label":"公司名称","value":"A"},{"label":"部门","value":"Y"}],"description":"two","source":""}]"#;
        c.execute(
            "INSERT INTO experiences(id,title,kind,versions) VALUES('e1','Example','skills',?1)",
            [versions],
        )
        .unwrap();
        migrate(&c).unwrap();
        migrate(&c).unwrap();
        let item = list(&c).unwrap().remove(0);
        assert_eq!(item.kind, "text");
        assert_eq!(
            item.fields,
            vec![field("公司名称", "A"), field("部门", "X")]
        );
        assert_eq!(item.versions[0].description, "one");
        assert_eq!(item.versions[1].description, "部门：Y\n\ntwo");
    }
    #[test]
    fn legacy_import_format_is_accepted() {
        let mut c = database();
        let legacy: Vec<Experience> = serde_json::from_str(r#"[{"id":"old","title":"Old","kind":"game","versions":[{"id":"v1","name":"General","fields":[{"label":"游戏平台","value":"PC"}],"description":"text"}]}]"#).unwrap();
        assert_eq!(import(&mut c, legacy).unwrap(), 1);
        let item = list(&c).unwrap().remove(0);
        assert_eq!((item.kind.as_str(), item.fields.len()), ("text", 1));
    }
}
