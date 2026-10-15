//! The foreground window's app behind [`ForegroundApp`], `GetLastInputInfo` behind
//! [`UserIdle`], and handing focus back to a window.
//!
//! An app is known by its exe file name, such as `KeePassXC.exe`, in [`AppRef::bundle_id`]:
//! unlike the full path, it's the same for every user and install folder, so excluded apps
//! match wherever the app is installed. The name is the exe's `FileDescription`.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows_sys::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
};
use windows_sys::Win32::System::SystemInformation::GetTickCount;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumChildWindows, EnumWindows, GW_OWNER, GWL_EXSTYLE, GetForegroundWindow, GetWindow,
    GetWindowLongW, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindow, IsWindowVisible,
    SetForegroundWindow, WS_EX_TOOLWINDOW,
};
use windows_sys::core::BOOL;

use super::{from_wide, wide};
use crate::apps::{AppRef, ForegroundApp, UserIdle};

/// Hosts the windows of Store apps, which belong to another process.
const FRAME_HOST: &str = "ApplicationFrameHost.exe";

/// The app that owns the foreground window. Names are read from the exe once and kept.
#[derive(Debug, Default)]
pub struct WinForegroundApp {
    names: Mutex<HashMap<String, Option<String>>>,
}

impl ForegroundApp for WinForegroundApp {
    fn current(&self) -> Option<AppRef> {
        // SAFETY: takes nothing.
        let window = unsafe { GetForegroundWindow() };
        let path = window_exe(window)?;
        let mut names = self.names.lock().unwrap_or_else(PoisonError::into_inner);
        let name = names
            .entry(path.clone())
            .or_insert_with(|| describe(&path))
            .clone();
        Some(AppRef {
            bundle_id: exe_name(&path),
            name: name.or_else(|| stem(&path)),
        })
    }

    /// Only Kopi names itself on the clipboard on Windows, so there's nothing to look up.
    fn running(&self, _bundle_id: &str) -> Option<AppRef> {
        None
    }
}

/// The apps with a window on the taskbar, for choosing apps to exclude. Kopi isn't in it.
pub fn running_apps() -> Vec<AppRef> {
    let mut windows: Vec<HWND> = Vec::new();
    // SAFETY: the callback only runs during the call, and `windows` outlives it.
    unsafe { EnumWindows(Some(collect), (&raw mut windows) as LPARAM) };
    // SAFETY: takes nothing.
    let own = unsafe { GetCurrentProcessId() };
    let mut seen = Vec::new();
    let mut apps = Vec::new();
    for window in windows {
        if !on_taskbar(window) || process_id(window) == Some(own) {
            continue;
        }
        let Some(path) = window_exe(window) else {
            continue;
        };
        if seen.contains(&path) {
            continue;
        }
        apps.push(AppRef {
            bundle_id: exe_name(&path),
            name: describe(&path).or_else(|| stem(&path)),
        });
        seen.push(path);
    }
    apps
}

/// A window to hand focus back to. A plain number, so it can cross threads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRef(isize);

/// The foreground window, unless it's one of Kopi's own.
pub fn foreground_window() -> Option<WindowRef> {
    // SAFETY: takes nothing.
    let window = unsafe { GetForegroundWindow() };
    // SAFETY: takes nothing.
    let own = unsafe { GetCurrentProcessId() };
    (!window.is_null() && process_id(window) != Some(own)).then_some(WindowRef(window as isize))
}

/// Brings `window` to the front, if it still exists. Windows allows it while Kopi is in
/// front, as it is while the panel has focus.
pub fn activate(window: WindowRef) -> bool {
    let window = window.0 as HWND;
    // SAFETY: IsWindow accepts any value, and SetForegroundWindow a live window.
    unsafe { IsWindow(window) != 0 && SetForegroundWindow(window) != 0 }
}

/// Idle time from `GetLastInputInfo`, which covers this session's keyboard and mouse.
#[derive(Debug, Clone, Default)]
pub struct WinUserIdle;

impl UserIdle for WinUserIdle {
    fn idle_time(&self) -> Option<Duration> {
        let mut info = LASTINPUTINFO {
            cbSize: size_of::<LASTINPUTINFO>() as u32,
            dwTime: 0,
        };
        // SAFETY: `info` is a LASTINPUTINFO with its size set.
        if unsafe { GetLastInputInfo(&mut info) } == 0 {
            return None;
        }
        // Both tick counts wrap after 49.7 days, so subtract with wrapping.
        // SAFETY: takes nothing.
        let now = unsafe { GetTickCount() };
        Some(Duration::from_millis(u64::from(
            now.wrapping_sub(info.dwTime),
        )))
    }
}

