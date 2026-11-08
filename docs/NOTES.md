# Styky — implementation notes

Running log of deviations from `PLAN.md`, findings and known limitations.

## Toolchain (M0)

- Rust 1.99 (`stable-x86_64-pc-windows-msvc`) via `winget Rustlang.Rustup`; VS 2022 Build Tools (VCTools workload) via winget.
  The VS installer reports "Restart your PC to finish installation", but `link.exe` works without a reboot.
- A fresh shell does not have `%USERPROFILE%\.cargo\bin` on `PATH` until the next login; prepend it by hand (`$env:Path += ";$env:USERPROFILE\.cargo\bin"`).
- `create-tauri-app` was run in a scratch directory and copied in, because `D:\dev\styky` already contained `docs/`.
- Scaffold versions: Tauri 2.12, React 19 (plan said 18; the template's 19 is current), Vite 8, TypeScript 6. `edition = "2024"`.
- `productName` is `Styky`, identifier `com.styky.app`.

## Deviations / decisions (M1)

1. **Acrylic is the default material, Mica is the fallback** (plan: Mica first). Mica only renders translucently while its
   window is *active*; an inactive window gets a flat fallback colour, and a desktop-layer widget is inactive almost always.
   Acrylic keeps its blur when inactive. `STYKY_MATERIAL=mica|acrylic|none` overrides it for experiments. Acrylic tint alpha is 0
   so only the CSS tint (`--panel-tint`, per the plan's rgba values) colours the panel. Look on very dark wallpapers is close to
   flat grey; it should be re-checked by eye on a lighter wallpaper (see "Not verified").
2. **`set_always_on_bottom` / `HWND_BOTTOM` is not used.** HWND_BOTTOM puts the panel *under* Progman (the desktop), so the
   panel disappears whenever the desktop is shown. `src-tauri/src/desktop.rs` instead inserts the panel directly above Progman
   (below all normal windows). It is re-applied on `Focused(false)`.
3. **Window-state saving.** The plugin does not save when we `prevent_close()`, so we save explicitly on close-to-hide and
   (debounced 600 ms) on every move/resize. `VISIBLE` and `DECORATIONS` are excluded from restore (the app always shows at
   start; it is frameless). First run places the panel top-right with a 24 px margin of the primary work area.
4. Tray menu has an extra **Launch at login** check item (wired to the autostart commands) so the toggle is usable before the
   Settings window exists (M4). Header `+` and the `⋯ → Settings…` entry are present but disabled placeholders until M2/M4.
5. UI uses CSS transitions/keyframes only for now (no `motion` dependency yet); the spring pass is M4. Added deps:
   `@fontsource-variable/inter`, `lucide-react`.
6. `--accent` is read from Rust (`UISettings.GetColorValue(Accent)`, command `get_accent`) at start and updated live via the
   `accent-changed` event (WinRT `ColorValuesChanged`). Light/dark follow `prefers-color-scheme`, which WebView2 tracks live.

## Win+D / Show Desktop investigation (time-boxed; ~1h used)

**Findings**

- Win+D does **not** just minimise windows: Windows also makes the desktop (`Progman`) a *topmost* window for as long as the
  desktop is shown. Anything at the bottom of the normal z-order, including a window placed just above Progman, is covered.
- A `skipTaskbar` panel (WS_EX_TOOLWINDOW) is **not** minimised by Win+D (it stays `IsWindowVisible` and non-iconic), so it is
  only the z-order that hides it.
- `HWND_BOTTOM` is worse: the panel is under the desktop and invisible even when no windows are open.

**Solution that shipped (robust, input still works — it is a normal top-level window, no reparenting):**
`desktop.rs` runs a 200 ms watcher thread. If no ordinary app window is visible (visible, not minimised, not cloaked, not a
tool window, not owned, titled, ≥ 80×80, not shell classes, not our own process) it makes the panel `HWND_TOPMOST`, which lands
above the desktop; as soon as an app window is visible again it drops back to "directly above Progman". Verified with
synthesized Win+D key presses and z-order dumps (`scripts/zorder.ps1`, `scripts/windtest.ps1`):

| state | z-order |
|---|---|
| apps visible | panel just above Progman (below all apps) |
| Win+D (desktop shown) | panel topmost, above Progman; screenshot confirms it is painted |
| Win+D again | back to just above Progman |

**Rejected:** WorkerW/Progman reparenting (`SetParent`) — a child window cannot use the DWM acrylic/Mica backdrop and is
awkward for WebView2; not worth losing the glass.

**Known limitations**

- Detection is polling (200 ms), so after Win+D the panel can take up to ~0.2 s to reappear.
- With *no* app windows open at all the panel is topmost (harmless, nothing to cover). A *topmost* panel would also sit above the
  taskbar if the user drags it over it, only while the desktop is shown.
- Other "show desktop"-like states (e.g. all windows minimised by hand) behave the same as Win+D: the panel is raised. That is
  the intended behaviour.
- A full-screen-exclusive app/game hides the panel like any window; not handled.

## Not verified (could not be checked from this session)

- Light theme / live light↔dark switching: not exercised (changing the user's Windows theme was out of bounds). The tokens exist
  in both modes and `prefers-color-scheme` is the mechanism; only dark was seen on screen.
- Mica fallback path and Windows accent *change* event (initial accent read was not eyeballed; the purple accent seen in the
  empty-state icon matches this machine's accent).
- Tray: left-click toggle, menu items, "Launch at login" toggle, the autostart registry entry (code compiles; not clicked).
- Header drag region and edge-resize with the mouse, hover/press states (only static screenshots were taken).
- Behaviour at display scales other than 100 % / on multi-monitor.
- Close-to-hide, single-instance (second launch re-shows the hidden panel) and restart persistence **were** verified by
  script (window message + relaunch + rect query).

## Dev helpers

`scripts/*.ps1` are Windows-only debugging helpers (`winpos`, `zorder`, `windtest`, `moveshot`) used for the checks above.
