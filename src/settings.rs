//! What the person chose: limits, position, theme and language.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;
use crate::theme::AUTO;
use crate::usage::{Kind, Limit};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Only the limits the person toggled; the rest follow `shown_by_default`.
    pub visible: BTreeMap<String, bool>,
    /// Distance from the taskbar's left edge, in physical pixels.
    pub offset_x: Option<i32>,
    pub theme: String,
    /// `None` follows the language Windows runs in.
    pub lang: Option<Lang>,
    /// Set after the first run, which turns on starting with Windows.
    pub initialized: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            visible: BTreeMap::new(),
            offset_x: None,
            theme: AUTO.to_string(),
            lang: None,
            initialized: false,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read_to_string(settings_path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }

    pub fn lang(&self) -> Lang {
        self.lang.unwrap_or_else(Lang::detect)
    }

    pub fn is_visible(&self, limit: &Limit) -> bool {
        self.visible
            .get(&limit.id)
            .copied()
            .unwrap_or(shown_by_default(limit))
    }

    pub fn toggle(&mut self, limit: &Limit) {
        let next = !self.is_visible(limit);
        self.visible.insert(limit.id.clone(), next);
    }
}

fn shown_by_default(limit: &Limit) -> bool {
    limit.kind != Kind::Extra
}

/// In the home folder on purpose. Windows redirects what the Store build of
/// the Claude app writes under AppData to a folder private to that app, so a
/// file the mod left there would never reach the widget.
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("USERPROFILE").unwrap_or_default();
    PathBuf::from(base).join(".claude-usage-widget")
}

pub fn usage_path() -> PathBuf {
    data_dir().join("usage.json")
}

fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit(id: &str, kind: Kind) -> Limit {
        Limit {
            id: id.into(),
            short: id.into(),
            title: id.into(),
            kind,
            percent: 0.0,
            value: "0%".into(),
            resets_at: None,
            window_ms: None,
            cap: None,
        }
    }

    #[test]
    fn new_limits_show_and_extra_usage_starts_hidden() {
        let settings = Settings::default();
        assert!(settings.is_visible(&limit("weekly:opus", Kind::Model)));
        assert!(!settings.is_visible(&limit("extra", Kind::Extra)));
    }

    #[test]
    fn a_toggle_overrides_the_default_and_round_trips() {
        let mut settings = Settings::default();
        settings.toggle(&limit("extra", Kind::Extra));
        settings.toggle(&limit("weekly", Kind::Weekly));
        settings.lang = Some(Lang::En);

        let text = serde_json::to_string(&settings).unwrap();
        let back: Settings = serde_json::from_str(&text).unwrap();
        assert!(back.is_visible(&limit("extra", Kind::Extra)));
        assert!(!back.is_visible(&limit("weekly", Kind::Weekly)));
        assert_eq!(back.lang(), Lang::En);
    }

    #[test]
    fn a_file_from_an_older_version_still_loads() {
        let back: Settings = serde_json::from_str(r#"{"offset_x": 5}"#).unwrap();
        assert_eq!(back.offset_x, Some(5));
        assert_eq!(back.theme, AUTO);
        assert!(back.lang.is_none());
    }
}
