//! Read one already-open tab through Chrome's user-enabled local DevTools connection.
use serde_json::{json, Value};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tungstenite::{client, Message, WebSocket};
use url::Url;

const SETUP: &str = "请在已登录的 Chrome 144 或更新版本打开 chrome://inspect/#remote-debugging，启用远程调试，再点一次抓取，并允许 Chrome 的连接请求。";
const CHANGED: &str = "Chrome 标签页地址已改变，请重新复制目标标签页的地址。";
const MAX_HTML: usize = 5 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(45);

fn chrome_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    return dirs::data_local_dir().map(|p| p.join("Google/Chrome/User Data"));
    #[cfg(target_os = "macos")]
    return dirs::data_dir().map(|p| p.join("Google/Chrome"));
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    dirs::config_dir().map(|p| p.join("google-chrome"))
}

fn connect(dir: &Path) -> Result<WebSocket<TcpStream>, String> {
    let info = std::fs::read_to_string(dir.join("DevToolsActivePort")).map_err(|_| SETUP)?;
    let mut lines = info.lines().map(str::trim);
    let port = lines
        .next()
        .and_then(|l| l.parse::<u16>().ok())
        .filter(|p| *p != 0)
        .ok_or(SETUP)?;
    let path = lines
        .next()
        .filter(|p| p.starts_with("/devtools/browser"))
        .ok_or(SETUP)?;
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let socket = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).map_err(|_| SETUP)?;
    socket
        .set_read_timeout(Some(TIMEOUT))
        .map_err(|e| e.to_string())?;
    socket
        .set_write_timeout(Some(TIMEOUT))
        .map_err(|e| e.to_string())?;
    client(format!("ws://{addr}{path}"), socket)
        .map(|(ws, _)| ws)
        .map_err(|_| SETUP.into())
}

fn call(
    ws: &mut WebSocket<TcpStream>,
    id: u8,
    method: &str,
    params: Value,
    session: Option<&str>,
) -> Result<Value, String> {
    let mut req = json!({ "id": id, "method": method, "params": params });
    if let Some(session) = session {
        req["sessionId"] = json!(session);
    }
    ws.send(Message::text(req.to_string()))
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    while start.elapsed() < TIMEOUT {
        match ws
            .read()
            .map_err(|_| format!("Chrome 连接已断开或超时。{SETUP}"))?
        {
            Message::Text(text) => {
                let msg: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
                if msg["id"] != id {
                    continue;
                }
                if msg.get("error").is_some() {
                    return Err(format!(
                        "Chrome 无法读取标签页：{}",
                        msg["error"]["message"].as_str().unwrap_or(method)
                    ));
                }
                return Ok(msg["result"].clone());
            }
            Message::Close(_) => break,
            _ => continue,
        }
    }
    Err(SETUP.into())
}

fn target_id<'a>(targets: &'a Value, wanted: &Url) -> Result<&'a str, String> {
    let mut matches = targets["targetInfos"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|t| {
            t["type"] == "page"
                && t["url"].as_str().and_then(|s| Url::parse(s).ok()).as_ref() == Some(wanted)
        })
        .filter_map(|t| t["targetId"].as_str());
    let id = matches
        .next()
        .ok_or("没有找到这个地址。请在 Chrome 展开目标职位，复制当前地址栏的完整链接后再试。")?;
    if matches.next().is_some() {
        return Err("Chrome 中有多个相同地址的标签页。请只保留要抓取的那一个，再试一次。".into());
    }
    Ok(id)
}

fn read_tab(dir: &Path, wanted: &Url) -> Result<String, String> {
    let mut ws = connect(dir)?;
    let targets = call(&mut ws, 1, "Target.getTargets", json!({}), None)?;
    let id = target_id(&targets, wanted)?;
    let attached = call(
        &mut ws,
        2,
        "Target.attachToTarget",
        json!({"targetId":id, "flatten":true}),
        None,
    )?;
    let session = attached["sessionId"].as_str().ok_or(CHANGED)?;
    // Recheck the address in the same evaluation that reads the DOM, in case the tab navigated.
    let expression = format!("(() => {{ if (location.href !== {}) return null; const html = document.documentElement.outerHTML; return html.length <= {MAX_HTML} ? html : false; }})()", json!(wanted.as_str()));
    let result = call(
        &mut ws,
        3,
        "Runtime.evaluate",
        json!({"expression":expression, "returnByValue":true}),
        Some(session),
    )?;
    let value = &result["result"]["value"];
    if value == false || value.as_str().is_some_and(|s| s.len() > MAX_HTML) {
        return Err("Chrome 页面超过 5 MB，请使用「网页文件」导入。".into());
    }
    value
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or(CHANGED.into())
}

pub(crate) fn capture(url: &str) -> Result<String, String> {
    crate::extract::validate_job_url(url)?;
    let wanted = Url::parse(url.trim()).map_err(|e| e.to_string())?;
    read_tab(&chrome_dir().ok_or(SETUP)?, &wanted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_tab_including_query_and_hash_must_be_unique() {
        let url = Url::parse("https://example.org/jobs?q=a#/42").unwrap();
        let tab = json!({"type":"page","url":url.as_str(),"targetId":"42"});
        let other = json!({"type":"page","url":"https://example.org/jobs?q=a#/43","targetId":"43"});
        assert_eq!(
            target_id(&json!({"targetInfos":[other, tab]}), &url).unwrap(),
            "42"
        );
        assert!(target_id(&json!({"targetInfos":[tab, tab]}), &url).is_err());
        assert!(target_id(&json!({"targetInfos":[other]}), &url).is_err());
    }

    #[test]
    #[ignore = "set OFFERFLOW_TEST_CHROME to a Chrome executable to run"]
    fn reads_rendered_tab_from_running_chrome() {
        use std::process::{Command, Stdio};
        let tmp = tempfile::tempdir().unwrap();
        let page = tmp.path().join("job.html");
        std::fs::write(&page, "<body><script>document.body.appendChild(document.createElement('h1')).textContent='OfferFlow rendered role'</script></body>").unwrap();
        let url = Url::from_file_path(&page).unwrap();
        let profile = tmp.path().join("profile");
        let mut chrome = Command::new(std::env::var_os("OFFERFLOW_TEST_CHROME").unwrap())
            .args([
                "--headless=new",
                "--remote-debugging-port=0",
                "--no-first-run",
                "--no-default-browser-check",
                "--disable-background-networking",
            ])
            .arg(format!("--user-data-dir={}", profile.display()))
            .arg(url.as_str())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let start = Instant::now();
        let result = loop {
            let result = read_tab(&profile, &url);
            if result.as_ref().is_ok_and(|s| s.contains("<h1>"))
                || start.elapsed() > Duration::from_secs(15)
            {
                break result;
            }
            std::thread::sleep(Duration::from_millis(250));
        };
        if let Ok(mut ws) = connect(&profile) {
            let _ = ws.send(Message::text(r#"{"id":1,"method":"Browser.close"}"#));
        }
        let cleanup = Instant::now();
        while chrome.try_wait().unwrap().is_none() && cleanup.elapsed() < Duration::from_secs(3) {
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = chrome.kill();
        let _ = chrome.wait();
        let html = result.unwrap();
        assert!(html.contains("<h1>OfferFlow rendered role</h1>"));
    }
}
