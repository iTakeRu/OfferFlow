//! Read the active document in the last visible Chrome window through Windows UI Automation.
use std::path::Path;
use std::time::{Duration, Instant};
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND};
use windows::Win32::System::Com::*;
use windows::Win32::System::Threading::*;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::WindowsAndMessaging::*;

const NO_TAB: &str =
    "请在 Chrome 展开 BOSS 直聘的目标职位，保持窗口未最小化，切回 OfferFlow 再点一次。";

fn is_chrome(hwnd: HWND) -> bool {
    unsafe {
        let mut class = [0u16; 64];
        let len = GetClassNameW(hwnd, &mut class) as usize;
        if !IsWindowVisible(hwnd).as_bool()
            || IsIconic(hwnd).as_bool()
            || String::from_utf16_lossy(&class[..len]) != "Chrome_WidgetWin_1"
            || GetWindowTextLengthW(hwnd) == 0
        {
            return false;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
            return false;
        };
        let mut path = [0u16; 1024];
        let mut length = path.len() as u32;
        let result = QueryFullProcessImageNameW(
            process,
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            &mut length,
        );
        let _ = CloseHandle(process);
        result.is_ok()
            && Path::new(&String::from_utf16_lossy(&path[..length as usize]))
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("chrome.exe"))
    }
}

fn last_chrome() -> Option<HWND> {
    unsafe {
        let mut hwnd = GetTopWindow(None).ok()?;
        for _ in 0..1024 {
            if is_chrome(hwnd) {
                return Some(hwnd);
            }
            hwnd = GetWindow(hwnd, GW_HWNDNEXT).ok()?;
        }
        None
    }
}

pub(crate) fn read_boss() -> Result<(String, String), String> {
    read_window(last_chrome().ok_or(NO_TAB)?, crate::boss::is_url)
}

fn read_window(hwnd: HWND, allowed: impl Fn(&str) -> bool) -> Result<(String, String), String> {
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED)
            .ok()
            .map_err(|e| e.to_string())?;
        // Release all UIA interfaces before leaving this thread's COM apartment.
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe { CoUninitialize() };
            }
        }
        let _apartment = Apartment;
        let ui: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            .map_err(|e| e.to_string())?;
        let window = ui.ElementFromHandle(hwnd).map_err(|_| NO_TAB)?;
        let kind = ui
            .CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &VARIANT::from(UIA_DocumentControlTypeId.0),
            )
            .map_err(|e| e.to_string())?;
        let onscreen = ui
            .CreatePropertyCondition(UIA_IsOffscreenPropertyId, &VARIANT::from(false))
            .map_err(|e| e.to_string())?;
        let condition = ui
            .CreateAndCondition(&kind, &onscreen)
            .map_err(|e| e.to_string())?;
        // Chrome can take a moment to expose its accessibility tree on the first read.
        let start = Instant::now();
        let doc = loop {
            match window.FindFirst(TreeScope_Descendants, &condition) {
                Ok(doc) => break doc,
                Err(_) if start.elapsed() < Duration::from_secs(3) => {
                    std::thread::sleep(Duration::from_millis(150))
                }
                Err(_) => return Err(NO_TAB.into()),
            }
        };
        let address: IUIAutomationValuePattern = doc
            .GetCurrentPatternAs(UIA_ValuePatternId)
            .map_err(|_| NO_TAB)?;
        let url = address.CurrentValue().map_err(|_| NO_TAB)?.to_string();
        if !allowed(&url) {
            return Err(NO_TAB.into());
        }
        let pattern: IUIAutomationTextPattern = doc
            .GetCurrentPatternAs(UIA_TextPatternId)
            .map_err(|_| NO_TAB)?;
        let max = crate::snapshot::MAX_TEXT_CHARS;
        let range = pattern.DocumentRange().map_err(|_| NO_TAB)?;
        let text = range.GetText(max as i32 + 1).map_err(|_| NO_TAB)?;
        if text.len() > max {
            return Err("页面正文过长，请只复制目标岗位描述。".into());
        }
        if address.CurrentValue().map_err(|_| NO_TAB)? != url.as_str() {
            return Err(NO_TAB.into());
        }
        Ok((url, text.to_string().replace('\u{fffc}', " ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a dedicated Chrome window with the supplied fixture URL"]
    fn reads_fixture_without_debugging() {
        let hwnd = std::env::var("OFFERFLOW_UIA_TEST_HWND")
            .unwrap()
            .parse::<usize>()
            .unwrap();
        let hwnd = HWND(hwnd as *mut _);
        assert!(is_chrome(hwnd));
        let url = std::env::var("OFFERFLOW_UIA_TEST_URL").unwrap();
        assert!(read_window(hwnd, |_| false).is_err());
        let (_, text) = read_window(hwnd, |source| source == url).unwrap();
        assert!(text.contains("Dynamic role"));
        assert_eq!(crate::boss::from_text(&text).unwrap().company, "示例科技");
    }
}
