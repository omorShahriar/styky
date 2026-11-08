//! Desktop-layer z-order handling for the panel (Windows).
//!
//! The panel lives just above the wallpaper/desktop icons and below every
//! normal window. `HWND_BOTTOM` cannot be used for that: it puts a window
//! *under* the desktop (Progman), which makes it invisible. And "Show Desktop"
//! (Win+D) raises the desktop above everything that is not minimised, so a
//! window sitting just above Progman vanishes with it.
//!
//! Strategy: when no ordinary application window is visible (i.e. the desktop
//! is showing), raise the panel above the desktop (as a topmost window); otherwise sink it back to
//! just above Progman. A tiny watcher thread re-evaluates this a few times a
//! second, because Windows gives us no event for Win+D.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Set while one of our own topmost windows (quick-add, settings, ...) is up.
static SUSPENDED: AtomicBool = AtomicBool::new(false);

/// Pause/resume the Show-Desktop watcher so it never fights our other windows.
#[allow(dead_code)]
pub fn suspend(on: bool) {
    SUSPENDED.store(on, Ordering::SeqCst);
}

use tauri::{AppHandle, Manager, WebviewWindow};
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, FindWindowW, GW_HWNDNEXT, GW_HWNDPREV, GW_OWNER, GWL_EXSTYLE, GetClassNameW, GetWindow,
    GetWindowLongW, GetWindowRect, GetWindowTextLengthW, GetWindowThreadProcessId, HWND_BOTTOM,
    HWND_NOTOPMOST, HWND_TOPMOST, IsIconic, IsWindowVisible, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetWindowPos,
    WS_EX_TOOLWINDOW,
};
use windows::core::{BOOL, w};

const SHELL_CLASSES: [&str; 5] = [
    "Progman",
    "WorkerW",
    "Shell_TrayWnd",
    "Shell_SecondaryTrayWnd",
    "Windows.UI.Core.CoreWindow",
];

struct Scan {
    own_pid: u32,
    found: bool,
}

unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
    // SAFETY: lparam is the `&mut Scan` passed by `app_window_visible` below.
    unsafe {
        let scan = &mut *(lparam.0 as *mut Scan);
        if !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return BOOL(1);
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == scan.own_pid {
            return BOOL(1);
        }
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut _,
            std::mem::size_of::<u32>() as u32,
        );
        if cloaked != 0 {
            return BOOL(1);
        }
        if GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOOLWINDOW.0 != 0 {
            return BOOL(1);
        }
        if GetWindow(hwnd, GW_OWNER).is_ok_and(|o| !o.is_invalid()) {
            return BOOL(1);
        }
        if GetWindowTextLengthW(hwnd) == 0 {
            return BOOL(1);
        }
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err()
            || rect.right - rect.left < 80
            || rect.bottom - rect.top < 80
        {
            return BOOL(1);
        }
        let mut class = [0u16; 64];
        let n = GetClassNameW(hwnd, &mut class) as usize;
        let class = String::from_utf16_lossy(&class[..n]);
        if SHELL_CLASSES.contains(&class.as_str()) {
            return BOOL(1);
        }
        scan.found = true;
        BOOL(0) // stop enumerating
    }
}

/// True if at least one ordinary, un-minimised application window is showing.
pub fn app_window_visible() -> bool {
    let mut scan = Scan {
        own_pid: std::process::id(),
        found: false,
    };
    // SAFETY: `scan` outlives the synchronous EnumWindows call.
    unsafe {
        let _ = EnumWindows(Some(visit), LPARAM(&mut scan as *mut Scan as isize));
    }
    scan.found
}

/// Put the panel in the right layer for the current desktop state.
pub fn settle(window: &WebviewWindow) {
    let Ok(hwnd) = window.hwnd() else { return };
    settle_hwnd(HWND(hwnd.0), app_window_visible());
}

fn settle_hwnd(hwnd: HWND, apps_visible: bool) {
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE;
    // SAFETY: plain Win32 calls on window handles; failures are ignored.
    unsafe {
        if !apps_visible {
            // Show Desktop makes the desktop itself topmost, so the panel has
            // to be topmost too (it lands above the desktop in that group).
            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, flags);
            return;
        }
        // Leave the topmost group, then slot in directly above the desktop.
        let _ = SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, flags);
        let after = match FindWindowW(w!("Progman"), None)
            .ok()
            .and_then(|p| GetWindow(p, GW_HWNDPREV).ok())
        {
            Some(h) if h == hwnd => return, // already just above the desktop
            Some(h) => h,
            None => HWND_BOTTOM,
        };
        let _ = SetWindowPos(hwnd, Some(after), 0, 0, 0, 0, flags);
    }
}

/// True if `hwnd` is stacked above the desktop window (Progman).
fn above_desktop(hwnd: HWND) -> bool {
    // SAFETY: plain Win32 calls on window handles.
    unsafe {
        let mut cur = hwnd;
        // Walk down the z-order from the panel; if we meet Progman we are above it.
        for _ in 0..2000 {
            match GetWindow(cur, GW_HWNDNEXT) {
                Ok(next) if !next.is_invalid() => {
                    let mut class = [0u16; 16];
                    let n = windows::Win32::UI::WindowsAndMessaging::GetClassNameW(next, &mut class)
                        as usize;
                    if String::from_utf16_lossy(&class[..n]) == "Progman" {
                        return true;
                    }
                    cur = next;
                }
                _ => return false,
            }
        }
        false
    }
}

/// Watch for "Show Desktop" and keep the panel visible through it.
pub fn spawn_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last: Option<bool> = None;
        loop {
            std::thread::sleep(Duration::from_millis(200));
            let Some(win) = app.get_webview_window(crate::window::MAIN) else {
                continue;
            };
            if SUSPENDED.load(Ordering::SeqCst) || !win.is_visible().unwrap_or(false) {
                continue;
            }
            let apps = app_window_visible();
            let changed = last != Some(apps);
            last = Some(apps);
            // While the desktop is showing, re-assert only if the shell raised
            // the desktop above us (it does so a moment *after* windows minimise).
            if let Ok(hwnd) = win.hwnd() {
                let hwnd = HWND(hwnd.0);
                if changed || (!apps && !above_desktop(hwnd)) {
                    settle_hwnd(hwnd, apps);
                }
            }
        }
    });
}
