//! Every word the widget shows, in Spanish and English.

use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::{FILETIME, SYSTEMTIME};
use windows::Win32::Globalization::GetUserDefaultUILanguage;
use windows::Win32::System::Time::{FileTimeToSystemTime, SystemTimeToTzSpecificLocalTime};

const SPANISH_PRIMARY_ID: u16 = 0x0a;
const PRIMARY_ID_MASK: u16 = 0x3ff;

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 60 * MINUTE_MS;
const DAY_MS: i64 = 24 * HOUR_MS;
/// From the Windows epoch, 1601, to the Unix one.
const UNIX_EPOCH_IN_FILETIME_MS: i64 = 11_644_473_600_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    Es,
    En,
}

/// A moment on this machine's clock and in its time zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalTime {
    pub date: (u16, u16, u16),
    pub weekday: u16,
    pub hour: u16,
    pub minute: u16,
}

impl LocalTime {
    pub fn from_ms(ms: i64) -> Option<Self> {
        let ticks = ((ms + UNIX_EPOCH_IN_FILETIME_MS) * 10_000) as u64;
        let file_time = FILETIME {
            dwLowDateTime: ticks as u32,
            dwHighDateTime: (ticks >> 32) as u32,
        };
        let mut utc = SYSTEMTIME::default();
        let mut local = SYSTEMTIME::default();
        unsafe {
            FileTimeToSystemTime(&file_time, &mut utc).ok()?;
            SystemTimeToTzSpecificLocalTime(None, &utc, &mut local).ok()?;
        }
        Some(Self {
            date: (local.wYear, local.wMonth, local.wDay),
            weekday: local.wDayOfWeek,
            hour: local.wHour,
            minute: local.wMinute,
        })
    }
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

    pub fn on_pace_for(self, percent: i64) -> String {
        format!(
            "{} {percent}%",
            self.pick("A este ritmo llegás al", "At this pace you'll reach")
        )
    }

    pub fn runs_out(self, at: LocalTime, now: LocalTime) -> String {
        format!(
            "{} {}",
            self.pick(
                "A este ritmo llegás al límite",
                "At this pace you'll hit the limit"
            ),
            self.moment(at, now)
        )
    }

    fn moment(self, at: LocalTime, now: LocalTime) -> String {
        let clock = format!("{:02}:{:02}", at.hour, at.minute);
        if at.date == now.date {
            return format!("{} {clock}", self.pick("hoy a las", "today at"));
        }
        match self {
            Lang::Es => format!("el {} a las {clock}", self.weekday(at.weekday)),
            Lang::En => format!("on {} at {clock}", self.weekday(at.weekday)),
        }
    }

    const fn weekday(self, day: u16) -> &'static str {
        match day {
            0 => self.pick("domingo", "Sunday"),
            1 => self.pick("lunes", "Monday"),
            2 => self.pick("martes", "Tuesday"),
            3 => self.pick("miércoles", "Wednesday"),
            4 => self.pick("jueves", "Thursday"),
            5 => self.pick("viernes", "Friday"),
            _ => self.pick("sábado", "Saturday"),
        }
    }

    pub const fn alert_near(self) -> &'static str {
        self.pick("Te acercás a un límite", "You're nearing a limit")
    }

    pub const fn alert_critical(self) -> &'static str {
        self.pick(
            "Estás por llegar a un límite",
            "You're about to hit a limit",
        )
    }

    pub const fn alert_reset(self) -> &'static str {
        self.pick("Se reinició un límite", "A limit has reset")
    }

    pub fn alert_line(self, title: &str, value: &str, left_ms: Option<i64>) -> String {
        match left_ms {
            Some(left) => format!(
                "{title}: {value}, {} {}",
                self.pick("reinicia en", "resets in"),
                self.left(left)
            ),
            None => format!("{title}: {value}"),
        }
    }

    pub fn alert_reset_line(self, title: &str) -> String {
        format!("{title} {}", self.pick("volvió a cero", "is back to zero"))
    }

    pub const fn alerts(self) -> &'static str {
        self.pick("Alertas", "Alerts")
    }

    pub const fn alert_at(self) -> &'static str {
        self.pick("Avisarme al", "Notify me at")
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

    pub const fn notice_no_claude(self) -> &'static str {
        self.pick(
            "Instalá la app de Claude para ver tu consumo.",
            "Install the Claude app to see your usage.",
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

    fn local(day: u16, weekday: u16) -> LocalTime {
        LocalTime {
            date: (2026, 11, day),
            weekday,
            hour: 9,
            minute: 5,
        }
    }

    #[test]
    fn running_out_says_when_in_plain_words() {
        let now = local(2, 1);
        assert_eq!(
            Lang::Es.runs_out(local(2, 1), now),
            "A este ritmo llegás al límite hoy a las 09:05"
        );
        assert_eq!(
            Lang::Es.runs_out(local(5, 4), now),
            "A este ritmo llegás al límite el jueves a las 09:05"
        );
        assert_eq!(
            Lang::En.runs_out(local(8, 0), now),
            "At this pace you'll hit the limit on Sunday at 09:05"
        );
    }

    #[test]
    fn local_time_reads_a_known_moment() {
        let time = LocalTime::from_ms(1_793_988_000_000).unwrap();
        assert_eq!(time.date.0, 2026);
        assert_eq!(time.date.1, 11);
        assert!(time.weekday <= 6);
    }

    #[test]
    fn alert_lines_mention_the_reset_when_there_is_one() {
        assert_eq!(
            Lang::Es.alert_line("Fable", "82%", Some(3 * DAY_MS)),
            "Fable: 82%, reinicia en 3d 0h"
        );
        assert_eq!(
            Lang::En.alert_line("Extra usage", "$40", None),
            "Extra usage: $40"
        );
    }

    #[test]
    fn the_language_round_trips_in_lowercase() {
        assert_eq!(serde_json::to_string(&Lang::Es).unwrap(), "\"es\"");
        let back: Lang = serde_json::from_str("\"en\"").unwrap();
        assert_eq!(back, Lang::En);
    }
}
