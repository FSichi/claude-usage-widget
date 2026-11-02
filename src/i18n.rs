//! Every word the widget shows, in Spanish and English.

use serde::{Deserialize, Serialize};
use windows::Win32::Globalization::GetUserDefaultUILanguage;

const SPANISH_PRIMARY_ID: u16 = 0x0a;
const PRIMARY_ID_MASK: u16 = 0x3ff;

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 60 * MINUTE_MS;
const DAY_MS: i64 = 24 * HOUR_MS;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Es,
    En,
}

impl Lang {
    /// The language Windows runs in, until the person picks one.
    pub fn detect() -> Self {
        let id = unsafe { GetUserDefaultUILanguage() };
        if id & PRIMARY_ID_MASK == SPANISH_PRIMARY_ID {
            Lang::Es
        } else {
            Lang::En
        }
    }

    const fn pick(self, es: &'static str, en: &'static str) -> &'static str {
        match self {
            Lang::Es => es,
            Lang::En => en,
        }
    }

    pub const fn weekly(self) -> &'static str {
        self.pick("Semanal", "Weekly")
    }

    pub const fn weekly_all(self) -> &'static str {
        self.pick("Semanal, todos los modelos", "Weekly, all models")
    }

    pub fn weekly_model(self, name: &str) -> String {
        format!("{}, {name}", self.weekly())
    }

    pub const fn session(self) -> &'static str {
        self.pick("5 horas", "5 hours")
    }

    pub const fn session_title(self) -> &'static str {
        self.pick("Sesión de 5 horas", "5-hour session")
    }

    pub const fn extra(self) -> &'static str {
        "Extra"
    }

    pub const fn extra_title(self) -> &'static str {
        self.pick("Uso extra", "Extra usage")
    }

    pub fn of_amount(self, amount: i64) -> String {
        format!("{} ${amount}", self.pick("de", "of"))
    }

    pub fn of_amount_month(self, amount: i64) -> String {
        format!(
            "{} {}",
            self.of_amount(amount),
            self.pick("este mes", "this month")
        )
    }

    pub const fn now(self) -> &'static str {
        self.pick("ahora", "now")
    }

    pub fn resets_in(self, left: &str) -> String {
        format!("{} {left}", self.pick("Reinicia en", "Resets in"))
    }

    pub fn resets_in_with_pace(self, left: &str, pace: i64) -> String {
        format!(
            "{} · {} {pace}%",
            self.resets_in(left),
            self.pick("ritmo esperado", "expected pace")
        )
    }

    pub fn left(self, ms: i64) -> String {
        if ms <= 0 {
            return self.now().to_string();
        }
        let minutes = ms / MINUTE_MS;
        match (minutes / (24 * 60), minutes / 60 % 24, minutes % 60) {
            (0, 0, m) => format!("{}m", m.max(1)),
            (0, h, m) => format!("{h}h {m}m"),
            (d, h, _) => format!("{d}d {h}h"),
        }
    }

    pub fn updated(self, age_ms: i64) -> String {
        let (amount, unit) = match age_ms {
            ms if ms < MINUTE_MS => ((ms / 1000).max(0), "s"),
            ms if ms < HOUR_MS => (ms / MINUTE_MS, "min"),
            ms if ms < DAY_MS => (ms / HOUR_MS, "h"),
            ms => (ms / DAY_MS, "d"),
        };
        match self {
            Lang::Es => format!("Actualizado hace {amount} {unit}"),
            Lang::En => format!("Updated {amount} {unit} ago"),
        }
    }

    pub const fn panel_title(self) -> &'static str {
        self.pick("Uso de Claude", "Claude usage")
    }

    pub const fn settings_title(self) -> &'static str {
        self.pick("Ajustes", "Settings")
    }

    pub const fn waiting(self) -> &'static str {
        self.pick("Esperando datos", "Waiting for data")
    }

    pub const fn no_data(self) -> &'static str {
        self.pick("Sin datos del plan", "No plan data")
    }

    pub const fn no_session_limit(self) -> &'static str {
        self.pick(
            "Tu plan no informa la sesión de 5 horas",
            "Your plan doesn't report a 5-hour session",
        )
    }

    pub const fn theme(self) -> &'static str {
        self.pick("Tema", "Theme")
    }

    pub const fn language(self) -> &'static str {
        self.pick("Idioma", "Language")
    }

    pub const fn start_with_windows(self) -> &'static str {
        self.pick("Iniciar con Windows", "Start with Windows")
    }

    pub const fn quit(self) -> &'static str {
        self.pick("Salir", "Quit")
    }

    pub const fn notice_first_run(self) -> &'static str {
        self.pick(
            "Abrí una sesión de Claude Code para empezar.",
            "Open a Claude Code session to get started.",
        )
    }

    pub const fn notice_stale(self) -> &'static str {
        self.pick(
            "Sin datos nuevos: no hay una sesión activa.",
            "No new data: no active session.",
        )
    }

    pub const fn notice_failed(self) -> &'static str {
        self.pick(
            "La app no pudo informar tu consumo.",
            "The app couldn't report your usage.",
        )
    }

    pub const fn notice_no_limits(self) -> &'static str {
        self.pick(
            "Esta cuenta no tiene límites de plan.",
            "This account has no plan limits.",
        )
    }

    pub const fn notice_unavailable(self) -> &'static str {
        self.pick(
            "La app no pudo leer tus límites por ahora.",
            "The app can't read your limits right now.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_left_reads_the_same_shape_in_both_languages() {
        let ms = 4 * DAY_MS + 13 * HOUR_MS + 5;
        assert_eq!(Lang::Es.left(ms), "4d 13h");
        assert_eq!(Lang::En.left(ms), "4d 13h");
        assert_eq!(Lang::Es.left(3 * HOUR_MS + 20 * MINUTE_MS), "3h 20m");
        assert_eq!(Lang::Es.left(30_000), "1m");
    }

    #[test]
    fn a_passed_reset_says_now() {
        assert_eq!(Lang::Es.left(-1), "ahora");
        assert_eq!(Lang::En.left(0), "now");
    }

    #[test]
    fn ages_follow_the_language() {
        assert_eq!(Lang::Es.updated(40_000), "Actualizado hace 40 s");
        assert_eq!(Lang::En.updated(3 * MINUTE_MS), "Updated 3 min ago");
        assert_eq!(Lang::En.updated(2 * DAY_MS), "Updated 2 d ago");
    }

    #[test]
    fn composed_phrases_read_right() {
        assert_eq!(
            Lang::Es.resets_in_with_pace("4d 13h", 35),
            "Reinicia en 4d 13h · ritmo esperado 35%"
        );
        assert_eq!(Lang::En.of_amount_month(50), "of $50 this month");
        assert_eq!(Lang::En.weekly_model("Fable"), "Weekly, Fable");
    }

    #[test]
    fn the_language_round_trips_in_lowercase() {
        assert_eq!(serde_json::to_string(&Lang::Es).unwrap(), "\"es\"");
        let back: Lang = serde_json::from_str("\"en\"").unwrap();
        assert_eq!(back, Lang::En);
    }
}
