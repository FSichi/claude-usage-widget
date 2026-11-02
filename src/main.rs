#![cfg_attr(not(test), windows_subsystem = "windows")]

mod alerts;
mod app;
mod bridge;
mod capsule;
mod gfx;
mod i18n;
mod notify;
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
    // Whether notifications reach this PC can only be seen by sending one.
    if std::env::args().any(|arg| arg == "--test-alert") {
        notify::register();
        let sent = notify::send("Claude Usage Widget", "Las alertas funcionan · Alerts work");
        if let Err(error) = sent {
            settings::log(&format!("test alert failed: {error}"));
        }
        return;
    }
    // Held for the life of the process: a second copy sees it and leaves.
    let _instance = unsafe { CreateMutexW(None, true, w!("Local\\ClaudeUsageWidget")) };
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return;
    }
    if let Err(error) = app::run() {
        eprintln!("claude-usage-widget: {error}");
    }
}
