//! One-way application sync through the user's authenticated native lark-cli.
use crate::db::Db;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{Manager, State};
use tauri_plugin_opener::OpenerExt;

const CONFIG: &str = "feishu-sync.json";
const STATUS: &str = "feishu-sync-status.json";
const PENDING: &str = "feishu-sync-pending.json";
const SOURCE: &str = "feishu-sync-source.json";
const CREATION: &str = "feishu-create-pending.json";
const FIELDS: [&str; 15] = [
    "公司",
    "岗位",
    "城市",
    "岗位分类",
    "薪资下限",
    "薪资上限",
    "岗位链接",
    "投递渠道",
    "投递日期",
    "当前阶段",
    "状态",
    "下次面试",
    "备注",
    "本地更新时间",
    "OfferFlow ID",
];

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FeishuSettings {
    pub auto_sync: bool,
    pub cli_path: String,
    pub profile: String,
    pub base_token: String,
    pub table_id: String,
    pub source_id: String,
    pub source_database: String,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct FeishuReport {
    pub running: bool,
    pub finished_at: Option<u64>,
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FeishuTable {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeishuTables {
    pub base_token: String,
    pub selected_table_id: Option<String>,
    pub tables: Vec<FeishuTable>,
}

#[derive(Default)]
pub struct FeishuRuntime {
    running: AtomicBool,
    stopping: AtomicBool,
    last_success: Mutex<Option<String>>,
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn read_json<T: serde::de::DeserializeOwned + Default>(path: &Path) -> Result<T, String> {
    match fs::read(path) {
        Ok(raw) => serde_json::from_slice(&raw)
            .map_err(|_| format!("无法读取同步配置：{}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e.to_string()),
    }
}

fn write_json(path: &Path, data: &impl Serialize) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    fs::write(
        &tmp,
        serde_json::to_vec_pretty(data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())
}

fn file_lock(dir: &Path) -> Result<File, String> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("feishu-sync.lock"))
        .map_err(|e| e.to_string())?;
    file.try_lock()
        .map_err(|_| "飞书同步正在运行，请稍后再试".to_string())?;
    Ok(file)
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

fn validate(settings: &FeishuSettings, dir: &Path) -> Result<(), String> {
    if !valid_id(&settings.base_token)
        || !settings.table_id.starts_with("tbl")
        || !valid_id(&settings.table_id)
        || !valid_id(&settings.source_id)
    {
        return Err("请先新建投递表或绑定已有表格".into());
    }
    if !settings.profile.is_empty() && !valid_id(&settings.profile) {
        return Err("飞书配置名称只能包含字母、数字和下划线".into());
    }
    let expected = fs::canonicalize(dir.join("offerflow.db")).map_err(|e| e.to_string())?;
    let source = fs::canonicalize(&settings.source_database)
        .map_err(|_| "同步连接的来源数据库不存在".to_string())?;
    if expected != source {
        return Err("此连接属于另一份 OfferFlow 数据库，请选择当前数据库的同步连接".into());
    }
    Ok(())
}

fn snapshot(db: &Db, settings: &FeishuSettings) -> Result<Vec<Value>, String> {
    let conn = db.0.lock();
    let mut stmt = conn.prepare("SELECT a.id,c.name,j.title,j.location,j.category,j.salary_min,j.salary_max,j.jd_url,a.channel,a.applied_at,s.name,a.status,a.notes,a.updated_at,
        (SELECT MIN(i.scheduled_at) FROM interviews i WHERE i.application_id=a.id AND i.status='待面试' AND datetime(i.scheduled_at)>=datetime('now','localtime'))
        FROM applications a JOIN jobs j ON j.id=a.job_id JOIN companies c ON c.id=j.company_id JOIN stages s ON s.id=a.stage_id ORDER BY a.id").map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let mut record = Map::new();
            for (name, column) in [
                ("公司", 1),
                ("岗位", 2),
                ("城市", 3),
                ("岗位分类", 4),
                ("岗位链接", 7),
                ("投递渠道", 8),
                ("投递日期", 9),
                ("当前阶段", 10),
                ("状态", 11),
                ("备注", 12),
                ("本地更新时间", 13),
                ("下次面试", 14),
            ] {
                record.insert(
                    name.into(),
                    json!(row.get::<_, Option<String>>(column)?.unwrap_or_default()),
                );
            }
            record.insert("薪资下限".into(), json!(row.get::<_, Option<i64>>(5)?));
            record.insert("薪资上限".into(), json!(row.get::<_, Option<i64>>(6)?));
            record.insert(
                "OfferFlow ID".into(),
                json!(format!("{}:{}", settings.source_id, row.get::<_, i64>(0)?)),
            );
            Ok(Value::Object(record))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string());
    rows
}

fn cli_path(settings: &FeishuSettings) -> Result<PathBuf, String> {
    if !settings.cli_path.is_empty() {
        let path = PathBuf::from(&settings.cli_path);
        if !path.is_file()
            || (cfg!(windows)
                && path
                    .extension()
                    .is_none_or(|ext| !ext.eq_ignore_ascii_case("exe")))
        {
            return Err("请选择 lark-cli 原生可执行文件（Windows 使用 .exe）".into());
        }
        return Ok(path);
    }
    let paths = std::env::var_os("PATH").unwrap_or_default();
    for dir in std::env::split_paths(&paths) {
        for name in [
            if cfg!(windows) {
                "lark-cli.exe"
            } else {
                "lark-cli"
            },
            "node_modules/@larksuite/cli/bin/lark-cli.exe",
        ] {
            let path = dir.join(name);
            if path.is_file() {
                return Ok(path);
            }
        }
    }
    Err("未找到飞书 CLI，请在连接设置中选择 lark-cli 可执行文件".into())
}

fn cli(settings: &FeishuSettings, args: &[String]) -> Result<Value, String> {
    let mut command = Command::new(cli_path(settings)?);
    command
        .arg("base")
        .args(args)
        .args(["--as", "user"])
        .env("LARKSUITE_CLI_NO_UPDATE_NOTIFIER", "1")
        .env("LARKSUITE_CLI_NO_SKILLS_NOTIFIER", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if !settings.profile.is_empty() {
        let home = dirs::home_dir()
            .ok_or("无法找到用户目录")?
            .join(".lark-channel");
        let profile = home.join("profiles").join(&settings.profile);
        command
            .env("LARK_CHANNEL", "1")
            .env("LARK_CHANNEL_PROFILE", &settings.profile)
            .env("LARK_CHANNEL_HOME", home)
            .env(
                "LARK_CHANNEL_CONFIG",
                profile.join("lark-cli-source/config.json"),
            )
            .env("LARKSUITE_CLI_CONFIG_DIR", profile.join("lark-cli"));
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("启动飞书 CLI 失败：{e}"))?;
    let stdout = child.stdout.take().ok_or("无法读取 CLI 输出")?;
    let stderr = child.stderr.take().ok_or("无法读取 CLI 错误")?;
    let reader = |stream: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            stream
                .take(8 * 1024 * 1024)
                .read_to_end(&mut bytes)
                .map(|_| bytes)
        })
    };
    let out = reader(Box::new(stdout));
    let err = reader(Box::new(stderr));
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < Duration::from_secs(60) => {
                std::thread::sleep(Duration::from_millis(50))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = out.join();
                let _ = err.join();
                return Err("飞书 CLI 超时或意外退出；写入结果可能未确认，请检查表格后重试".into());
            }
        }
    };
    let stdout = out
        .join()
        .map_err(|_| "读取 CLI 输出失败")?
        .map_err(|e| e.to_string())?;
    let stderr = err
        .join()
        .map_err(|_| "读取 CLI 错误失败")?
        .map_err(|e| e.to_string())?;
    let payload: Value = serde_json::from_slice(if status.success() { &stdout } else { &stderr })
        .map_err(|_| "飞书 CLI 返回了无法识别的结果".to_string())?;
    if !status.success() || payload["ok"] != true {
        let detail = &payload["error"];
        return Err(format!(
            "飞书同步失败：{} {}",
            detail["code"]
                .as_i64()
                .map(|v| v.to_string())
                .unwrap_or_default(),
            detail["message"]
                .as_str()
                .unwrap_or("请检查 CLI 登录状态和表格权限")
        ));
    }
    payload
        .get("data")
        .cloned()
        .ok_or("飞书 CLI 响应缺少 data".into())
}

fn arguments(command: &str, settings: &FeishuSettings) -> Vec<String> {
    [
        command,
        "--base-token",
        &settings.base_token,
        "--table-id",
        &settings.table_id,
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn field_schema() -> Value {
    json!(FIELDS.iter().map(|name| json!({"name": name, "type": if name.starts_with("薪资") { "number" } else { "text" }})).collect::<Vec<_>>())
}

fn connection_options(cli_path: String, profile: String) -> Result<FeishuSettings, String> {
    let settings = FeishuSettings {
        cli_path: cli_path.trim().into(),
        profile: profile.trim().into(),
        ..Default::default()
    };
    if !settings.profile.is_empty() && !valid_id(&settings.profile) {
        return Err("飞书配置名称只能包含字母、数字和下划线".into());
    }
    Ok(settings)
}

fn validate_table_url(raw: &str) -> Result<(), String> {
    let url = url::Url::parse(raw).map_err(|_| "请输入完整的飞书多维表格链接")?;
    let host = url.host_str().unwrap_or_default();
    let allowed = ["feishu.cn", "larkoffice.com", "larksuite.com"]
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")));
    let path = url.path();
    if !allowed
        || url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || !(path.starts_with("/base/") || path.starts_with("/wiki/"))
    {
        return Err("请使用飞书多维表格的 /base/ 或 /wiki/ 链接".into());
    }
    Ok(())
}

fn resolve_tables(
    raw: &str,
    call: &mut impl FnMut(&[String]) -> Result<Value, String>,
) -> Result<FeishuTables, String> {
    validate_table_url(raw)?;
    let resolved = call(&["+url-resolve".into(), "--url".into(), raw.into()])?;
    let base = resolved["base_token"]
        .as_str()
        .filter(|id| valid_id(id))
        .ok_or("链接未指向多维表格")?;
    let result = call(&["+table-list".into(), "--base-token".into(), base.into()])?;
    let tables: Vec<FeishuTable> =
        serde_json::from_value(result["tables"].clone()).map_err(|_| "无法读取数据表列表")?;
    if tables.is_empty()
        || tables
            .iter()
            .any(|table| !table.id.starts_with("tbl") || !valid_id(&table.id))
    {
        return Err("此多维表格中没有可用的数据表".into());
    }
    let selected = resolved["table_id"]
        .as_str()
        .filter(|id| tables.iter().any(|table| table.id == *id))
        .map(str::to_string)
        .or_else(|| (tables.len() == 1).then(|| tables[0].id.clone()));
    Ok(FeishuTables {
        base_token: base.into(),
        selected_table_id: selected,
        tables,
    })
}

fn add_missing_fields(
    settings: &FeishuSettings,
    call: &mut impl FnMut(&[String]) -> Result<Value, String>,
) -> Result<(), String> {
    let schema = call(&arguments("+field-list", settings))?;
    let existing = schema["fields"].as_array().ok_or("无法读取飞书字段定义")?;
    let mut missing = Vec::new();
    for field in field_schema().as_array().ok_or("字段定义无效")? {
        if let Some(actual) = existing
            .iter()
            .find(|actual| actual["name"] == field["name"])
        {
            if actual["type"] != field["type"] {
                return Err(format!(
                    "字段「{}」应为 {} 类型，请在飞书调整后重新绑定",
                    field["name"].as_str().unwrap_or_default(),
                    field["type"].as_str().unwrap_or_default()
                ));
            }
        } else {
            missing.push(field.clone());
        }
    }
    if !missing.is_empty() {
        let mut args = arguments("+field-create", settings);
        args.extend(["--json".into(), json!(missing).to_string()]);
        call(&args)?;
    }
    Ok(())
}

fn preserve_source(settings: &mut FeishuSettings, dir: &Path) -> Result<(), String> {
    let previous: FeishuSettings = read_json(&dir.join(CONFIG))?;
    let stored: Option<String> = read_json(&dir.join(SOURCE))?;
    settings.source_id = if let Some(source) = stored.filter(|id| valid_id(id)) {
        source
    } else if valid_id(&previous.source_id) {
        previous.source_id
    } else {
        format!(
            "{:x}{:x}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos(),
            std::process::id()
        )
    };
    settings.source_database = dir.join("offerflow.db").to_string_lossy().into_owned();
    // Save before the network write so a failed attempt keeps the same identity.
    write_json(&dir.join(SOURCE), &Some(&settings.source_id))
}

fn finish_connection(settings: &FeishuSettings, dir: &Path) -> Result<(), String> {
    validate(settings, dir)?;
    write_json(&dir.join(CONFIG), settings)?;
    for file in [STATUS, CREATION] {
        match fs::remove_file(dir.join(file)) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

fn require_settled(dir: &Path) -> Result<(), String> {
    if dir.join(PENDING).exists() {
        return Err("上次新增记录尚未确认，请先同步当前表格，再更换连接".into());
    }
    Ok(())
}

fn create_table(
    settings: &mut FeishuSettings,
    name: &str,
    dir: &Path,
    retry: bool,
    call: &mut impl FnMut(&[String]) -> Result<Value, String>,
) -> Result<(), String> {
    if name.trim().is_empty() || name.chars().count() > 100 {
        return Err("表格名称应为 1 至 100 个字符".into());
    }
    if dir.join(CREATION).exists() && !retry {
        return Err("上次建表结果未确认。请先查看飞书云盘，已有表格可直接绑定。".into());
    }
    write_json(&dir.join(CREATION), &json!({"name": name}))?;
    let result = call(&[
        "+base-create".into(),
        "--name".into(),
        name.trim().into(),
        "--table-name".into(),
        "投递记录".into(),
        "--fields".into(),
        field_schema().to_string(),
        "--time-zone".into(),
        "Asia/Shanghai".into(),
    ])?;
    write_json(&dir.join(CREATION), &result)?;
    settings.base_token = result["base"]["base_token"]
        .as_str()
        .filter(|id| valid_id(id))
        .ok_or("建表返回的链接无效，请在飞书云盘查看")?
        .into();
    settings.table_id = result["table"]["id"]
        .as_str()
        .filter(|id| id.starts_with("tbl") && valid_id(id))
        .ok_or("建表返回的数据表 ID 无效，请在飞书云盘查看")?
        .into();
    finish_connection(settings, dir)
}

#[tauri::command(async)]
pub fn list_feishu_tables(
    url: String,
    cli_path: String,
    profile: String,
) -> Result<FeishuTables, String> {
    let options = connection_options(cli_path, profile)?;
    resolve_tables(url.trim(), &mut |args| cli(&options, args))
}

#[tauri::command(async)]
pub fn create_feishu_table(
    name: String,
    cli_path: String,
    profile: String,
    retry: bool,
) -> Result<FeishuSettings, String> {
    let dir = crate::db::default_db_dir();
    let _lock = file_lock(&dir)?;
    require_settled(&dir)?;
    let mut settings = connection_options(cli_path, profile)?;
    let options = settings.clone();
    self::cli_path(&options)?;
    preserve_source(&mut settings, &dir)?;
    create_table(&mut settings, &name, &dir, retry, &mut |args| {
        cli(&options, args)
    })?;
    Ok(settings)
}

#[tauri::command(async)]
pub fn bind_feishu_table(
    url: String,
    table_id: String,
    cli_path: String,
    profile: String,
) -> Result<FeishuSettings, String> {
    let dir = crate::db::default_db_dir();
    let _lock = file_lock(&dir)?;
    require_settled(&dir)?;
    let mut settings = connection_options(cli_path, profile)?;
    let target = resolve_tables(url.trim(), &mut |args| cli(&settings, args))?;
    if !target.tables.iter().any(|table| table.id == table_id) {
        return Err("请选择链接中的数据表".into());
    }
    settings.base_token = target.base_token;
    settings.table_id = table_id;
    preserve_source(&mut settings, &dir)?;
    add_missing_fields(&settings, &mut |args| cli(&settings, args))?;
    finish_connection(&settings, &dir)?;
    Ok(settings)
}

type Remote = HashMap<String, (String, Value)>;
fn read_remote(
    settings: &FeishuSettings,
    call: &mut impl FnMut(&[String]) -> Result<Value, String>,
) -> Result<Remote, String> {
    let mut result = HashMap::new();
    let mut offset = 0;
    loop {
        let mut args = arguments("+record-list", settings);
        args.extend([
            "--format".into(),
            "json".into(),
            "--limit".into(),
            "200".into(),
            "--offset".into(),
            offset.to_string(),
        ]);
        let page = call(&args)?;
        let fields = page["fields"].as_array().ok_or("飞书字段矩阵缺失")?;
        let ids = page["record_id_list"]
            .as_array()
            .ok_or("飞书记录 ID 缺失")?;
        let rows = page["data"].as_array().ok_or("飞书记录矩阵缺失")?;
        if ids.len() != rows.len() {
            return Err("飞书返回的记录数量不一致".into());
        }
        for (id, row) in ids.iter().zip(rows) {
            let row = row.as_array().ok_or("飞书记录格式异常")?;
            if row.len() != fields.len() {
                return Err("飞书记录字段不完整".into());
            }
            let record = fields
                .iter()
                .zip(row)
                .map(|(key, value)| {
                    Ok((key.as_str().ok_or("字段名无效")?.to_string(), value.clone()))
                })
                .collect::<Result<Map<_, _>, String>>()?;
            if let Some(key) = record
                .get("OfferFlow ID")
                .and_then(Value::as_str)
                .filter(|key| key.starts_with(&format!("{}:", settings.source_id)))
            {
                if result
                    .insert(
                        key.to_string(),
                        (
                            id.as_str().ok_or("记录 ID 无效")?.into(),
                            Value::Object(record),
                        ),
                    )
                    .is_some()
                {
                    return Err("飞书表格中有重复的 OfferFlow ID，请先处理重复记录".into());
                }
            }
        }
        if page["has_more"] != true {
            return Ok(result);
        }
        if rows.is_empty() || offset >= 100_000 {
            return Err("飞书分页异常，已停止同步".into());
        }
        offset += rows.len();
    }
}

fn equivalent(expected: &Value, actual: &Value) -> bool {
    expected.as_object().is_some_and(|fields| {
        fields.iter().all(|(key, value)| {
            if let Some(text) = value.as_str() {
                actual[key].as_str().unwrap_or_default() == text
            } else {
                actual.get(key) == Some(value)
            }
        })
    })
}

fn sync_data(
    settings: &FeishuSettings,
    rows: &[Value],
    dir: &Path,
    report: &mut FeishuReport,
    call: &mut impl FnMut(&[String]) -> Result<Value, String>,
) -> Result<(), String> {
    let schema = call(&arguments("+field-list", settings))?;
    let fields = schema["fields"].as_array().ok_or("无法读取飞书字段定义")?;
    for name in FIELDS {
        let ty = if ["薪资下限", "薪资上限"].contains(&name) {
            "number"
        } else {
            "text"
        };
        if !fields.iter().any(|f| f["name"] == name && f["type"] == ty) {
            return Err(format!("飞书字段「{name}」缺失或类型改变，请检查表格"));
        }
    }
    let remote = read_remote(settings, call)?;
    let pending: Option<String> = read_json(&dir.join(PENDING))?;
    if let Some(key) = pending {
        if !remote.contains_key(&key) {
            return Err(format!(
                "记录 {key} 上次新增结果未确认，请先核对飞书表格；未自动重建"
            ));
        }
        fs::remove_file(dir.join(PENDING)).map_err(|e| e.to_string())?;
    }
    for row in rows {
        let key = row["OfferFlow ID"].as_str().ok_or("本地来源 ID 缺失")?;
        let existing = remote.get(key);
        if existing.is_some_and(|(_, actual)| equivalent(row, actual)) {
            report.unchanged += 1;
            continue;
        }
        let mut args = arguments("+record-upsert", settings);
        args.extend(["--json".into(), row.to_string()]);
        if let Some((id, _)) = existing {
            args.extend(["--record-id".into(), id.clone()]);
        } else {
            write_json(&dir.join(PENDING), &Some(key))?;
        }
        let result = call(&args)?;
        if existing.is_some() {
            if result["updated"] != true {
                return Err("飞书更新结果异常，请核对表格".into());
            }
            report.updated += 1;
        } else {
            if result["created"] != true
                || result["record"]["record_id_list"]
                    .as_array()
                    .is_none_or(|ids| ids.len() != 1 || ids[0].as_str().is_none())
            {
                return Err("飞书新增结果异常，请核对表格".into());
            }
            fs::remove_file(dir.join(PENDING)).map_err(|e| e.to_string())?;
            report.created += 1;
        }
    }
    Ok(())
}

struct Running<'a>(&'a AtomicBool);
impl Drop for Running<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

fn run_sync(db: &Db, runtime: &FeishuRuntime, dir: &Path) -> Result<FeishuReport, String> {
    if runtime.running.swap(true, Ordering::SeqCst) {
        return Err("飞书同步正在运行，请稍后再试".into());
    }
    let _running = Running(&runtime.running);
    let _lock = file_lock(dir)?;
    let mut report = FeishuReport::default();
    let result = (|| {
        let settings: FeishuSettings = read_json(&dir.join(CONFIG))?;
        validate(&settings, dir)?;
        let rows = snapshot(db, &settings)?;
        sync_data(&settings, &rows, dir, &mut report, &mut |args| {
            cli(&settings, args)
        })?;
        *runtime
            .last_success
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(json!([settings, rows]).to_string());
        Ok::<_, String>(())
    })();
    report.finished_at = Some(now());
    report.error = result.err();
    write_json(&dir.join(STATUS), &report)?;
    match &report.error {
        Some(error) => Err(error.clone()),
        None => Ok(report),
    }
}

pub fn start_auto_sync(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(15));
        let runtime = app.state::<FeishuRuntime>();
        if runtime.stopping.load(Ordering::SeqCst) {
            break;
        }
        if runtime.running.load(Ordering::SeqCst) {
            continue;
        }
        let dir = crate::db::default_db_dir();
        let Ok(settings) = read_json::<FeishuSettings>(&dir.join(CONFIG)) else {
            continue;
        };
        if !settings.auto_sync || settings.base_token.is_empty() {
            continue;
        }
        // A failed request is retried at most every five minutes, including auth errors.
        if let Ok(report) = read_json::<FeishuReport>(&dir.join(STATUS)) {
            if report.error.is_some()
                && report
                    .finished_at
                    .is_some_and(|time| now().saturating_sub(time) < 300_000)
            {
                continue;
            }
        }
        let db = app.state::<Db>();
        let unchanged = snapshot(&db, &settings).is_ok_and(|rows| {
            runtime
                .last_success
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
                == Some(&json!([settings, rows]).to_string())
        });
        if !unchanged {
            let _ = run_sync(&db, &runtime, &dir);
        }
    });
}

pub fn stop(runtime: &FeishuRuntime) {
    runtime.stopping.store(true, Ordering::SeqCst);
}

#[tauri::command(async)]
pub fn get_feishu_settings() -> Result<FeishuSettings, String> {
    read_json(&crate::db::default_db_dir().join(CONFIG))
}

#[tauri::command(async)]
pub fn set_feishu_settings(settings: FeishuSettings) -> Result<(), String> {
    let dir = crate::db::default_db_dir();
    let _lock = file_lock(&dir)?;
    validate(&settings, &dir)?;
    write_json(&dir.join(CONFIG), &settings)
}

#[tauri::command(async)]
pub fn get_feishu_status(runtime: State<FeishuRuntime>) -> Result<FeishuReport, String> {
    let mut report: FeishuReport = read_json(&crate::db::default_db_dir().join(STATUS))?;
    report.running = runtime.running.load(Ordering::SeqCst);
    Ok(report)
}

#[tauri::command(async)]
pub fn sync_feishu_now(
    db: State<Db>,
    runtime: State<FeishuRuntime>,
) -> Result<FeishuReport, String> {
    run_sync(&db, &runtime, &crate::db::default_db_dir())
}

#[tauri::command(async)]
pub fn open_feishu_table(app: tauri::AppHandle) -> Result<(), String> {
    let settings: FeishuSettings = read_json(&crate::db::default_db_dir().join(CONFIG))?;
    if !valid_id(&settings.base_token) {
        return Err("尚未连接飞书表格".into());
    }
    app.opener()
        .open_url(
            format!(
                "https://my.feishu.cn/base/{}?table={}",
                settings.base_token, settings.table_id
            ),
            None::<&str>,
        )
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn resolve_preserves_table_selection_and_requires_choice_for_multiple_tables() {
        let mut calls = 0;
        let result = resolve_tables(
            "https://example.feishu.cn/base/base1?table=tbl2",
            &mut |_| {
                calls += 1;
                Ok(if calls == 1 {
                    json!({"base_token":"base1","table_id":"tbl2"})
                } else {
                    json!({"tables":[{"id":"tbl1","name":"One"},{"id":"tbl2","name":"Two"}]})
                })
            },
        )
        .unwrap();
        assert_eq!(result.selected_table_id.as_deref(), Some("tbl2"));
        let mut calls = 0;
        let result = resolve_tables("https://example.feishu.cn/base/base1", &mut |_| {
            calls += 1;
            Ok(if calls == 1 {
                json!({"base_token":"base1"})
            } else {
                json!({"tables":[{"id":"tbl1","name":"One"},{"id":"tbl2","name":"Two"}]})
            })
        })
        .unwrap();
        assert!(result.selected_table_id.is_none());
        assert!(validate_table_url("https://feishu.cn.example.com/base/base1").is_err());
        assert!(validate_table_url("https://example.feishu.cn/docx/abc").is_err());
    }

    #[test]
    fn bind_adds_only_missing_fields_and_rejects_conflicts_before_writing() {
        let settings = FeishuSettings::default();
        let mut writes = Vec::new();
        add_missing_fields(&settings, &mut |args| {
            if args[0] == "+field-list" { Ok(json!({"fields":[{"name":"公司","type":"text"},{"name":"原有内容","type":"text"}]})) }
            else { writes.push(serde_json::from_str::<Value>(args.last().unwrap()).unwrap()); Ok(json!({})) }
        }).unwrap();
        assert_eq!(writes.len(), 1);
        assert_eq!(writes[0].as_array().unwrap().len(), 14);
        assert!(!writes[0]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["name"] == "公司" || f["name"] == "原有内容"));
        let mut calls = 0;
        assert!(add_missing_fields(&settings, &mut |_| {
            calls += 1;
            Ok(json!({"fields":[{"name":"公司","type":"number"}]}))
        })
        .is_err());
        assert_eq!(calls, 1);
    }

    #[test]
    fn reconnect_keeps_legacy_source_and_existing_settings_on_failed_create() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("offerflow.db"), []).unwrap();
        let previous = FeishuSettings {
            source_id: "legacy123".into(),
            base_token: "oldBase".into(),
            ..Default::default()
        };
        write_json(&dir.path().join(CONFIG), &previous).unwrap();
        let mut next = FeishuSettings::default();
        preserve_source(&mut next, dir.path()).unwrap();
        assert_eq!(next.source_id, "legacy123");
        let mut count = 0;
        for _ in 0..2 {
            assert!(
                create_table(&mut next, "Offers", dir.path(), false, &mut |_| {
                    count += 1;
                    Err("timeout".into())
                })
                .is_err()
            );
        }
        assert_eq!(count, 1);
        assert_eq!(
            read_json::<FeishuSettings>(&dir.path().join(CONFIG)).unwrap(),
            previous
        );
        create_table(&mut next, "Offers", dir.path(), true, &mut |_| {
            Ok(json!({"base":{"base_token":"newBase"},"table":{"id":"tblNew"}}))
        })
        .unwrap();
        assert_eq!(
            read_json::<FeishuSettings>(&dir.path().join(CONFIG))
                .unwrap()
                .source_id,
            "legacy123"
        );
        assert!(!dir.path().join(CREATION).exists());
    }

    #[test]
    fn fresh_connection_uses_default_cli_and_keeps_its_source_on_retry() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = connection_options(String::new(), String::new()).unwrap();
        assert!(settings.profile.is_empty());
        preserve_source(&mut settings, dir.path()).unwrap();
        let source = settings.source_id.clone();
        let mut retry = FeishuSettings::default();
        preserve_source(&mut retry, dir.path()).unwrap();
        assert_eq!(retry.source_id, source);
        assert!(connection_options(String::new(), "../other".into()).is_err());
    }
    fn schema() -> Value {
        json!({"fields": FIELDS.iter().map(|name| json!({"name":name,"type":if name.starts_with("薪资") {"number"} else {"text"}})).collect::<Vec<_>>()})
    }
    fn page(rows: Vec<(&str, &str, &str)>) -> Value {
        json!({"fields":["OfferFlow ID","公司"], "data":rows.iter().map(|(_,key,name)| json!([key,name])).collect::<Vec<_>>(),
            "record_id_list":rows.iter().map(|(id,_,_)| id).collect::<Vec<_>>(), "has_more":false})
    }
    #[test]
    fn existing_rows_update_in_place_and_identical_rows_are_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let settings = FeishuSettings {
            source_id: "source".into(),
            ..Default::default()
        };
        let mut report = FeishuReport::default();
        let mut writes = Vec::new();
        sync_data(
            &settings,
            &[
                json!({"OfferFlow ID":"source:1","公司":"new"}),
                json!({"OfferFlow ID":"source:2","公司":"same"}),
            ],
            dir.path(),
            &mut report,
            &mut |args| match args[0].as_str() {
                "+field-list" => Ok(schema()),
                "+record-list" => Ok(page(vec![
                    ("rec1", "source:1", "old"),
                    ("rec2", "source:2", "same"),
                ])),
                _ => {
                    writes.push(args.to_vec());
                    Ok(json!({"updated":true,"record":{"update":{}}}))
                }
            },
        )
        .unwrap();
        assert_eq!(
            (report.created, report.updated, report.unchanged),
            (0, 1, 1)
        );
        assert_eq!(writes.len(), 1);
        assert_eq!(&writes[0][writes[0].len() - 2..], &["--record-id", "rec1"]);
    }
    #[test]
    fn ambiguous_create_does_not_retry_until_remote_record_is_found() {
        let dir = tempfile::tempdir().unwrap();
        let settings = FeishuSettings {
            source_id: "source".into(),
            ..Default::default()
        };
        let rows = [json!({"OfferFlow ID":"source:1","公司":"new"})];
        let mut creates = 0;
        for _ in 0..2 {
            assert!(sync_data(
                &settings,
                &rows,
                dir.path(),
                &mut FeishuReport::default(),
                &mut |args| match args[0].as_str() {
                    "+field-list" => Ok(schema()),
                    "+record-list" => Ok(page(vec![])),
                    _ => {
                        creates += 1;
                        Err("timeout".into())
                    }
                }
            )
            .is_err());
        }
        assert_eq!(creates, 1);
        let mut report = FeishuReport::default();
        sync_data(
            &settings,
            &rows,
            dir.path(),
            &mut report,
            &mut |args| match args[0].as_str() {
                "+field-list" => Ok(schema()),
                _ => Ok(page(vec![("rec1", "source:1", "new")])),
            },
        )
        .unwrap();
        assert_eq!(report.unchanged, 1);
        assert!(!dir.path().join(PENDING).exists());
    }
    #[test]
    fn duplicates_and_schema_drift_stop_before_writes() {
        let dir = tempfile::tempdir().unwrap();
        let settings = FeishuSettings {
            source_id: "source".into(),
            ..Default::default()
        };
        let error = sync_data(
            &settings,
            &[],
            dir.path(),
            &mut FeishuReport::default(),
            &mut |args| match args[0].as_str() {
                "+field-list" => Ok(schema()),
                _ => Ok(page(vec![("r1", "source:1", "a"), ("r2", "source:1", "b")])),
            },
        )
        .unwrap_err();
        assert!(error.contains("重复"));
        let error = sync_data(
            &settings,
            &[],
            dir.path(),
            &mut FeishuReport::default(),
            &mut |_| Ok(json!({"fields":[]})),
        )
        .unwrap_err();
        assert!(error.contains("字段"));
    }
    #[test]
    fn exclusive_lock_is_released_after_drop() {
        let dir = tempfile::tempdir().unwrap();
        let lock = file_lock(dir.path()).unwrap();
        assert!(file_lock(dir.path()).is_err());
        drop(lock);
        assert!(file_lock(dir.path()).is_ok());
    }
}
