//! 用应用自带的 WebView2 在后台窗口里打开岗位页，等脚本把内容渲染出来再读页面。
//!
//! 和用户自己在浏览器里打开一样：不调用站点私有接口、不做任何解密。
//! 外部页面拿不到任何 IPC 权限（capabilities 只放行了 `main` 窗口的本地页面），
//! 页面内容只通过 `eval_with_callback` 由我们主动读出来。

use crate::extract::validate_job_url;
use crate::job_page::{detect_job, DetailSource};
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::webview::NewWindowResponse;
use tauri::{AppHandle, Manager, Url, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

const LABEL: &str = "page-loader";
const POLL: Duration = Duration::from_millis(400);
const TIMEOUT: Duration = Duration::from_secs(25);
const MAX_HTML_BYTES: usize = 5 * 1024 * 1024;
/// 和快照判断「空壳页面」用同一个量级：Moka 未渲染的框架页只有二十几个字。
const MIN_TEXT_CHARS: usize = 150;
/// 文字长度连续这么多次不变，就当页面已经加载完。
const SETTLED_POLLS: u32 = 6;

pub(crate) struct LoadedPage {
    pub url: String,
    pub html: String,
}

/// 同一时间只加载一个页面，避免两个抓取共用一个窗口。
static BUSY: Mutex<()> = Mutex::new(());

pub(crate) const STILL_LOADING: &str = "页面在 25 秒内没有加载出岗位内容，已把加载窗口显示出来。\
     如果需要登录或完成验证，请在那个窗口里处理好，然后回到这里再点一次「抓取快照」。";

/// 在页面里执行脚本并取回结果（JSON 字符串）。页面正在跳转时可能拿不到，返回 None。
fn eval(window: &WebviewWindow, js: &str) -> Option<String> {
    let (tx, rx) = mpsc::channel();
    window
        .eval_with_callback(js, move |result| {
            let _ = tx.send(result);
        })
        .ok()?;
    rx.recv_timeout(Duration::from_secs(5)).ok()
}

const PROBE_JS: &str = "(() => { try { return [document.readyState, \
     (document.body ? document.body.innerText : '').replace(/\\s/g, '').length, location.href]; } \
     catch (e) { return ['error', 0, '']; } })()";

const HTML_JS: &str = "document.documentElement.outerHTML";

fn probe(window: &WebviewWindow) -> Option<(bool, usize, String)> {
    let (state, len, url): (String, usize, String) =
        serde_json::from_str(&eval(window, PROBE_JS)?).ok()?;
    Some((state == "complete", len, url))
}

fn read_html(window: &WebviewWindow) -> Option<String> {
    let html: String = serde_json::from_str(&eval(window, HTML_JS)?).ok()?;
    (html.len() <= MAX_HTML_BYTES).then_some(html)
}

fn open_window(app: &AppHandle, url: Url) -> Result<WebviewWindow, String> {
    // 上次超时留下的窗口：用户可能刚在里面登录 / 验证过，直接在原窗口里重新打开链接。
    if let Some(window) = app.get_webview_window(LABEL) {
        window.navigate(url).map_err(|e| e.to_string())?;
        return Ok(window);
    }
    WebviewWindowBuilder::new(app, LABEL, WebviewUrl::External(url))
        .title("OfferFlow · 加载岗位页面")
        .inner_size(1100.0, 800.0)
        .visible(false)
        .focused(false)
        // 只允许普通网页地址；弹窗一律不开。
        .on_navigation(|u| matches!(u.scheme(), "http" | "https"))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .build()
        .map_err(|e| format!("无法打开页面加载窗口：{e}"))
}

/// 打开链接，等岗位内容出现后返回渲染后的页面。成功后关闭窗口；超时则把窗口显示给用户处理。
pub(crate) fn load_page(app: &AppHandle, url: &str) -> Result<LoadedPage, String> {
    let url = url.trim();
    validate_job_url(url)?;
    let parsed = Url::parse(url).map_err(|e| format!("链接无效：{e}"))?;
    let _guard = BUSY
        .try_lock()
        .map_err(|_| "正在加载另一个页面，请稍候".to_string())?;

    let window = open_window(app, parsed)?;
    let started = Instant::now();
    let (mut last_len, mut settled) = (usize::MAX, 0u32);
    while started.elapsed() < TIMEOUT {
        std::thread::sleep(POLL);
        let Some((complete, len, current)) = probe(&window) else {
            continue;
        };
        settled = if complete && len >= MIN_TEXT_CHARS && len == last_len {
            settled + 1
        } else {
            0
        };
        last_len = len;
        if settled < 2 {
            continue;
        }
        let Some(html) = read_html(&window) else {
            continue;
        };
        // 认出了岗位就不用再等；认不出的普通页面，等文字稳定一会儿再收。
        let recognised = matches!(
            detect_job(&html, Some(&current)),
            Some((_, DetailSource::Structured | DetailSource::Site))
        );
        if recognised || settled >= SETTLED_POLLS {
            let _ = window.close();
            return Ok(LoadedPage { url: current, html });
        }
    }
    let _ = window.show();
    let _ = window.set_focus();
    Err(STILL_LOADING.to_string())
}
