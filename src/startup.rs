//! The "start with Windows" entry, under the user's own Run key.

use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegGetValueW, RegOpenKeyExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RRF_RT_REG_SZ,
};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE: PCWSTR = w!("ClaudeUsageWidget");

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
