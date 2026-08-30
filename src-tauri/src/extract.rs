use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub fetch_mode: String, // "http" | "browser"
    pub cookie: String,
    pub browser_path: String,
    pub provider: String, // "none" | "ollama" | "openai"
    pub ollama_url: String,
    pub ollama_model: String,
    pub openai_url: String,
    pub openai_key: String,
    pub openai_model: String,
    pub proxy: String,
    pub mirror: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            fetch_mode: "http".to_string(),
            cookie: String::new(),
            browser_path: String::new(),
            provider: "none".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "qwen2.5:7b".to_string(),
            openai_url: "https://api.openai.com/v1".to_string(),
            openai_key: String::new(),
            openai_model: "gpt-4o-mini".to_string(),
            proxy: String::new(),
            mirror: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtractedJob {
    pub title: Option<String>,
    pub company: Option<String>,
    pub salary_min: Option<i64>,
    pub salary_max: Option<i64>,
    pub location: Option<String>,
    pub department: Option<String>,
    pub category: Option<String>,
    pub summary: Option<String>,
}

pub fn settings_path() -> PathBuf {
    crate::db::default_db_dir().join("settings.json")
}

fn load_settings() -> Settings {
    let p = settings_path();
    if let Ok(s) = std::fs::read_to_string(&p) {
        if let Ok(v) = serde_json::from_str::<Settings>(&s) {
            return v;
        }
    }
    Settings::default()
}

fn save_settings_file(s: &Settings) -> Result<(), String> {
    let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
    let p = settings_path();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, json).map_err(|e| e.to_string())
}

// ---------- HTML -> text ----------
fn clean_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_space = false;
    let mut prev_newline = 0u8;
    for ch in s.chars() {
        if ch == '\n' || ch == '\r' {
            if !in_space && prev_newline < 2 {
                out.push('\n');
                prev_newline += 1;
            }
            in_space = true;
        } else if ch == ' ' || ch == '\t' {
            in_space = true;
        } else {
            if in_space && prev_newline == 0 {
                out.push(' ');
            }
            out.push(ch);
            in_space = false;
            prev_newline = 0;
        }
    }
    out.trim().to_string()
}

fn decode_entities(s: &str) -> String {
    s.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
}

