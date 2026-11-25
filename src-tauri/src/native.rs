//! Read only focused-control metadata. Never inspect text or password contents.
use windows::{
    core::Result as WinResult,
    Win32::{
        Foundation::*,
        System::{Com::*, DataExchange::*, Memory::*, Ole::*, Threading::*},
        UI::{Accessibility::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
};

use std::os::windows::io::{FromRawHandle, OwnedHandle};

pub fn single_instance() -> Result<Option<OwnedHandle>, String> {
    unsafe {
        let handle = CreateMutexW(None, false, windows::core::w!("Local\\SONIQ.Desktop"))
            .map_err(|e| e.to_string())?;
        let existed = GetLastError() == ERROR_ALREADY_EXISTS;
        let owned = OwnedHandle::from_raw_handle(handle.0);
        if existed {
            if let Ok(window) = FindWindowW(None, windows::core::w!("SONIQ")) {
                let _ = ShowWindow(window, SW_RESTORE);
                let _ = SetForegroundWindow(window);
            }
            return Ok(None);
        }
        Ok(Some(owned))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Target {
    window: isize,
    process: i32,
    runtime_id: Vec<i32>,
    rich_edit: Option<isize>,
}

struct Com;
impl Com {
    fn initialize() -> Result<Self, String> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|e| e.to_string())?;
        }
        Ok(Self)
    }
}
impl Drop for Com {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

pub fn focused_target() -> Result<Target, String> {
    unsafe {
        let mut process = 0;
        GetWindowThreadProcessId(GetForegroundWindow(), Some(&mut process));
        if process == std::process::id() {
            return Err("SONIQ is focused; use transcript preview.".into());
        }
    }
    std::thread::spawn(|| {
        let _com = Com::initialize()?;
        unsafe { focused().map_err(|e| format!("Focused field cannot be verified: {e}")) }
    })
    .join()
    .map_err(|_| "Focused-field check failed".to_string())?
}

unsafe fn focused() -> Result<Target, String> {
    fn inspect<T>(result: WinResult<T>, property: &str) -> Result<T, String> {
        result.map_err(|error| format!("Cannot verify {property}: {error}"))
    }
    let window = GetForegroundWindow();
    let automation: IUIAutomation = inspect(
        CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER),
        "UI Automation",
    )?;
    let element = inspect(automation.GetFocusedElement(), "focused element")?;
    let refusal = if inspect(element.CurrentIsPassword(), "password status")?.as_bool() {
        Some("Password fields require manual handling.")
    } else if !inspect(element.CurrentIsEnabled(), "enabled status")?.as_bool() {
        Some("The focused field is disabled.")
    } else if !inspect(element.CurrentHasKeyboardFocus(), "keyboard focus")?.as_bool() {
        Some("The field no longer has keyboard focus.")
    } else if ![UIA_EditControlTypeId, UIA_DocumentControlTypeId]
        .contains(&inspect(element.CurrentControlType(), "control type")?)
    {
        Some("The focused control is not a supported text field.")
    } else {
        None
    };
    if let Some(reason) = refusal {
        return Err(reason.into());
    }
    if let Ok(pattern) =
        element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
    {
        if inspect(pattern.CurrentIsReadOnly(), "read-only status")?.as_bool() {
            return Err("The focused field is read-only.".into());
        }
    }
    let process = inspect(element.CurrentProcessId(), "field process")?;
    if process == std::process::id() as i32 {
        return Err("SONIQ is focused; use transcript preview.".into());
    }
    let rich_edit = if inspect(element.CurrentClassName(), "control class")? == "RichEditD2DPT" {
        let handle = inspect(element.CurrentNativeWindowHandle(), "editor window")?;
        let mut native_process = 0;
        GetWindowThreadProcessId(handle, Some(&mut native_process));
        if handle.0.is_null()
            || !IsWindowUnicode(handle).as_bool()
            || native_process != process as u32
        {
            return Err("Cannot verify the Unicode editor window.".into());
        }
        Some(handle.0 as isize)
    } else {
        None
    };
    let array = inspect(element.GetRuntimeId(), "field identity")?;
    let result = (|| -> WinResult<Vec<i32>> {
        let lower = SafeArrayGetLBound(array, 1)?;
        let upper = SafeArrayGetUBound(array, 1)?;
        if upper < lower || upper - lower > 64 {
            return Err(windows::core::Error::from_win32());
        }
        let mut values = Vec::new();
        for index in lower..=upper {
            let mut value = 0i32;
            SafeArrayGetElement(array, &index, &mut value as *mut _ as *mut _)?;
            values.push(value);
        }
        Ok(values)
    })();
    let _ = SafeArrayDestroy(array);
    if GetForegroundWindow() != window {
        return Err("Focus changed while checking the field.".into());
    }
    Ok(Target {
        window: window.0 as isize,
        process,
        runtime_id: inspect(result, "field identity values")?,
        rich_edit,
    })
}

pub fn insert(target: &Target, text: &str, is_current: impl Fn() -> bool) -> Result<(), String> {
    if text.chars().any(char::is_control) {
        return Err("Use manual copy for multiline text or control characters.".into());
    }
    if text.encode_utf16().count() > 10000 {
        return Err("Transcript is too long for automatic insertion.".into());
    }
    let _com = Com::initialize()?;
    let verify = || -> Result<(), String> {
        if !is_current() {
            return Err("Insertion cancelled.".into());
        }
        unsafe {
            for key in [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN] {
                if GetAsyncKeyState(key.0 as i32) < 0 {
                    return Err("Release modifier keys, then copy the transcript manually.".into());
                }
            }
            if &focused()? != target {
                return Err("Focus changed since recording started.".into());
            }
        }
        if !is_current() {
            return Err("Insertion cancelled.".into());
        }
        Ok(())
    };
    verify()?;
    if target.rich_edit.is_none() {
        let mut inputs = Vec::new();
        let mut sent_any = false;
        for character in text.chars() {
            if let Err(error) = verify() {
                return Err(if sent_any {
                    format!(
                        "{error} Text may be partially inserted; review the target before copying."
                    )
                } else {
                    error
                });
            }
            inputs.clear();
            let mut units = [0; 2];
            for &unit in character.encode_utf16(&mut units).iter() {
                for flags in [KEYEVENTF_UNICODE, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP] {
                    inputs.push(INPUT {
                        r#type: INPUT_KEYBOARD,
                        Anonymous: INPUT_0 {
                            ki: KEYBDINPUT {
                                wVk: VIRTUAL_KEY(0),
                                wScan: unit,
                                dwFlags: flags,
                                time: 0,
                                dwExtraInfo: 0,
                            },
                        },
                    });
                }
            }
            if !is_current() {
                return Err("Insertion cancelled. Text may be partially inserted; review the target before copying.".into());
            }
            let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
            if sent != inputs.len() as u32 {
                return Err("Windows blocked some input. Text may be partially inserted; review the target before copying.".into());
            }
            sent_any = true;
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        return Ok(());
    }
    let handle = target.rich_edit.unwrap();
    let mut attempted = false;
    for character in text.chars() {
        let result = (|| -> Result<(), String> {
            verify()?;
            let mut units = [0; 2];
            // Modern Notepad corrupted queued VK_PACKET input. Its verified
            // Unicode RichEdit HWND accepts synchronous WM_CHAR instead.
            for &unit in character.encode_utf16(&mut units).iter() {
                attempted = true;
                let sent = unsafe {
                    SendMessageTimeoutW(
                        HWND(handle as *mut _),
                        WM_CHAR,
                        WPARAM(unit as usize),
                        LPARAM(1),
                        SMTO_ABORTIFHUNG | SMTO_ERRORONEXIT,
                        1000,
                        None,
                    )
                };
                if sent.0 == 0 {
                    return Err("The editor blocked input or did not respond.".into());
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            return Err(if attempted {
                format!("{error} Text may be partially inserted; review the target before copying.")
            } else {
                error
            });
        }
    }
    Ok(())
}

pub fn copy(text: &str) -> Result<(), String> {
    unsafe {
        OpenClipboard(Some(GetForegroundWindow()))
            .map_err(|e| format!("Clipboard is busy: {e}"))?;
        let result = (|| -> Result<(), String> {
            let units: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
            let memory = GlobalAlloc(GMEM_MOVEABLE, units.len() * 2).map_err(|e| e.to_string())?;
            let pointer = GlobalLock(memory);
            if pointer.is_null() {
                let _ = GlobalFree(Some(memory));
                return Err("Cannot allocate clipboard text.".into());
            }
            std::ptr::copy_nonoverlapping(units.as_ptr(), pointer as *mut u16, units.len());
            let _ = GlobalUnlock(memory);
            if let Err(error) = EmptyClipboard()
                .and_then(|_| SetClipboardData(13, Some(HANDLE(memory.0))).map(|_| ()))
            {
                let _ = GlobalFree(Some(memory));
                return Err(error.to_string());
            }
            Ok(())
        })();
        let _ = CloseClipboard();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_target_is_refused_before_typing() {
        let stale = Target {
            window: 0,
            process: 0,
            runtime_id: vec![],
            rich_edit: None,
        };
        assert!(insert(&stale, "This must never be typed", || true).is_err());
    }

    #[test]
    fn submission_keys_and_oversized_text_are_refused_before_target_access() {
        let target = Target {
            window: 0,
            process: 0,
            runtime_id: vec![],
            rich_edit: None,
        };
        for text in [
            "line\nbreak",
            "tab\tkey",
            "return\rkey",
            "escape\u{1b}",
            "null\0",
        ] {
            let error =
                insert(&target, text, || panic!("Target must not be accessed")).unwrap_err();
            assert!(error.contains("control characters"));
        }
        let error = insert(&target, &"x".repeat(10001), || {
            panic!("Target must not be accessed")
        })
        .unwrap_err();
        assert!(error.contains("too long"));
    }

    #[test]
    fn cancelled_session_is_refused_before_target_access() {
        let target = Target {
            window: 0,
            process: 0,
            runtime_id: vec![],
            rich_edit: None,
        };
        assert_eq!(
            insert(&target, "Do not send", || false).unwrap_err(),
            "Insertion cancelled."
        );
    }
}
