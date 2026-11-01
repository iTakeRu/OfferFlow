//! JD 网页快照：把投递时看到的岗位页面原样留存下来。
//!
//! 设计约束：
//! - 只做普通的页面访问，或读取用户自己在浏览器里「另存为」的文件；
//!   不调用站点私有接口、不做任何解密。
//! - 快照一经保存就不再修改（只能删除）——岗位下架后，这是回看 JD 的唯一依据。
//! - 抓取与保存分两步：抓到的页面先放进 `snapshots/pending/`，
//!   用户确认岗位信息、点了保存才正式入库；取消或异常退出的残留在下次启动时清掉。

use crate::db::Db;
use crate::extract::{decode_body, fetch_page_html, url_host, validate_job_url};
use crate::html::{html_to_text, Page};
use crate::job_page::{detect_job, DetailSource, JobDetail};
use crate::models::JobSnapshot;
use crate::page_window;
use rusqlite::{params, Row};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, State};

const MAX_FILE_BYTES: u64 = 20 * 1024 * 1024;
const MAX_TEXT_CHARS: usize = 500_000;

/// 抓取结果，尚未入库。
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDraft {
    /// 待保存的原始页面文件；粘贴文本时为空。
    pub token: Option<String>,
    pub source_url: Option<String>,
    pub source_kind: String,
    pub text: String,
    /// 不靠 AI 从页面里识别出的岗位字段（有的话）。
    pub detail: Option<JobDetail>,
    pub detail_source: Option<DetailSource>,
}

fn snapshots_dir() -> PathBuf {
    crate::db::default_db_dir().join("snapshots")
}

fn pending_dir() -> PathBuf {
    snapshots_dir().join("pending")
}

/// 启动时清理上次没保存的抓取结果。
pub fn clear_pending() {
    let _ = std::fs::remove_dir_all(pending_dir());
}

fn new_token() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_millis())
        .unwrap_or_default();
    format!("{millis}-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

/// token 会拼进文件路径，只接受我们自己生成的格式，杜绝 `..` 之类的路径穿越。
fn pending_path(token: &str) -> Result<PathBuf, String> {
    let valid = !token.is_empty()
        && token.len() <= 64
        && token.chars().all(|c| c.is_ascii_digit() || c == '-');
    if !valid {
        return Err("快照标识无效".to_string());
    }
    Ok(pending_dir().join(format!("{token}.html")))
}

/// 来源地址只是元数据，但会在界面上被点开，所以只保留 http(s) 地址。
fn clean_source_url(url: Option<String>) -> Option<String> {
    url.map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty() && url_host(u).is_ok())
}

/// 可见文字少于这个数，基本就是等脚本填内容的页面框架（Moka 的框架页只有二十几个字）。
const MIN_PAGE_CHARS: usize = 150;

const NEEDS_BROWSER: &str = "抓到的只是空白的页面框架：这个站点的岗位内容由脚本加载，HTTP 抓取拿不到。\
     请在「系统设置」里把抓取方式改为「无头浏览器」后重新抓取，或在浏览器里「另存为」网页后导入文件。";

const THIN_PAGE: &str = "页面里几乎没有正文，可能需要登录才能看到。\
     可以在浏览器里打开岗位页后「另存为」网页再导入，或直接粘贴 JD 文本。";

/// `#/job/xxx` 这类前端路由：`#` 后面的部分根本不会发给服务器，HTTP 抓取只能拿到门户首页。
fn hash_routed(url: &str) -> bool {
    url.split_once('#')
        .is_some_and(|(_, frag)| frag.starts_with('/') || frag.starts_with("!/"))
}