pub fn html_to_text(html: &str) -> String {
    let lower = html.to_ascii_lowercase();
    let lb = lower.as_bytes();
    let b = html.as_bytes();
    let mut out = String::with_capacity(html.len());
    let mut i = 0usize;
    let mut skip_until: Option<&'static str> = None;

    while i < b.len() {
        if let Some(close) = skip_until {
            if let Some(pos) = lower[i..].find(close) {
                i += pos + close.len();
                while i < b.len() && b[i] != b'>' {
                    i += 1;
                }
                if i < b.len() {
                    i += 1;
                }
                skip_until = None;
                continue;
            } else {
                break;
            }
        }

        if lb[i] == b'<' {
            let after = &lower[i..];
            let mut block_close: Option<&'static str> = None;
            if after.starts_with("<script") {
                block_close = Some("</script");
            } else if after.starts_with("<style") {
                block_close = Some("</style");
            } else if after.starts_with("<noscript") {
                block_close = Some("</noscript");
            }
            if let Some(close) = block_close {
                skip_until = Some(close);
                continue;
            }
            if after.starts_with("<br") {
                out.push('\n');
            } else {
                let mut is_block_close = false;
                for t in [
                    "</p", "</div", "</li", "</tr", "</h1", "</h2", "</h3", "</h4", "</h5", "</h6",
                    "</section", "</article", "</ul", "</ol", "</table", "</pre",
                ] {
                    if after.starts_with(t) {
                        is_block_close = true;
                        break;
                    }
                }
                if is_block_close {
                    out.push('\n');
                }
            }
            i += 1;
            while i < b.len() && b[i] != b'>' {
                i += 1;
            }
            if i < b.len() {
                i += 1;
            }
            continue;
        } else {
            let ch = html[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
    }

    clean_text(&decode_entities(&out))
}

// ---------- Browser discovery ----------
fn find_browser(custom: &str) -> Option<PathBuf> {
    if !custom.is_empty() && Path::new(custom).exists() {
        return Some(PathBuf::from(custom));
    }
    let fixed = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];
    for c in fixed {
        if Path::new(c).exists() {
            return Some(PathBuf::from(c));
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        for c in [
            r"Microsoft\Edge\Application\msedge.exe",
            r"Google\Chrome\Application\chrome.exe",
        ] {
            let p = Path::new(&local).join(c);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

fn fetch_via_browser(browser: &PathBuf, url: &str) -> Result<String, String> {
    let tmp = std::env::temp_dir().join(format!("offerflow_browser_{}", std::process::id()));
    std::fs::create_dir_all(&tmp).ok();
    let res = std::process::Command::new(browser)
        .arg("--headless=new")
        .arg("--disable-gpu")
        .arg("--no-first-run")
        .arg("--no-default-browser-check")
        .arg(format!("--user-data-dir={}", tmp.display()))
        .arg("--virtual-time-budget=8000")
        .arg("--dump-dom")
        .arg(url)
        .output()
        .map_err(|e| e.to_string());
    let _ = std::fs::remove_dir_all(&tmp);
    let out = res?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    Ok(html_to_text(&String::from_utf8_lossy(&out.stdout)))
}

fn build_agent(proxy_url: &str) -> Result<ureq::Agent, String> {
    let mut builder = ureq::AgentBuilder::new();
    if !proxy_url.is_empty() {
        let proxy = ureq::Proxy::new(proxy_url).map_err(|e| e.to_string())?;
        builder = builder.proxy(proxy);
    }
    Ok(builder.build())
}

fn fetch_via_http(url: &str, s: &Settings) -> Result<String, String> {
    let agent = build_agent(&s.proxy)?;
    let mut req = agent
        .get(url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36",
        )
        .set(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        );
    if !s.cookie.is_empty() {
        req = req.set("Cookie", &s.cookie);
    }
    let body = match req.call() {
        Ok(resp) => resp.into_string().map_err(|e| e.to_string())?,
        Err(ureq::Error::Status(code, resp)) => {
            let b = resp.into_string().unwrap_or_default();
            let snippet = &b[..b.len().min(300)];
            return Err(format!("HTTP {code}: {snippet}"));
        }
        Err(e) => return Err(e.to_string()),
    };
    Ok(html_to_text(&body))
}

// ---------- Model calls ----------
fn ollama_generate(s: &Settings, prompt: &str) -> Result<String, String> {
    let url = format!("{}/api/generate", s.ollama_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": s.ollama_model,
        "prompt": prompt,
        "stream": false
    })
    .to_string();
    let resp = ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| e.to_string())?;
    let text = resp.into_string().map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(v.get("response")
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .to_string())
}

fn openai_chat(s: &Settings, prompt: &str) -> Result<String, String> {
    let url = format!("{}/chat/completions", s.openai_url.trim_end_matches('/'));
    let body = serde_json::json!({
        "model": s.openai_model,
        "messages": [
            {"role": "system", "content": "你是招聘信息提取助手，只输出 JSON。"},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0
    })
    .to_string();
    let agent = build_agent(&s.proxy)?;
    let resp = agent
        .post(&url)
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Bearer {}", s.openai_key))
        .send_string(&body)
        .map_err(|e| e.to_string())?;
    let text = resp.into_string().map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(v.pointer("/choices/0/message/content")
        .and_then(|c| c.as_str())
        .unwrap_or("")
        .to_string())
}

fn extract_jd_impl(s: &Settings, content: &str) -> Result<ExtractedJob, String> {
    let prompt = format!(
        "你是招聘信息提取助手。从下面的职位描述中提取信息，只返回一个 JSON 对象，字段如下：\
         {{ \"title\": \"岗位名称\", \"company\": \"公司名\", \"salaryMin\": 月薪下限数字(单位k，如12)，\"salaryMax\": 月薪上限数字(k)，\"location\": \"工作城市\", \"department\": \"部门\", \"category\": \"岗位分类(从 技术类/产品类/设计类/运营类/市场类/职能类/销售类/数据类/其他 中选择最接近的一项)\", \"summary\": \"岗位职责与要求的简短总结\" }}。\
         如果某个字段不存在则用 null。不要输出 JSON 以外的任何内容。\n\n职位描述：\n{content}"
    );
    let raw = match s.provider.as_str() {
        "ollama" => ollama_generate(s, &prompt)?,
        "openai" => openai_chat(s, &prompt)?,
        _ => return Err("请先在「设置」中选择提取模型提供商".to_string()),
    };
    let raw = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```JSON")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let start = raw.find('{');
    let end = raw.rfind('}');
    let json_part = match (start, end) {
        (Some(a), Some(b)) if b > a => &raw[a..=b],
        _ => raw,
    };
    serde_json::from_str::<ExtractedJob>(json_part)
        .map_err(|e| format!("解析失败：{e}。原始输出：{raw}"))
}

fn list_ollama_models_impl(s: &Settings) -> Result<Vec<String>, String> {
    let url = format!("{}/api/tags", s.ollama_url.trim_end_matches('/'));
    let resp = ureq::get(&url).call().map_err(|e| e.to_string())?;
    let text = resp.into_string().map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    if let Some(models) = v.get("models").and_then(|m| m.as_array()) {
        for m in models {
            if let Some(name) = m.get("name").and_then(|n| n.as_str()) {
                out.push(name.to_string());
            }
        }
    }
    Ok(out)
}

// ---------- Tauri commands ----------
#[tauri::command]
pub fn get_settings() -> Settings {
    load_settings()
}

#[tauri::command]
pub fn set_settings(settings: Settings) -> Result<(), String> {
    save_settings_file(&settings)
}

#[tauri::command]
pub fn fetch_jd_text(url: String) -> Result<String, String> {
    let s = load_settings();
    if s.fetch_mode == "browser" {
        if let Some(browser) = find_browser(&s.browser_path) {
            if let Ok(text) = fetch_via_browser(&browser, &url) {
                return Ok(text);
            }
        }
    }
    fetch_via_http(&url, &s)
}

#[tauri::command]
pub fn extract_jd_info(content: String) -> Result<ExtractedJob, String> {
    let s = load_settings();
    extract_jd_impl(&s, &content)
}

#[tauri::command]
pub fn list_ollama_models() -> Result<Vec<String>, String> {
    let s = load_settings();
    list_ollama_models_impl(&s)
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDetail {
    pub title: String,
    pub company: String,
    pub jd_text: Option<String>,
    pub department: Option<String>,
    pub location: Option<String>,
    pub category: Option<String>,
    pub salary_min: Option<i64>,
    pub salary_max: Option<i64>,
}

const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0 Safari/537.36";

fn fixed_16(s: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    let b = s.as_bytes();
    let n = b.len().min(16);
    out[..n].copy_from_slice(&b[..n]);
    out
}

fn decrypt_mokahr(data_b64: &str, key: &str, iv: &str) -> Result<String, String> {
    use aes::cipher::{block_padding::Pkcs7, generic_array::GenericArray, BlockDecryptMut, KeyIvInit};
    use aes::Aes128;
    use base64::Engine as _;
    use cbc::Decryptor;

    let mut buf = base64::engine::general_purpose::STANDARD
        .decode(data_b64)
        .map_err(|e| format!("base64 解码失败：{e}"))?;
    let key_arr = *GenericArray::from_slice(&fixed_16(key));
    let iv_arr = *GenericArray::from_slice(&fixed_16(iv));
    let cipher = Decryptor::<Aes128>::new(&key_arr, &iv_arr);
    let pt = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|e| format!("AES 解密失败：{e}"))?;
    String::from_utf8(pt.to_vec()).map_err(|e| format!("解密结果非 UTF-8：{e}"))
}

fn origin_of(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    let host = rest.split('/').next()?;
    Some(format!("{scheme}://{host}"))
}

fn parse_mokahr(url: &str) -> Result<(String, String), String> {
    let trimmed = url.trim();
    let (page, frag) = match trimmed.split_once('#') {
        Some((p, f)) => (p.to_string(), f),
        None => (trimmed.to_string(), ""),
    };
    if page.is_empty() {
        return Err("URL 无效".to_string());
    }
    let job_id = frag
        .trim_end_matches('/')
        .rsplit('/')
        .find(|s| !s.is_empty())
        .ok_or("URL 中未找到岗位 ID（#/job/... 部分）")?
        .to_string();
    Ok((page, job_id))
}

fn extract_init_value(html: &str) -> Option<String> {
    let idx = html.find("init-data")?;
    let rest = &html[idx..].trim_start_matches("init-data");
    let vpos = rest.find("value=")?;
    let after = rest[vpos + "value=".len()..].trim_start();
    let mut chars = after.chars();
    let quote = chars.next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let body = &after[quote.len_utf8()..];
    let end = body.find(quote)?;
    Some(body[..end].to_string())
}

fn get_raw(agent: &ureq::Agent, url: &str, s: &Settings) -> Result<String, String> {
    let mut req = agent
        .get(url)
        .set("User-Agent", USER_AGENT)
        .set(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        );
    if !s.cookie.is_empty() {
        req = req.set("Cookie", &s.cookie);
    }
    match req.call() {
        Ok(resp) => resp.into_string().map_err(|e| e.to_string()),
        Err(ureq::Error::Status(code, resp)) => {
            let b = resp.into_string().unwrap_or_default();
            let snippet = &b[..b.len().min(300)];
            Err(format!("HTTP {code}：{snippet}"))
        }
        Err(e) => Err(e.to_string()),
    }
}

fn post_json(agent: &ureq::Agent, url: &str, referer: &str, body: &str) -> Result<String, String> {
    let req = agent
        .post(url)
        .set("Content-Type", "application/json")
        .set("User-Agent", USER_AGENT)
        .set("Referer", referer);
    match req.send_string(body) {
        Ok(resp) => resp.into_string().map_err(|e| e.to_string()),
        Err(ureq::Error::Status(code, resp)) => {
            let b = resp.into_string().unwrap_or_default();
            let snippet = &b[..b.len().min(300)];
            Err(format!("HTTP {code}：{snippet}"))
        }
        Err(e) => Err(e.to_string()),
    }
}

fn json_str<'a>(v: &'a serde_json::Value, keys: &[&str]) -> Option<String> {
    for k in keys {
        if let Some(s) = v.get(*k).and_then(|x| x.as_str()) {
            if !s.trim().is_empty() {
                return Some(s.to_string());
            }
        }
    }
    None
}

fn json_int(v: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    for k in keys {
        if let Some(n) = v.get(*k).and_then(|x| x.as_i64()) {
            return Some(n);
        }
    }
    None
}

fn fetch_mokahr_impl(url: &str, s: &Settings) -> Result<JobDetail, String> {
    let (page_url, job_id) = parse_mokahr(url)?;
    let origin = origin_of(&page_url).ok_or("无法解析 URL 域名")?;
    let agent = build_agent(&s.proxy)?;

    let html = get_raw(&agent, &page_url, s)?;
    let init_val = extract_init_value(&html).ok_or("页面中未找到 init-data（站点可能已改版）")?;
    let init: serde_json::Value = serde_json::from_str(&decode_entities(&init_val))
        .map_err(|e| format!("init-data 解析失败：{e}"))?;

    let iv = init
        .get("aesIv")
        .and_then(|v| v.as_str())
        .ok_or("init-data 缺少 aesIv")?
        .to_string();
    let org_id = init
        .pointer("/org/id")
        .and_then(|v| v.as_str())
        .ok_or("init-data 缺少 org.id")?
        .to_string();
    let site_id = init.get("siteId").cloned().ok_or("init-data 缺少 siteId")?;
    let company = init
        .pointer("/org/name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let body = serde_json::json!({
        "jobId": job_id,
        "orgId": org_id,
        "siteId": site_id,
        "locale": "zh-CN",
    })
    .to_string();

    let api = format!("{origin}/api/outer/ats-apply/website/job");
    let text = post_json(&agent, &api, &page_url, &body)?;
    let env: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("岗位接口响应解析失败：{e}"))?;
    let data_b64 = env.get("data").and_then(|v| v.as_str()).ok_or("接口响应缺少 data")?;
    let key = env
        .get("necromancer")
        .and_then(|v| v.as_str())
        .ok_or("接口响应缺少 necromancer 密钥")?;

    let plain = decrypt_mokahr(data_b64, key, &iv)?;
    let dec: serde_json::Value =
        serde_json::from_str(&plain).map_err(|e| format!("解密结果解析失败：{e}"))?;
    let data = dec.get("data").cloned().unwrap_or(serde_json::Value::Null);

    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or("岗位详情缺少标题")?
        .to_string();
    let jd_html = data.get("jobDescription").and_then(|v| v.as_str()).unwrap_or("");
    let jd_text = if jd_html.trim().is_empty() {
        None
    } else {
        Some(html_to_text(jd_html))
    };

    Ok(JobDetail {
        title,
        company,
        jd_text,
        department: json_str(&data, &["department", "dept", "departmentName"]),
        location: json_str(&data, &["cityName", "workAddress", "location", "city", "address"]),
        category: json_str(&data, &["category", "jobCategory", "categoryName"]),
        salary_min: json_int(&data, &["salaryMin", "salary_min", "minSalary"]),
        salary_max: json_int(&data, &["salaryMax", "salary_max", "maxSalary"]),
    })
}

#[tauri::command]
pub fn fetch_mokahr_job_detail(url: String) -> Result<JobDetail, String> {
    let s = load_settings();
    fetch_mokahr_impl(&url, &s)
}

// ---------- Lever / Greenhouse：JSON-LD 结构化数据解析 ----------
fn extract_json_ld_job(html: &str) -> Option<JobDetail> {
    let lower = html.to_ascii_lowercase();
    let mut offset = 0usize;

    while let Some(pos) = lower[offset..].find("<script") {
        let start = offset + pos;
        let Some(tag_end_rel) = lower[start..].find('>') else {
            break;
        };
        let tag_end = start + tag_end_rel;
        let tag = &lower[start..tag_end];
        if tag.contains("ld+json") {
            let Some(close_rel) = lower[tag_end..].find("</script") else {
                break;
            };
            let close = tag_end + close_rel;
            let content = html[tag_end + 1..close].trim();
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(content) {
                if let Some(job) = job_from_ld_json(&v) {
                    return Some(job);
                }
            }
        }
        offset = start + 1;
    }
    None
}

fn job_from_ld_json(v: &serde_json::Value) -> Option<JobDetail> {
    let items: Vec<&serde_json::Value> = match v.as_array() {
        Some(arr) => arr.iter().collect(),
        None => vec![v],
    };

    for item in items {
        let typed = item
            .get("@type")
            .and_then(|x| x.as_str())
            .map(|s| s.eq_ignore_ascii_case("jobposting"))
            .unwrap_or(false);
        let has_title_org = item
            .get("title")
            .and_then(|x| x.as_str())
            .map(|t| !t.trim().is_empty())
            .unwrap_or(false)
            && item.get("hiringOrganization").is_some();
        if !typed && !has_title_org {
            continue;
        }

        let Some(title) = item
            .get("title")
            .and_then(|x| x.as_str())
            .filter(|t| !t.trim().is_empty())
        else {
            continue;
        };

        let company = item
            .get("hiringOrganization")
            .and_then(|x| x.get("name"))
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        let jd_html = item.get("description").and_then(|x| x.as_str()).unwrap_or("");
        let jd_text = if jd_html.trim().is_empty() {
            None
        } else {
            Some(html_to_text(jd_html))
        };

        let department = item
            .get("department")
            .and_then(|d| d.get("name"))
            .and_then(|x| x.as_str())
            .or_else(|| {
                item.get("hiringOrganization")
                    .and_then(|h| h.get("department"))
                    .and_then(|d| d.get("name"))
                    .and_then(|x| x.as_str())
            })
            .map(|s| s.to_string());

        let mut location: Option<String> = None;
        if let Some(loc) = item.get("jobLocation") {
            if let Some(name) = loc.get("name").and_then(|x| x.as_str()) {
                location = Some(name.to_string());
            } else if let Some(addr) = loc.get("address") {
                location = ["addressLocality", "addressRegion", "addressCountry"]
                    .iter()
                    .find_map(|k| addr.get(*k).and_then(|x| x.as_str()))
                    .map(|s| s.to_string());
            }
        }

        return Some(JobDetail {
            title: title.to_string(),
            company,
            jd_text,
            department,
            location,
            category: None,
            salary_min: None,
            salary_max: None,
        });
    }
    None
}

fn fetch_json_ld_impl(url: &str, s: &Settings) -> Result<JobDetail, String> {
    let agent = build_agent(&s.proxy)?;
    let html = get_raw(&agent, url, s)?;
    extract_json_ld_job(&html).ok_or_else(|| "页面中未找到职位结构化数据（JSON-LD）".to_string())
}

#[tauri::command]
pub fn fetch_job_detail(url: String) -> Result<JobDetail, String> {
    let s = load_settings();
    let u = url.trim();
    if u.contains("mokahr.com") {
        return fetch_mokahr_impl(u, &s);
    }
    if u.contains("lever.co") {
        return fetch_json_ld_impl(u, &s);
    }
    if u.contains("greenhouse.io") {
        return fetch_json_ld_impl(u, &s);
    }
    Err("暂不支持该网站，仅支持 MokaHR / Lever / Greenhouse".to_string())
}