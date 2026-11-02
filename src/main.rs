#![cfg_attr(not(test), windows_subsystem = "windows")]

mod app;
mod bridge;
mod capsule;
mod gfx;
mod i18n;
mod panel;
mod settings;
mod startup;
mod taskbar;
mod theme;
mod usage;

use windows::core::w;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;

fn main() {
    // Held for the life of the process: a second copy sees it and leaves.
    let _instance = unsafe { CreateMutexW(None, true, w!("Local\\ClaudeUsageWidget")) };
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return;
    }
    if let Err(error) = app::run() {
        eprintln!("claude-usage-widget: {error}");
    }
}