/// 页面里有没有值得存档的岗位内容；没有时给出原因和办法。
/// `rendered`：页面是否执行过脚本（无头浏览器抓取、或用户从浏览器另存的文件）。
fn check_content(page_text: &str, source_url: Option<&str>, rendered: bool) -> Result<(), String> {
    let thin = page_text.chars().filter(|c| !c.is_whitespace()).count() < MIN_PAGE_CHARS;
    if !rendered && (thin || source_url.is_some_and(hash_routed)) {
        return Err(NEEDS_BROWSER.to_string());
    }
    if thin {
        return Err(THIN_PAGE.to_string());
    }
    Ok(())
}

fn make_draft(
    source_url: Option<String>,
    source_kind: &str,
    html: &str,
    rendered: bool,
) -> Result<SnapshotDraft, String> {
    let page_text = html_to_text(html);
    let found = detect_job(html, source_url.as_deref());
    // 有 JobPosting 数据就说明内容是真的，不管页面其余部分多空。
    if !matches!(found, Some((_, DetailSource::Structured))) {
        check_content(&page_text, source_url.as_deref(), rendered)?;
    }
    let (mut detail, detail_source) = found.unzip();
    // 识别到的职位描述就是 JD 本身，比整页文字（导航、页脚、推荐岗位）干净得多。
    let text = detail
        .as_mut()
        .and_then(|d| d.jd_text.take())
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(page_text);
    let token = new_token();
    let path = pending_path(&token)?;
    std::fs::create_dir_all(pending_dir()).map_err(|e| e.to_string())?;
    std::fs::write(&path, html.as_bytes()).map_err(|e| e.to_string())?;
    Ok(SnapshotDraft {
        token: Some(token),
        source_url: clean_source_url(source_url),
        source_kind: source_kind.to_string(),
        text,
        detail,
        detail_source,
    })
}

// ---------- 保存下来的网页文件 ----------

/// Chrome / Edge「另存为」会在文件头写 `<!-- saved from url=(0048)https://... -->`，
/// 括号里是地址的字符数。按长度截取，不能按 `-` 截——网址里本来就常有连字符。
fn saved_from_url(html: &str) -> Option<String> {
    const MARKER: &str = "saved from url=(";
    let idx = html.find(MARKER)?;
    if idx > 4096 {
        return None;
    }
    let rest = &html[idx + MARKER.len()..];
    let close = rest.find(')')?;
    let after = &rest[close + 1..];
    let by_length = rest[..close]
        .parse::<usize>()
        .ok()
        .filter(|&n| n > 0 && after.chars().count() >= n)
        .map(|n| after.chars().take(n).collect::<String>());
    let url =
        by_length.unwrap_or_else(|| after.chars().take_while(|c| !c.is_whitespace()).collect());
    (!url.is_empty()).then_some(url)
}

fn page_url(html: &str) -> Option<String> {
    saved_from_url(html).or_else(|| {
        let page = Page::new(html);
        page.link("canonical").or_else(|| page.meta("og:url"))
    })
}

/// 解析 MIME 头部块（支持续行折叠）。
fn parse_headers(block: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in block.lines() {
        if line.starts_with([' ', '\t']) {
            if let Some(last) = out.last_mut() {
                last.1.push(' ');
                last.1.push_str(line.trim());
            }
        } else if let Some((k, v)) = line.split_once(':') {
            out.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }
    out
}

fn header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v.as_str())
}

fn header_param(value: &str, name: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let key = format!("{name}=");
    let pos = lower.find(&key)? + key.len();
    let rest = value[pos..].trim_start();
    let v = if let Some(stripped) = rest.strip_prefix('"') {
        stripped.split('"').next()?
    } else {
        rest.split([';', ' ']).next()?
    };
    Some(v.to_string())
}

fn split_head_body(part: &str) -> Option<(&str, &str)> {
    if let Some(i) = part.find("\r\n\r\n") {
        return Some((&part[..i], &part[i + 4..]));
    }
    part.find("\n\n").map(|i| (&part[..i], &part[i + 2..]))
}

