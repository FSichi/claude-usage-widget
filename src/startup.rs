//! How Windows gets to start the widget: the Run key and the Start menu.

use std::path::{Path, PathBuf};

use windows::core::{w, Interface, Result, HSTRING, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, IPersistFile, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RRF_RT_REG_SZ,
};
use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

use crate::settings;

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE: PCWSTR = w!("ClaudeUsageWidget");
const MENU_FOLDER: &str = r"Microsoft\Windows\Start Menu\Programs";
const MENU_ENTRY: &str = "Claude Usage Widget.lnk";

/// Without this entry, a widget someone quit has no way back short of hunting
/// for the exe. On its own thread, since it needs COM and a moment of disk.
pub fn ensure_menu_entry() {
    std::thread::spawn(|| {
        let Some(link) = menu_entry_path().filter(|link| !link.exists()) else {
            return;
        };
        let Ok(exe) = std::env::current_exe() else {
            return;
        };
        if let Err(error) = write_shortcut(&exe, &link) {
            settings::log(&format!("start menu entry failed: {error}"));
        }
    });
}

fn menu_entry_path() -> Option<PathBuf> {
    let roaming = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(roaming).join(MENU_FOLDER).join(MENU_ENTRY))
}

fn write_shortcut(target: &Path, link: &Path) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let shortcut: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)?;
        shortcut.SetPath(&HSTRING::from(target))?;
        if let Some(folder) = target.parent() {
            shortcut.SetWorkingDirectory(&HSTRING::from(folder))?;
        }
        shortcut
            .cast::<IPersistFile>()?
            .Save(&HSTRING::from(link), true)
    }
}

pub fn is_enabled() -> bool {
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            RUN_KEY,
            VALUE,
            RRF_RT_REG_SZ,
            None,
            None,
            None,
        )
    };
    result.is_ok()
}

pub fn set_enabled(enabled: bool) {
    let mut key = HKEY::default();
    unsafe {
        if RegOpenKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, KEY_SET_VALUE, &mut key).is_err() {
            return;
        }
        match enabled.then(|| std::env::current_exe().ok()).flatten() {
            Some(exe) => {
                let command = HSTRING::from(format!("\"{}\"", exe.display()));
                // The stored size counts the terminating null.
                let bytes = std::slice::from_raw_parts(
                    command.as_ptr() as *const u8,
                    (command.len() + 1) * 2,
                );
                let _ = RegSetValueExW(key, VALUE, None, REG_SZ, Some(bytes));
            }
            None => {
                let _ = RegDeleteValueW(key, VALUE);
            }
        }
        let _ = RegCloseKey(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_a_shortcut_file_windows_recognises() {
        let target = std::env::current_exe().unwrap();
        let link = std::env::temp_dir().join("claude-usage-widget-test.lnk");
        write_shortcut(&target, &link).unwrap();
        let bytes = std::fs::read(&link).unwrap();
        assert_eq!(bytes[..4], [0x4c, 0, 0, 0]);
    }

    #[test]
    fn the_entry_goes_in_the_users_start_menu() {
        let path = menu_entry_path().unwrap();
        assert!(path.ends_with(Path::new(MENU_FOLDER).join(MENU_ENTRY)));
    }
}