unsafe extern "system" fn collect(window: HWND, param: LPARAM) -> BOOL {
    // SAFETY: `param` is the Vec that `running_apps` passed.
    let windows = unsafe { &mut *(param as *mut Vec<HWND>) };
    windows.push(window);
    1
}

unsafe extern "system" fn find_other_process(window: HWND, param: LPARAM) -> BOOL {
    // SAFETY: `param` is the pair that `window_exe` passed.
    let (host, found) = unsafe { &mut *(param as *mut (u32, Option<u32>)) };
    match process_id(window) {
        Some(pid) if pid != *host => {
            *found = Some(pid);
            0
        }
        _ => 1,
    }
}

/// Whether a top-level window shows on the taskbar: visible, titled, not owned by another
/// window and not a tool window.
fn on_taskbar(window: HWND) -> bool {
    // SAFETY: plain queries on a window handle from EnumWindows.
    unsafe {
        IsWindowVisible(window) != 0
            && GetWindowTextLengthW(window) > 0
            && GetWindow(window, GW_OWNER).is_null()
            && (GetWindowLongW(window, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW) == 0
    }
}

fn process_id(window: HWND) -> Option<u32> {
    let mut pid = 0;
    // SAFETY: `pid` is a local.
    unsafe { GetWindowThreadProcessId(window, &mut pid) };
    (pid != 0).then_some(pid)
}

/// The exe behind a window. A Store app's window belongs to ApplicationFrameHost, so its
/// child window from another process names the app.
fn window_exe(window: HWND) -> Option<String> {
    if window.is_null() {
        return None;
    }
    let pid = process_id(window)?;
    let path = process_path(pid)?;
    if exe_name(&path).is_some_and(|name| name.eq_ignore_ascii_case(FRAME_HOST)) {
        let mut search = (pid, None::<u32>);
        // SAFETY: the callback only runs during the call, and `search` outlives it.
        unsafe {
            EnumChildWindows(
                window,
                Some(find_other_process),
                (&raw mut search) as LPARAM,
            )
        };
        if let Some(child) = search.1 {
            return process_path(child).or(Some(path));
        }
    }
    Some(path)
}

fn process_path(pid: u32) -> Option<String> {
    // SAFETY: the handle is closed before returning, and the buffer's size is passed with it.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        let ok =
            QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
        CloseHandle(process);
        (ok != 0).then(|| from_wide(&buf[..len as usize]))
    }
}

fn exe_name(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
}

fn stem(path: &str) -> Option<String> {
    Path::new(path)
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
}

/// The exe's `FileDescription`, such as "Google Chrome", or its `ProductName`.
fn describe(path: &str) -> Option<String> {
    let path = wide(path);
    // SAFETY: `info` is sized as GetFileVersionInfoSizeW asks, and VerQueryValueW's results
    // point into it, which is alive while they're read.
    unsafe {
        let size = GetFileVersionInfoSizeW(path.as_ptr(), std::ptr::null_mut());
        if size == 0 {
            return None;
        }
        let mut info = vec![0u8; size as usize];
        if GetFileVersionInfoW(path.as_ptr(), 0, size, info.as_mut_ptr().cast()) == 0 {
            return None;
        }
        let query = |key: &str| -> Option<(*const u8, u32)> {
            let mut ptr = std::ptr::null_mut();
            let mut len = 0;
            let ok = VerQueryValueW(info.as_ptr().cast(), wide(key).as_ptr(), &mut ptr, &mut len);
            (ok != 0 && !ptr.is_null() && len > 0).then_some((ptr.cast_const().cast(), len))
        };
        // The first language and code page, or US English in Unicode.
        let (lang, codepage) = match query(r"\VarFileInfo\Translation") {
            Some((ptr, len)) if len >= 4 => {
                let pair = std::slice::from_raw_parts(ptr.cast::<u16>(), 2);
                (pair[0], pair[1])
            }
            _ => (0x0409, 0x04B0),
        };
        ["FileDescription", "ProductName"].iter().find_map(|key| {
            let (ptr, len) = query(&format!(r"\StringFileInfo\{lang:04x}{codepage:04x}\{key}"))?;
            let text = from_wide(std::slice::from_raw_parts(ptr.cast::<u16>(), len as usize));
            let text = text.trim();
            (!text.is_empty()).then(|| text.to_owned())
        })
    }
}