fn hex_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn decode_quoted_printable(input: &str) -> Vec<u8> {
    let b = input.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'=' {
            // 软换行
            if b.get(i + 1) == Some(&b'\r') && b.get(i + 2) == Some(&b'\n') {
                i += 3;
                continue;
            }
            if b.get(i + 1) == Some(&b'\n') {
                i += 2;
                continue;
            }
            if let (Some(h), Some(l)) = (
                b.get(i + 1).copied().and_then(hex_value),
                b.get(i + 2).copied().and_then(hex_value),
            ) {
                out.push((h << 4) | l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// 从 MHTML（「网页，单个文件」）里取出主页面 HTML 和原始地址。
fn parse_mhtml(bytes: &[u8]) -> Result<(Option<String>, String), String> {
    const INVALID: &str = "不是有效的 MHTML 文件";
    // MIME 头部、QP 与 base64 编码的正文都是 ASCII，按 UTF-8 宽松读取不会丢信息。
    let raw = String::from_utf8_lossy(bytes);
    let (head, body) = split_head_body(&raw).ok_or(INVALID)?;
    let headers = parse_headers(head);
    let url = header(&headers, "snapshot-content-location").map(str::to_string);
    let boundary = header(&headers, "content-type")
        .and_then(|ct| header_param(ct, "boundary"))
        .ok_or(INVALID)?;
    for part in body.split(&format!("--{boundary}")) {
        let part = part.trim_start_matches(['\r', '\n']);
        if part.starts_with("--") {
            break;
        }
        let Some((part_head, part_body)) = split_head_body(part) else {
            continue;
        };
        let part_headers = parse_headers(part_head);
        let content_type = header(&part_headers, "content-type").unwrap_or("");
        if !content_type.to_ascii_lowercase().starts_with("text/html") {
            continue;
        }
        let encoding = header(&part_headers, "content-transfer-encoding")
            .unwrap_or("")
            .to_ascii_lowercase();
        let decoded = match encoding.trim() {
            "quoted-printable" => decode_quoted_printable(part_body),
            "base64" => {
                use base64::Engine as _;
                let compact: String = part_body.split_whitespace().collect();
                base64::engine::general_purpose::STANDARD
                    .decode(compact)
                    .map_err(|e| format!("MHTML 正文解码失败：{e}"))?
            }
            _ => part_body.as_bytes().to_vec(),
        };
        let html = decode_body(&decoded, Some(content_type));
        let url = url.or_else(|| header(&part_headers, "content-location").map(str::to_string));
        return Ok((url, html));
    }
    Err("MHTML 文件里没有找到网页正文".to_string())
}

fn read_page_file(path: &Path) -> Result<(Option<String>, String), String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("只能导入普通文件".to_string());
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err("网页文件超过 20 MB 限制".to_string());
    }
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    match ext.as_str() {
        "mhtml" | "mht" => parse_mhtml(&bytes),
        "html" | "htm" => {
            let html = decode_body(&bytes, None);
            Ok((page_url(&html), html))
        }
        _ => Err(format!("不支持的文件类型：.{ext}，请选择 .html 或 .mhtml")),
    }
}

// ---------- 命令 ----------

/// 先按设置的抓取方式直接取页面（普通网页最快）；拿不到岗位内容——脚本渲染的空壳、
/// 前端路由、被防火墙拦下——就改用应用自带的浏览器内核在后台窗口里打开。
#[tauri::command(async)]
pub fn capture_snapshot_from_url(app: AppHandle, url: String) -> Result<SnapshotDraft, String> {
    validate_job_url(&url)?;
    if !hash_routed(&url) {
        let direct = fetch_page_html(&url)
            .and_then(|page| make_draft(Some(page.url), "url", &page.html, page.rendered));
        if let Ok(draft) = direct {
            return Ok(draft);
        }
    }
    let page = page_window::load_page(&app, &url)?;
    make_draft(Some(page.url), "url", &page.html, true)
}

#[tauri::command(async)]
pub fn capture_snapshot_from_file(path: String) -> Result<SnapshotDraft, String> {
    let (url, html) = read_page_file(Path::new(&path))?;
    // 从浏览器另存的文件是渲染后的页面。
    make_draft(url, "file", &html, true)
}

#[tauri::command]
pub fn discard_snapshot_draft(token: String) -> Result<(), String> {
    let path = pending_path(&token)?;
    let _ = std::fs::remove_file(path);
    Ok(())
}

fn row_snapshot(r: &Row) -> rusqlite::Result<JobSnapshot> {
    Ok(JobSnapshot {
        id: r.get(0)?,
        job_id: r.get(1)?,
        source_url: r.get(2)?,
        source_kind: r.get(3)?,
        text: r.get(4)?,
        html_path: r.get(5)?,
        captured_at: r.get(6)?,
    })
}

const SNAPSHOT_COLUMNS: &str = "id, job_id, source_url, source_kind, text, html_path, captured_at";

#[tauri::command(async)]
pub fn save_job_snapshot(
    state: State<Db>,
    job_id: i64,
    source_url: Option<String>,
    source_kind: String,
    text: String,
    token: Option<String>,
) -> Result<JobSnapshot, String> {
    if !["url", "file", "paste"].contains(&source_kind.as_str()) {
        return Err("快照来源无效".to_string());
    }
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("JD 内容不能为空".to_string());
    }
    if text.chars().count() > MAX_TEXT_CHARS {
        return Err("JD 内容过长".to_string());
    }
    let source_url = match source_url.map(|u| u.trim().to_string()) {
        Some(u) if u.is_empty() => None,
        Some(u) if url_host(&u).is_err() => {
            return Err("来源地址必须是 http:// 或 https:// 开头的网址".to_string())
        }
        other => other,
    };
    let pending = token.as_deref().map(pending_path).transpose()?;
    if pending.as_ref().is_some_and(|p| !p.is_file()) {
        return Err("抓取结果已过期，请重新抓取".to_string());
    }

    let mut conn = state.0.lock();
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO job_snapshots (job_id, source_url, source_kind, text) VALUES (?1, ?2, ?3, ?4)",
        params![job_id, source_url, source_kind, text],
    )
    .map_err(|e| {
        if e.to_string().contains("FOREIGN KEY") {
            "岗位不存在".to_string()
        } else {
            e.to_string()
        }
    })?;
    let id = tx.last_insert_rowid();
    let mut moved: Option<PathBuf> = None;
    if let Some(pending) = pending {
        let dir = snapshots_dir().join(job_id.to_string());
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let dest = dir.join(format!("{id}.html"));
        std::fs::rename(&pending, &dest).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE job_snapshots SET html_path = ?1 WHERE id = ?2",
            params![dest.to_string_lossy().to_string(), id],
        )
        .map_err(|e| e.to_string())?;
        moved = Some(dest);
    }
    let result = tx
        .query_row(
            &format!("SELECT {SNAPSHOT_COLUMNS} FROM job_snapshots WHERE id = ?1"),
            params![id],
            row_snapshot,
        )
        .map_err(|e| e.to_string())?;
    if let Err(e) = tx.commit() {
        if let Some(dest) = moved {
            let _ = std::fs::remove_file(dest);
        }
        return Err(e.to_string());
    }
    Ok(result)
}

