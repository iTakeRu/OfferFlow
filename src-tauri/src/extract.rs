use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Settings {
    pub fetch_mode: String, // "http" | "browser"
    pub cookie: String,
    pub cookie_domain: String,
    pub browser_path: String,
    pub provider: String, // "none" | "ollama" | "openai"
    pub ollama_url: String,
    pub ollama_model: String,
    pub openai_url: String,
    pub openai_key: String,
    pub openai_model: String,
    pub proxy: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            fetch_mode: "http".to_string(),
            cookie: String::new(),
            cookie_domain: String::new(),
            browser_path: String::new(),
            provider: "none".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            ollama_model: "qwen2.5:7b".to_string(),
            openai_url: "https://api.openai.com/v1".to_string(),
            openai_key: String::new(),
            openai_model: "gpt-4o-mini".to_string(),
            proxy: String::new(),
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
        if let Ok(mut v) = serde_json::from_str::<Settings>(&s) {
            let secrets_path = settings_path().with_file_name("settings.secrets.json");
            if let Ok(raw) = std::fs::read_to_string(&secrets_path) {
                if let Ok(secret) = serde_json::from_str::<SecretSettings>(&raw) {
                    v.cookie = unprotect_secret(&secret.cookie).unwrap_or_default();
                    v.openai_key = unprotect_secret(&secret.openai_key).unwrap_or_default();
                }
            } else if !v.cookie.is_empty() || !v.openai_key.is_empty() {
                // Migrate legacy plaintext secrets on first read.
                let _ = save_settings_file(&v);
            }
            return v;
        }
    }
    Settings::default()
}

