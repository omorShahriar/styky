//! Microsoft Store (MSIX) flavor.
//!
//! The same binary ships in the NSIS build and in the MSIX package. At runtime we ask Windows
//! whether the process has package identity; only then do the Store-specific paths kick in:
//! no self-updater (the Store updates the app), launch at login through the manifest's
//! `windows.startupTask` extension instead of the HKCU Run key, and the package AUMID for toasts.

/// Task id declared in `packaging/msix/AppxManifest.xml` (`uap5:StartupTask TaskId`).
pub const STARTUP_TASK_ID: &str = "StykyStartup";

/// Launch-at-login state as far as the UI cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Startup {
    Enabled,
    Disabled,
    /// Disabled in Task Manager > Startup apps (or by policy); the app cannot turn it back on.
    Blocked,
}

/// True when the process runs with package identity (installed from an MSIX).
#[cfg(windows)]
pub fn is_packaged() -> bool {
    use std::sync::OnceLock;
    use windows::Win32::Storage::Packaging::Appx::GetCurrentPackageFullName;
    static PACKAGED: OnceLock<bool> = OnceLock::new();
    *PACKAGED.get_or_init(|| {
        let mut len = 0u32;
        // APPMODEL_ERROR_NO_PACKAGE (15700) when unpackaged, ERROR_INSUFFICIENT_BUFFER (122) when packaged.
        let rc = unsafe { GetCurrentPackageFullName(&mut len, None) };
        rc.0 != 15700
    })
}

#[cfg(not(windows))]
pub fn is_packaged() -> bool {
    false
}

/// Store flavor: packaged at runtime, or built with `--features store` (fallback / testing).
/// Decides what the UI offers (e.g. no "Check for updates"); identity-dependent code uses [`is_packaged`].
pub fn is_store() -> bool {
    cfg!(feature = "store") || is_packaged()
}

/// `PackageFamilyName!AppId`, e.g. `Publisher.Styky_8wekyb3d8bbwe!Styky`.
#[cfg(windows)]
pub fn package_aumid() -> Option<String> {
    use windows::Win32::Storage::Packaging::Appx::GetCurrentApplicationUserModelId;
    use windows::core::PWSTR;
    if !is_packaged() {
        return None;
    }
    let mut len = 0u32;
    let _ = unsafe { GetCurrentApplicationUserModelId(&mut len, None) };
    if len == 0 {
        return None;
    }
    let mut buf = vec![0u16; len as usize];
    let rc = unsafe { GetCurrentApplicationUserModelId(&mut len, Some(PWSTR(buf.as_mut_ptr()))) };
    if rc.0 != 0 {
        return None;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..end]))
}

#[cfg(not(windows))]
pub fn package_aumid() -> Option<String> {
    None
}

/// WinRT async calls are awaited on a throwaway thread: a Tauri sync command may run on an STA thread, where blocking `.get()` is not allowed.
#[cfg(windows)]
fn blocking<T: Send + 'static>(f: impl FnOnce() -> windows::core::Result<T> + Send + 'static) -> Option<T> {
    std::thread::spawn(move || f().ok()).join().ok().flatten()
}

#[cfg(windows)]
pub fn startup_state() -> Option<Startup> {
    use windows::ApplicationModel::{StartupTask, StartupTaskState};
    use windows::core::HSTRING;
    blocking(|| {
        let task = StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))?.get()?;
        Ok(match task.State()? {
            StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy => Startup::Enabled,
            StartupTaskState::Disabled => Startup::Disabled,
            _ => Startup::Blocked,
        })
    })
}

#[cfg(windows)]
pub fn set_startup(enabled: bool) -> Option<Startup> {
    use windows::ApplicationModel::StartupTask;
    use windows::core::HSTRING;
    blocking(move || {
        let task = StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))?.get()?;
        if enabled {
            task.RequestEnableAsync()?.get()?;
        } else {
            task.Disable()?;
        }
        Ok(())
    })?;
    startup_state()
}

#[cfg(not(windows))]
pub fn startup_state() -> Option<Startup> {
    None
}

#[cfg(not(windows))]
pub fn set_startup(_enabled: bool) -> Option<Startup> {
    None
}
