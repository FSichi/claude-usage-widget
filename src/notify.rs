//! Windows notifications, sent under the widget's own name and logo.

use std::fs;

use windows::core::{w, Result, HSTRING, PCWSTR};
use windows::Data::Xml::Dom::XmlDocument;
use windows::Win32::System::Registry::{RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ};
use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

use crate::settings::{self, data_dir};

const APP_ID: &str = "ClaudeUsageWidget";
const APP_NAME: &str = "Claude Usage Widget";
const APP_KEY: PCWSTR = w!("Software\\Classes\\AppUserModelId\\ClaudeUsageWidget");
const LOGO: &[u8] = include_bytes!("../assets/icon.png");

/// An unpackaged app is only allowed to notify under an identity it registers.
pub fn register() {
    let folder = data_dir();
    let logo = folder.join("icon.png");
    let _ = fs::create_dir_all(&folder);
    let _ = fs::write(&logo, LOGO);
    set_value(w!("DisplayName"), APP_NAME);
    set_value(w!("IconUri"), &logo.to_string_lossy());
}

fn set_value(name: PCWSTR, value: &str) {
    let text = HSTRING::from(value);
    // The stored size counts the terminating null.
    let size = (text.len() + 1) * 2;
    let _ = unsafe {
        RegSetKeyValueW(
            HKEY_CURRENT_USER,
            APP_KEY,
            name,
            REG_SZ.0,
            Some(text.as_ptr().cast()),
            size as u32,
        )
    };
}

/// On its own thread: the UI thread shares input with the taskbar.
pub fn show(title: String, body: String) {
    std::thread::spawn(move || {
        if let Err(error) = send(&title, &body) {
            settings::log(&format!("notification failed: {error}"));
        }
    });
}

pub fn send(title: &str, body: &str) -> Result<()> {
    let document = XmlDocument::new()?;
    document.LoadXml(&HSTRING::from(toast_xml(title, body)))?;
    let toast = ToastNotification::CreateToastNotification(&document)?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(APP_ID))?.Show(&toast)
}

fn toast_xml(title: &str, body: &str) -> String {
    format!(
        "<toast><visual><binding template=\"ToastGeneric\">\
         <text>{}</text><text>{}</text>\
         </binding></visual></toast>",
        escape(title),
        escape(body)
    )
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_cannot_break_the_notification() {
        let xml = toast_xml("A & B", "<1%>");
        assert!(xml.contains("<text>A &amp; B</text>"));
        assert!(xml.contains("<text>&lt;1%&gt;</text>"));
    }
}