fn save_settings_file(s: &Settings) -> Result<(), String> {
    let mut public = s.clone();
    public.cookie.clear();
    public.openai_key.clear();
    let json = serde_json::to_string_pretty(&public).map_err(|e| e.to_string())?;
    let p = settings_path();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, json).map_err(|e| e.to_string())?;
    let secrets = SecretSettings {
        cookie: protect_secret(&s.cookie)?,
        openai_key: protect_secret(&s.openai_key)?,
    };
    let secrets_path = p.with_file_name("settings.secrets.json");
    std::fs::write(
        &secrets_path,
        serde_json::to_vec_pretty(&secrets).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&secrets_path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SecretSettings {
    cookie: String,
    openai_key: String,
}

#[cfg(windows)]
fn protect_secret(value: &str) -> Result<String, String> {
    dpapi_encode(value.as_bytes())
}
#[cfg(windows)]
fn unprotect_secret(value: &str) -> Result<String, String> {
    let bytes = dpapi_decode(value)?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

#[cfg(windows)]
#[repr(C)]
struct DataBlob {
    cb_data: u32,
    pb_data: *mut u8,
}
#[cfg(windows)]
#[link(name = "Crypt32")]
extern "system" {
    fn CryptProtectData(
        input: *mut DataBlob,
        description: *const u16,
        entropy: *mut DataBlob,
        reserved: *mut std::ffi::c_void,
        prompt: *mut std::ffi::c_void,
        flags: u32,
        output: *mut DataBlob,
    ) -> i32;
    fn CryptUnprotectData(
        input: *mut DataBlob,
        description: *mut *mut u16,
        entropy: *mut DataBlob,
        reserved: *mut std::ffi::c_void,
        prompt: *mut std::ffi::c_void,
        flags: u32,
        output: *mut DataBlob,
    ) -> i32;
}
#[cfg(windows)]
#[link(name = "Kernel32")]
extern "system" {
    fn LocalFree(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

#[cfg(windows)]
fn dpapi_encode(input: &[u8]) -> Result<String, String> {
    use base64::Engine as _;
    let mut owned = input.to_vec();
    let mut source = DataBlob {
        cb_data: owned.len() as u32,
        pb_data: owned.as_mut_ptr(),
    };
    let mut output = DataBlob {
        cb_data: 0,
        pb_data: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptProtectData(
            &mut source,
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            4,
            &mut output,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let result =
        unsafe { std::slice::from_raw_parts(output.pb_data, output.cb_data as usize).to_vec() };
    unsafe {
        LocalFree(output.pb_data.cast());
    }
    Ok(base64::engine::general_purpose::STANDARD.encode(result))
}

#[cfg(windows)]
fn dpapi_decode(value: &str) -> Result<Vec<u8>, String> {
    use base64::Engine as _;
    let mut owned = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|e| e.to_string())?;
    let mut source = DataBlob {
        cb_data: owned.len() as u32,
        pb_data: owned.as_mut_ptr(),
    };
    let mut output = DataBlob {
        cb_data: 0,
        pb_data: std::ptr::null_mut(),
    };
    let ok = unsafe {
        CryptUnprotectData(
            &mut source,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut output,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let result =
        unsafe { std::slice::from_raw_parts(output.pb_data, output.cb_data as usize).to_vec() };
    unsafe {
        LocalFree(output.pb_data.cast());
    }
    Ok(result)
}

#[cfg(not(windows))]
fn protect_secret(value: &str) -> Result<String, String> {
    use base64::Engine as _;
    Ok(base64::engine::general_purpose::STANDARD.encode(value))
}
#[cfg(not(windows))]
fn unprotect_secret(value: &str) -> Result<String, String> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
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

pub(crate) fn decode_entities(s: &str) -> String {
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
                    "</p",
                    "</div",
                    "</li",
                    "</tr",
                    "</h1",
                    "</h2",
                    "</h3",
                    "</h4",
                    "</h5",
                    "</h6",
                    "</section",
                    "</article",
                    "</ul",
                    "</ol",
                    "</table",
                    "</pre",
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
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|v| v.as_nanos())
        .unwrap_or_default();
    let tmp = std::env::temp_dir().join(format!(
        "offerflow_browser_{}_{}",
        std::process::id(),
        nonce
    ));
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
    if out.stdout.len() > 5 * 1024 * 1024 {
        return Err("浏览器抓取结果超过 5 MB 限制".to_string());
    }
    // --dump-dom 输出的是渲染后的 DOM，Chromium 统一按 UTF-8 输出。
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

const MAX_REDIRECTS: u32 = 5;

fn agent_builder() -> ureq::AgentBuilder {
    ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(20))
        .timeout_write(std::time::Duration::from_secs(20))
}

fn with_proxy(builder: ureq::AgentBuilder, proxy_url: &str) -> Result<ureq::AgentBuilder, String> {
    if proxy_url.is_empty() {
        return Ok(builder);
    }
    let proxy = ureq::Proxy::new(proxy_url).map_err(|e| e.to_string())?;
    Ok(builder.proxy(proxy))
}

/// 抓取岗位页面用：关闭自动重定向，改由 `get_page` 逐跳校验目标地址，
/// 否则公网页面可以 302 到本机 / 私网地址绕过 `validate_job_url`。
fn build_fetch_agent(proxy_url: &str) -> Result<ureq::Agent, String> {
    Ok(with_proxy(agent_builder().redirects(0), proxy_url)?.build())
}

/// 调用远程模型接口用：允许正常重定向，走用户配置的代理。
fn build_api_agent(proxy_url: &str) -> Result<ureq::Agent, String> {
    Ok(with_proxy(agent_builder().redirects(MAX_REDIRECTS), proxy_url)?.build())
}

/// 调用本机 Ollama 用：同样带超时，但不能走代理（代理会把 localhost 请求打飞）。
fn build_local_agent() -> ureq::Agent {
    agent_builder().redirects(MAX_REDIRECTS).build()
}

pub(crate) fn url_host(url: &str) -> Result<String, String> {
    let trimmed = url.trim();
    let rest = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .ok_or_else(|| "仅支持 http:// 或 https:// 地址".to_string())?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return Err("URL 不允许包含用户名或密码".to_string());
    }
    let host = if authority.starts_with('[') {
        authority
            .strip_prefix('[')
            .and_then(|s| s.split(']').next())
            .unwrap_or("")
    } else {
        authority.split(':').next().unwrap_or("")
    };
    if host.is_empty() {
        return Err("URL 缺少主机名".to_string());
    }
    Ok(host.trim_end_matches('.').to_ascii_lowercase())
}

fn host_matches(host: &str, allowed: &str) -> bool {
    let allowed = allowed
        .trim()
        .trim_start_matches('.')
        .trim_end_matches('.')
        .to_ascii_lowercase();
    !allowed.is_empty() && (host == allowed || host.ends_with(&format!(".{allowed}")))
}

fn validate_job_url(url: &str) -> Result<String, String> {
    let host = url_host(url)?;
    if host == "localhost" || host.ends_with(".localhost") {
        return Err("不允许抓取本机地址".to_string());
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        let private = match ip {
            std::net::IpAddr::V4(v) => {
                v.is_private() || v.is_loopback() || v.is_link_local() || v.is_unspecified()
            }
            std::net::IpAddr::V6(v) => {
                v.is_loopback()
                    || v.is_unspecified()
                    || v.is_unique_local()
                    || v.is_unicast_link_local()
            }
        };
        if private {
            return Err("不允许抓取本机或私网地址".to_string());
        }
    }
    Ok(host)
}

fn charset_label(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let idx = lower.find("charset=")?;
    let rest = &lower[idx + "charset=".len()..];
    const QUOTES: [char; 2] = ['"', '\''];
    let token = rest
        .trim_start_matches(QUOTES)
        .split([';', ',', ' ', '"', '\'', '/', '>'])
        .find(|part| !part.is_empty())?;
    Some(token.to_string())
}

fn encoding_from_label(label: &str) -> Option<&'static encoding_rs::Encoding> {
    encoding_rs::Encoding::for_label(label.as_bytes())
}

/// 按 Content-Type、`<meta charset>` 解码响应体。中文招聘站点仍有 GBK 页面，
/// 直接要求 UTF-8 会让整次抓取失败。
pub(crate) fn decode_body(bytes: &[u8], content_type: Option<&str>) -> String {
    if let Some(enc) = content_type
        .and_then(charset_label)
        .as_deref()
        .and_then(encoding_from_label)
    {
        return enc.decode(bytes).0.into_owned();
    }
    // 声明的编码优先于「看起来像 UTF-8」：GBK 的双字节序列经常恰好是合法
    // UTF-8（比如「占位」），先试 from_utf8 会解出一串乱码还自认为成功。
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(4096)]);
    if let Some(enc) = charset_label(&head)
        .as_deref()
        .and_then(encoding_from_label)
    {
        return enc.decode(bytes).0.into_owned();
    }
    if let Ok(text) = std::str::from_utf8(bytes) {
        return text.to_string();
    }
    // 没有任何声明又不是 UTF-8：按中文站点最常见的 GBK 兜底解码。
    encoding_rs::GBK.decode(bytes).0.into_owned()
}

fn response_text(resp: ureq::Response) -> Result<String, String> {
    const MAX: u64 = 5 * 1024 * 1024;
    if resp
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok())
        .is_some_and(|n| n > MAX)
    {
        return Err("响应超过 5 MB 限制".to_string());
    }
    let content_type = resp.header("Content-Type").map(str::to_string);
    let mut bytes = Vec::new();
    resp.into_reader()
        .take(MAX + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX {
        return Err("响应超过 5 MB 限制".to_string());
    }
    Ok(decode_body(&bytes, content_type.as_deref()))
}

fn status_error(code: u16, resp: ureq::Response) -> String {
    let body = resp.into_string().unwrap_or_default();
    let snippet: String = body.chars().take(300).collect();
    format!("HTTP {code}：{snippet}")
}

fn join_url(base: &str, location: &str) -> Result<String, String> {
    url::Url::parse(base)
        .map_err(|e| format!("无法解析当前地址：{e}"))?
        .join(location.trim())
        .map(String::from)
        .map_err(|e| format!("无法解析重定向地址：{e}"))
}

/// 抓取页面并逐跳跟随重定向；每一跳都重新过一次地址校验。
/// Cookie 只在目标域名匹配设置时注入，重定向到别的域名后自然不会再带上。
fn get_page(agent: &ureq::Agent, url: &str, s: &Settings) -> Result<(String, String), String> {
    let mut current = url.trim().to_string();
    for _ in 0..=MAX_REDIRECTS {
        let host = validate_job_url(&current)?;
        let mut req = agent.get(&current).set("User-Agent", USER_AGENT).set(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        );
        if !s.cookie.is_empty() && host_matches(&host, &s.cookie_domain) {
            req = req.set("Cookie", &s.cookie);
        }
        match req.call() {
            Ok(resp) if (300..400).contains(&resp.status()) => {
                let location = resp
                    .header("Location")
                    .ok_or("重定向响应缺少 Location 头")?
                    .to_string();
                current = join_url(&current, &location)?;
            }
            Ok(resp) => return Ok((current, response_text(resp)?)),
            Err(ureq::Error::Status(code, resp)) => return Err(status_error(code, resp)),
            Err(e) => return Err(e.to_string()),
        }
    }
    Err(format!("重定向超过 {MAX_REDIRECTS} 次，已停止抓取"))
}

/// 按设置里的抓取方式取回页面原始 HTML，返回 (最终地址, HTML)。
/// 这里只做普通的页面访问：不调用站点私有接口、不做任何解密。
pub(crate) fn fetch_page_html(url: &str) -> Result<(String, String), String> {
    let s = load_settings();
    let url = url.trim();
    validate_job_url(url)?;
    if s.fetch_mode == "browser" {
        // 用户特意选了浏览器模式通常正是因为 HTTP 抓不到，
        // 这里把真实错误抛出去，不要静默回落到 HTTP 让人对着无关报错排查。
        return match find_browser(&s.browser_path) {
            Some(browser) => fetch_via_browser(&browser, url)
                .map(|html| (url.to_string(), html))
                .map_err(|e| format!("无头浏览器抓取失败：{e}")),
            None => {
                Err("未找到 Edge / Chrome，请在设置中指定浏览器路径，或改用 HTTP 抓取".to_string())
            }
        };
    }
    let agent = build_fetch_agent(&s.proxy)?;
    get_page(&agent, url, &s)
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
    let resp = build_local_agent()
        .post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body)
        .map_err(|e| e.to_string())?;
    let text = response_text(resp)?;
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
    let agent = build_api_agent(&s.proxy)?;
    let resp = agent
        .post(&url)
        .set("Content-Type", "application/json")
        .set("Authorization", &format!("Bearer {}", s.openai_key))
        .send_string(&body)
        .map_err(|e| e.to_string())?;
    let text = response_text(resp)?;
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
        _ => return Err("未启用 AI 提取，可在「系统设置」中配置 Ollama 或 API".to_string()),
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
    let resp = build_local_agent().get(&url).call().map_err(|e| match e {
        ureq::Error::Status(code, _) => format!("Ollama 返回错误（HTTP {code}），请检查服务状态"),
        ureq::Error::Transport(_) => format!(
            "无法连接 Ollama（{}）。请确认已启动服务：ollama serve",
            s.ollama_url
        ),
    })?;
    let text = response_text(resp)?;
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
#[tauri::command(async)]
pub fn get_settings() -> Settings {
    load_settings()
}

#[tauri::command(async)]
pub fn set_settings(settings: Settings) -> Result<(), String> {
    if !["none", "ollama", "openai"].contains(&settings.provider.as_str()) {
        return Err("模型提供商无效".to_string());
    }
    if settings.fetch_mode != "http" && settings.fetch_mode != "browser" {
        return Err("抓取方式无效".to_string());
    }
    if settings.fetch_mode == "browser" && !settings.browser_path.trim().is_empty() {
        let path = Path::new(&settings.browser_path);
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if !path.is_file()
            || ![
                "msedge.exe",
                "chrome.exe",
                "chromium.exe",
                "google-chrome",
                "chromium",
            ]
            .contains(&name.as_str())
        {
            return Err("浏览器路径必须指向 Edge、Chrome 或 Chromium 可执行文件".to_string());
        }
    }
    if !settings.cookie_domain.is_empty()
        && (settings
            .cookie_domain
            .chars()
            .any(|c| ['/', ':', '@'].contains(&c))
            || settings.cookie_domain.trim().is_empty())
    {
        return Err("Cookie 域名格式无效，请只填写 example.com".to_string());
    }
    if settings.provider == "openai" {
        let host = url_host(&settings.openai_url)?;
        if settings.openai_url.starts_with("http://")
            && host != "localhost"
            && host != "127.0.0.1"
            && host != "::1"
        {
            return Err("远程 OpenAI 兼容接口必须使用 HTTPS".to_string());
        }
        if settings.openai_key.trim().is_empty() || settings.openai_model.trim().is_empty() {
            return Err("OpenAI API Key 和模型名不能为空".to_string());
        }
    }
    if settings.provider == "ollama" {
        url_host(&settings.ollama_url)?;
        if settings.ollama_model.trim().is_empty() {
            return Err("Ollama 模型名不能为空".to_string());
        }
    }
    if !settings.proxy.trim().is_empty() {
        url_host(&settings.proxy)?;
    }
    save_settings_file(&settings)
}

#[tauri::command(async)]
pub fn extract_jd_info(content: String) -> Result<ExtractedJob, String> {
    if content.len() > 500_000 {
        return Err("JD 内容过长".to_string());
    }
    let s = load_settings();
    extract_jd_impl(&s, &content)
}

#[tauri::command(async)]
pub fn list_ollama_models() -> Result<Vec<String>, String> {
    let s = load_settings();
    list_ollama_models_impl(&s)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaStatus {
    pub installed: bool,
    pub running: bool,
    pub url: String,
}

#[tauri::command(async)]
pub fn check_ollama() -> OllamaStatus {
    let s = load_settings();
    let base = s.ollama_url.trim_end_matches('/');
    // 只是探测服务在不在，连接超时要短：Windows 上连一个没人监听的回环端口，
    // 系统会重发 SYN，约 2 秒后才返回「拒绝连接」。
    let running = agent_builder()
        .timeout_connect(std::time::Duration::from_millis(500))
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .get(&format!("{base}/"))
        .call()
        .map(|r| r.status() == 200)
        .unwrap_or(false);
    OllamaStatus {
        installed: running || ollama_installed(),
        running,
        url: s.ollama_url.clone(),
    }
}

/// 直接扫 PATH 找可执行文件。以前是起 `where` / `which` 子进程：
/// 慢，而且 GUI 程序启动控制台子进程会闪出一个黑窗口。
fn command_in_path(cmd: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    let file = if cfg!(windows) {
        format!("{cmd}.exe")
    } else {
        cmd.to_string()
    };
    std::env::split_paths(&paths).any(|dir| dir.join(&file).is_file())
}

fn ollama_installed() -> bool {
    if command_in_path("ollama") {
        return true;
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    for key in ["LOCALAPPDATA", "PROGRAMFILES"] {
        if let Ok(base) = std::env::var(key) {
            let p = Path::new(&base);
            candidates.push(p.join("Programs").join("Ollama").join("ollama.exe"));
            candidates.push(p.join("Ollama").join("ollama.exe"));
        }
    }
    candidates.iter().any(|p| p.exists())
}
/// 从页面结构化数据里识别出的岗位字段。
#[derive(Serialize, Deserialize, Clone)]
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

// ---------- JSON-LD 结构化数据解析 ----------
/// 很多招聘站点会在页面里以 schema.org `JobPosting` 发布结构化数据（本意就是
/// 给搜索引擎读的公开信息），能拿到就不必动用 AI。与站点无关，任何页面都试一次。
pub(crate) fn extract_json_ld_job(html: &str) -> Option<JobDetail> {
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

        let jd_html = item
            .get("description")
            .and_then(|x| x.as_str())
            .unwrap_or("");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_matches_real_hosts_only() {
        assert_eq!(
            url_host("https://jobs.example.com/a").unwrap(),
            "jobs.example.com"
        );
        assert!(host_matches("jobs.example.com", "example.com"));
        assert!(!host_matches("example.com.evil.test", "example.com"));
        assert!(!host_matches("evil-example.com", "example.com"));
        assert!(!host_matches("jobs.example.com", ""));
    }

    #[test]
    fn rejects_unsafe_job_urls() {
        assert!(validate_job_url("file:///etc/passwd").is_err());
        assert!(validate_job_url("http://localhost:3000").is_err());
        assert!(validate_job_url("http://127.0.0.1/private").is_err());
        assert!(validate_job_url("http://192.168.1.2/private").is_err());
        assert!(validate_job_url("https://user:pass@example.com").is_err());
        assert!(validate_job_url("https://example.com/job").is_ok());
    }

    #[test]
    fn decodes_non_utf8_pages() {
        let (gbk, _, _) = encoding_rs::GBK.encode("岗位职责");
        assert_eq!(
            decode_body(&gbk, Some("text/html; charset=gb2312")),
            "岗位职责"
        );
        let meta = format!(
            "<meta http-equiv=\"Content-Type\" content=\"text/html; charset=gbk\">{}",
            "占位"
        );
        let (body, _, _) = encoding_rs::GBK.encode(&meta);
        assert!(decode_body(&body, None).ends_with("占位"));
        assert_eq!(decode_body("纯 UTF-8".as_bytes(), None), "纯 UTF-8");
    }

    #[test]
    fn resolves_relative_redirects() {
        assert_eq!(
            join_url("https://example.com/jobs/a?x=1", "/jobs/b").unwrap(),
            "https://example.com/jobs/b"
        );
        assert_eq!(
            join_url("https://example.com/jobs/a", "https://other.example/c").unwrap(),
            "https://other.example/c"
        );
        // 重定向到本机地址仍会被逐跳校验拦下。
        assert!(validate_job_url(
            &join_url("https://example.com/a", "http://127.0.0.1/x").unwrap()
        )
        .is_err());
    }

    #[test]
    fn reads_job_posting_json_ld_from_any_page() {
        let html = r#"<html><head><script type="application/ld+json">
            {"@context":"https://schema.org","@type":"JobPosting","title":"后端工程师",
             "hiringOrganization":{"@type":"Organization","name":"示例科技"},
             "jobLocation":{"address":{"addressLocality":"上海"}},
             "description":"<p>负责 &amp; 维护服务</p>"}
            </script></head><body>ignored</body></html>"#;
        let job = extract_json_ld_job(html).unwrap();
        assert_eq!(job.title, "后端工程师");
        assert_eq!(job.company, "示例科技");
        assert_eq!(job.location.as_deref(), Some("上海"));
        assert_eq!(job.jd_text.as_deref(), Some("负责 & 维护服务"));
        assert!(extract_json_ld_job("<html><body>no data</body></html>").is_none());
    }

    #[cfg(windows)]
    #[test]
    fn protects_secrets_with_dpapi() {
        let encrypted = protect_secret("top-secret").unwrap();
        assert!(!encrypted.contains("top-secret"));
        assert_eq!(unprotect_secret(&encrypted).unwrap(), "top-secret");
    }
}
