//! Windows accent colour: read via WinRT `UISettings` and watch for changes.

use tauri::{AppHandle, Emitter};

pub const FALLBACK_ACCENT: &str = "#0a84ff";

#[cfg(windows)]
fn read(settings: &windows::UI::ViewManagement::UISettings) -> Option<String> {
    use windows::UI::ViewManagement::UIColorType;
    let c = settings.GetColorValue(UIColorType::Accent).ok()?;
    Some(format!("#{:02x}{:02x}{:02x}", c.R, c.G, c.B))
}

/// Current accent as `#rrggbb`.
pub fn accent() -> String {
    #[cfg(windows)]
    {
        if let Some(hex) = windows::UI::ViewManagement::UISettings::new()
            .ok()
            .and_then(|s| read(&s))
        {
            return hex;
        }
    }
    FALLBACK_ACCENT.to_string()
}

/// Keeps the `UISettings` object (and its change handler) alive.
#[allow(dead_code)]
pub struct AccentWatcher(#[cfg(windows)] windows::UI::ViewManagement::UISettings);

/// Start emitting `accent-changed` (payload `#rrggbb`) when the user changes
/// the Windows accent colour or theme.
pub fn watch(app: &AppHandle) -> Option<AccentWatcher> {
    #[cfg(windows)]
    {
        use windows::Foundation::TypedEventHandler;
        use windows::UI::ViewManagement::UISettings;
        use windows::core::IInspectable;

        let settings = UISettings::new().ok()?;
        let handle = app.clone();
        settings
            .ColorValuesChanged(&TypedEventHandler::<UISettings, IInspectable>::new(
                move |sender, _| {
                    if let Some(s) = sender.as_ref()
                        && let Some(hex) = read(s)
                    {
                        let _ = handle.emit("accent-changed", hex);
                    }
                    Ok(())
                },
            ))
            .ok()?;
        Some(AccentWatcher(settings))
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        None
    }
}