#[tauri::command]
pub fn list_job_snapshots(state: State<Db>, job_id: i64) -> Result<Vec<JobSnapshot>, String> {
    let conn = state.0.lock();
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {SNAPSHOT_COLUMNS} FROM job_snapshots WHERE job_id = ?1 ORDER BY id DESC"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![job_id], row_snapshot)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_job_snapshot(state: State<Db>, id: i64) -> Result<(), String> {
    let conn = state.0.lock();
    let html_path: Option<String> = conn
        .query_row(
            "SELECT html_path FROM job_snapshots WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    conn.execute("DELETE FROM job_snapshots WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    drop(conn);
    if let Some(path) = html_path {
        crate::commands::remove_managed_files(vec![path]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_page_url_from_saved_html() {
        let url = "https://jobs-cn.example.com/p/job-1";
        let saved = format!(
            "<!DOCTYPE html>\n<!-- saved from url=({:04}){url} -->\n<html></html>",
            url.len()
        );
        assert_eq!(page_url(&saved).as_deref(), Some(url));
        let canonical = r#"<html><head><link rel="canonical" href="https://a.example/j?x=1&amp;y=2"></head></html>"#;
        assert_eq!(
            page_url(canonical).as_deref(),
            Some("https://a.example/j?x=1&y=2")
        );
        assert!(page_url("<html></html>").is_none());
    }

    #[test]
    fn extracts_html_from_chrome_mhtml() {
        let mhtml = "From: <Saved by Blink>\r\n\
Snapshot-Content-Location: https://jobs.example.com/p/42\r\n\
Subject: Job\r\n\
MIME-Version: 1.0\r\n\
Content-Type: multipart/related;\r\n\
\ttype=\"text/html\";\r\n\
\tboundary=\"----MultipartBoundary--abc----\"\r\n\
\r\n\
\r\n\
------MultipartBoundary--abc----\r\n\
Content-Type: text/html\r\n\
Content-ID: <frame-1@mhtml.blink>\r\n\
Content-Transfer-Encoding: quoted-printable\r\n\
Content-Location: https://jobs.example.com/p/42\r\n\
\r\n\
<html><head><meta charset=3D\"utf-8\"></head><body><p>=E5=B2=97=E4=BD=8D=\r\n\
=E8=81=8C=E8=B4=A3</p></body></html>\r\n\
------MultipartBoundary--abc----\r\n\
Content-Type: image/png\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
iVBORw0KGgo=\r\n\
------MultipartBoundary--abc------\r\n";
        let (url, html) = parse_mhtml(mhtml.as_bytes()).unwrap();
        assert_eq!(url.as_deref(), Some("https://jobs.example.com/p/42"));
        assert_eq!(html_to_text(&html), "岗位职责");
        assert!(parse_mhtml(b"not a mhtml file").is_err());
    }

    #[test]
    fn rejects_path_like_tokens() {
        assert!(pending_path("123-4").is_ok());
        assert!(pending_path("../../evil").is_err());
        assert!(pending_path("").is_err());
        assert!(pending_path("abc").is_err());
    }

    #[test]
    fn keeps_only_web_source_urls() {
        assert_eq!(
            clean_source_url(Some(" https://a.example/j ".into())).as_deref(),
            Some("https://a.example/j")
        );
        assert!(clean_source_url(Some("file:///C:/x.html".into())).is_none());
        assert!(clean_source_url(Some("javascript:alert(1)".into())).is_none());
    }

    #[test]
    fn rejects_unrendered_page_shells() {
        let shell = "<html><head><title>示例科技 - 校园招聘</title></head>\
                     <body><div id=\"app\"></div><script>boot()</script></body></html>";
        let url = || Some("https://jobs.example.com/portal".to_string());
        let err = |r: Result<SnapshotDraft, String>| r.err().unwrap_or_default();
        assert_eq!(err(make_draft(url(), "url", shell, false)), NEEDS_BROWSER);
        // 渲染过还是这么空，就不是抓取方式的问题了
        assert_eq!(err(make_draft(url(), "url", shell, true)), THIN_PAGE);

        // 内容再多，HTTP 也拿不到 # 后面路由指向的那个岗位
        let portal = format!("<body><p>{}</p></body>", "热招职位 ".repeat(100));
        let routed = Some("https://jobs.example.com/portal#/job/abc".to_string());
        assert_eq!(
            err(make_draft(routed, "url", &portal, false)),
            NEEDS_BROWSER
        );
        assert!(hash_routed("https://a.example/x#!/job/1"));
        assert!(!hash_routed("https://a.example/x#section-2"));
    }
}
