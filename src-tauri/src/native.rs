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
        let handle = CreateMutexW(None, false, windows::core::w!("Local\\FlowForgeAI.Desktop"))
            .map_err(|e| e.to_string())?;
        let existed = GetLastError() == ERROR_ALREADY_EXISTS;
        let owned = OwnedHandle::from_raw_handle(handle.0);
        if existed {
            if let Ok(window) = FindWindowW(None, windows::core::w!("FlowForge AI")) {
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
            return Err("FlowForge is focused; use transcript preview.".into());
        }
    }
    std::thread::spawn(|| {
        let _com = Com::initialize()?;
        unsafe { focused().map_err(|e| format!("Focused field cannot be verified: {e}")) }
    })
    .join()
    .map_err(|_| "Focused-field check failed".to_string())?
}

unsafe fn focused() -> WinResult<Target> {
    let window = GetForegroundWindow();
    let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)?;
    let element = automation.GetFocusedElement()?;
    if element.CurrentIsPassword()?.as_bool()
        || !element.CurrentIsEnabled()?.as_bool()
        || !element.CurrentHasKeyboardFocus()?.as_bool()
        || ![UIA_EditControlTypeId, UIA_DocumentControlTypeId]
            .contains(&element.CurrentControlType()?)
    {
        return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
            0x80070005u32 as i32,
        )));
    }
    if let Ok(pattern) =
        element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId)
    {
        if pattern.CurrentIsReadOnly()?.as_bool() {
            return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
                0x80070005u32 as i32,
            )));
        }
    }
    let process = element.CurrentProcessId()?;
    if process == std::process::id() as i32 {
        return Err(windows::core::Error::from_hresult(windows::core::HRESULT(
            0x80070005u32 as i32,
        )));
    }
    let array = element.GetRuntimeId()?;
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
        return Err(windows::core::Error::from_win32());
    }
    Ok(Target {
        window: window.0 as isize,
        process,
        runtime_id: result?,
    })
}

pub fn insert(target: &Target, text: &str) -> Result<(), String> {
    if text.chars().any(char::is_control) {
        return Err("Use manual copy for multiline text or control characters.".into());
    }
    if text.encode_utf16().count() > 10000 {
        return Err("Transcript is too long for automatic insertion.".into());
    }
    let _com = Com::initialize()?;
    unsafe {
        // Never release the user's physical modifiers or steal focus.
        for key in [VK_CONTROL, VK_MENU, VK_SHIFT, VK_LWIN, VK_RWIN] {
            if GetAsyncKeyState(key.0 as i32) < 0 {
                return Err("Release modifier keys, then copy the transcript manually.".into());
            }
        }
        if &focused().map_err(|e| e.to_string())? != target {
            return Err("Focus changed since recording started.".into());
        }
        let mut inputs = Vec::new();
        for unit in text.encode_utf16() {
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
        // One batch prevents our own input from interleaving with another batch.
        // Windows has no atomic "verify focus + type" API; see documented limits.
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent != inputs.len() as u32 {
            return Err("Windows blocked some input. Text may be partially inserted; review the target before copying.".into());
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
        };
        assert!(insert(&stale, "This must never be typed").is_err());
    }
}
