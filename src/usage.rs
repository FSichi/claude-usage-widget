//! The file the bridge writes, and the limits the UI draws from it.

use serde::Deserialize;

use crate::i18n::Lang;

const MINUTE_MS: i64 = 60_000;
const HOUR_MS: i64 = 60 * MINUTE_MS;
const DAY_MS: i64 = 24 * HOUR_MS;

/// The source refreshes every minute; past this it stopped reporting.
pub const STALE_AFTER_MS: i64 = 5 * MINUTE_MS;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub updated_at: i64,
    pub plan: Option<Plan>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub status: String,
    pub plan: Option<String>,
    #[serde(default)]
    pub windows: Vec<PlanWindow>,
    pub extra_usage: Option<ExtraUsage>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanWindow {
    pub label: String,
    pub percent_used: f64,
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExtraUsage {
    #[serde(default)]
    pub enabled: bool,
    pub spent: Option<String>,
    pub monthly_limit: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Session,
    Weekly,
    Model,
    Extra,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Normal,
    High,
    Critical,
}

/// One row the UI can show or hide, whatever the plan reports.
#[derive(Debug, Clone, PartialEq)]
pub struct Limit {
    pub id: String,
    pub short: String,
    pub title: String,
    pub kind: Kind,
    pub percent: f64,
    pub value: String,
    pub resets_at: Option<i64>,
    pub window_ms: Option<i64>,
    /// The monthly cap of extra usage, in dollars.
    pub cap: Option<i64>,
}

impl Limit {
    pub fn severity(&self) -> Severity {
        match self.percent {
            p if p >= 95.0 => Severity::Critical,
            p if p >= 80.0 => Severity::High,
            _ => Severity::Normal,
        }
    }

    /// How far through its window the limit is: the percent a steady pace
    /// would have used by now.
    pub fn pace(&self, now: i64) -> Option<f64> {
        let window = self.window_ms?;
        let left = (self.resets_at? - now).clamp(0, window);
        Some(100.0 * (window - left) as f64 / window as f64)
    }

    /// What goes after the percent: the countdown, or the extra usage cap.
    pub fn trailing(&self, now: i64, lang: Lang) -> String {
        match (self.cap, self.resets_at) {
            (Some(cap), _) => lang.of_amount(cap),
            (None, Some(at)) => lang.left(at - now),
            (None, None) => String::new(),
        }
    }
}

pub fn parse(text: &str) -> Option<Snapshot> {
    serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()
}

pub fn limits(plan: &Plan, lang: Lang) -> Vec<Limit> {
    let mut out: Vec<Limit> = plan
        .windows
        .iter()
        .map(|window| limit_from_window(window, lang))
        .collect();
    if let Some(extra) = plan.extra_usage.as_ref().filter(|e| e.enabled) {
        out.push(limit_from_extra(extra, lang));
    }
    out
}

fn limit_from_window(window: &PlanWindow, lang: Lang) -> Limit {
    let (id, short, title, kind, window_ms) = classify(&window.label, lang);
    Limit {
        id,
        short,
        title,
        kind,
        percent: window.percent_used.max(0.0),
        value: format!("{}%", window.percent_used.round() as i64),
        resets_at: window.resets_at.as_deref().and_then(parse_iso_ms),
        window_ms,
        cap: None,
    }
}

fn limit_from_extra(extra: &ExtraUsage, lang: Lang) -> Limit {
    let spent = amount(extra.spent.as_deref());
    let limit = amount(extra.monthly_limit.as_deref());
    let percent = if limit > 0.0 {
        100.0 * spent / limit
    } else {
        0.0
    };
    Limit {
        id: "extra".into(),
        short: lang.extra().into(),
        title: lang.extra_title().into(),
        kind: Kind::Extra,
        percent,
        value: format!("${}", spent.round() as i64),
        resets_at: None,
        window_ms: None,
        cap: Some(limit.round() as i64),
    }
}

fn amount(text: Option<&str>) -> f64 {
    text.and_then(|t| t.parse().ok()).unwrap_or(0.0)
}

fn classify(label: &str, lang: Lang) -> (String, String, String, Kind, Option<i64>) {
    let lower = label.to_lowercase();
    if lower.contains("5-hour") || lower.contains("5 hour") || lower.contains("five") {
        return (
            "session".into(),
            lang.session().into(),
            lang.session_title().into(),
            Kind::Session,
            Some(5 * HOUR_MS),
        );
    }
    if lower.starts_with("weekly") {
        let name = label.split('·').nth(1).unwrap_or("").trim();
        if name.is_empty() || name.eq_ignore_ascii_case("all models") {
            return (
                "weekly".into(),
                lang.weekly().into(),
                lang.weekly_all().into(),
                Kind::Weekly,
                Some(7 * DAY_MS),
            );
        }
        return (
            format!("weekly:{}", name.to_lowercase()),
            name.to_string(),
            lang.weekly_model(name),
            Kind::Model,
            Some(7 * DAY_MS),
        );
    }
    (
        lower,
        label.to_string(),
        label.to_string(),
        Kind::Other,
        None,
    )
}

/// `2026-11-06T18:00:00.814Z`, the only form the app reports, to epoch ms.
fn parse_iso_ms(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    if bytes.len() < 20 || bytes[4] != b'-' || bytes[10] != b'T' || !text.ends_with('Z') {
        return None;
    }
    let field = |range: std::ops::Range<usize>| text.get(range)?.parse::<i64>().ok();
    let (year, month, day) = (field(0..4)?, field(5..7)?, field(8..10)?);
    let (hour, minute, second) = (field(11..13)?, field(14..16)?, field(17..19)?);
    let millis = match bytes[19] {
        b'.' => field(20..23).unwrap_or(0),
        _ => 0,
    };
    let days = days_from_civil(year, month, day);
    Some((days * 86_400 + hour * 3600 + minute * 60 + second) * 1000 + millis)
}

/// Days since 1970-01-01 for a proleptic Gregorian date.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    const ULTRA: &str = r#"{
      "updatedAt": 1793594280402,
      "plan": {
        "status": "ok",
        "plan": "Ultra",
        "windows": [
          {"label": "Weekly · all models", "percentUsed": 7, "resetsAt": "2026-11-06T18:00:00.814Z", "resetsIn": "4d 13h"},
          {"label": "Weekly · Fable", "percentUsed": 0, "resetsAt": "2026-11-06T18:00:00.000Z", "resetsIn": "4d 13h"}
        ],
        "extraUsage": {"enabled": true, "spent": "12.50", "monthlyLimit": "50.00", "currency": "USD"}
      }
    }"#;

    fn plan() -> Plan {
        parse(ULTRA).unwrap().plan.unwrap()
    }

    #[test]
    fn reads_the_plan_snapshot() {
        let found = limits(&plan(), Lang::Es);
        let ids: Vec<_> = found.iter().map(|l| l.id.as_str()).collect();
        assert_eq!(ids, ["weekly", "weekly:fable", "extra"]);
        assert_eq!(found[0].short, "Semanal");
        assert_eq!(found[1].short, "Fable");
        assert_eq!(found[1].kind, Kind::Model);
        assert_eq!(found[0].value, "7%");
    }

    #[test]
    fn labels_follow_the_language_but_ids_do_not() {
        let es = limits(&plan(), Lang::Es);
        let en = limits(&plan(), Lang::En);
        assert_eq!(en[0].short, "Weekly");
        assert_eq!(en[0].title, "Weekly, all models");
        assert_eq!(en[2].title, "Extra usage");
        let ids = |found: &[Limit]| found.iter().map(|l| l.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&es), ids(&en));
    }

    #[test]
    fn a_five_hour_window_appears_only_when_reported() {
        let mut plan = plan();
        assert!(limits(&plan, Lang::Es)
            .iter()
            .all(|l| l.kind != Kind::Session));
        plan.windows.insert(
            0,
            PlanWindow {
                label: "5-hour".into(),
                percent_used: 41.2,
                resets_at: None,
            },
        );
        let found = limits(&plan, Lang::Es);
        assert_eq!(found[0].id, "session");
        assert_eq!(found[0].value, "41%");
        assert_eq!(found[0].window_ms, Some(5 * HOUR_MS));
    }

    #[test]
    fn extra_usage_reads_as_money_and_needs_enabling() {
        let mut plan = plan();
        let extra = limits(&plan, Lang::Es).pop().unwrap();
        assert_eq!(extra.value, "$13");
        assert_eq!(extra.trailing(0, Lang::Es), "de $50");
        assert_eq!(extra.trailing(0, Lang::En), "of $50");
        assert!((extra.percent - 25.0).abs() < 1e-9);

        plan.extra_usage.as_mut().unwrap().enabled = false;
        assert!(limits(&plan, Lang::Es).iter().all(|l| l.id != "extra"));
    }

    #[test]
    fn an_unknown_limit_still_shows() {
        let found = limit_from_window(
            &PlanWindow {
                label: "Monthly credits".into(),
                percent_used: 3.0,
                resets_at: None,
            },
            Lang::En,
        );
        assert_eq!(found.kind, Kind::Other);
        assert_eq!(found.short, "Monthly credits");
    }

    #[test]
    fn parses_the_timestamp_the_app_reports() {
        assert_eq!(parse_iso_ms("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(
            parse_iso_ms("2026-11-06T18:00:00.814Z"),
            Some(1_793_988_000_814)
        );
        assert_eq!(parse_iso_ms("not a date"), None);
    }

    #[test]
    fn pace_follows_the_window() {
        let limit = &limits(&plan(), Lang::Es)[0];
        let reset = limit.resets_at.unwrap();
        assert_eq!(limit.pace(reset), Some(100.0));
        assert_eq!(limit.pace(reset - 7 * DAY_MS), Some(0.0));
        assert_eq!(limit.pace(reset - 8 * DAY_MS), Some(0.0));
        let half = limit.pace(reset - 7 * DAY_MS / 2).unwrap();
        assert!((half - 50.0).abs() < 1e-9);
    }

    #[test]
    fn severity_marks_the_last_stretch() {
        let mut limit = limits(&plan(), Lang::Es).remove(0);
        assert_eq!(limit.severity(), Severity::Normal);
        limit.percent = 80.0;
        assert_eq!(limit.severity(), Severity::High);
        limit.percent = 95.0;
        assert_eq!(limit.severity(), Severity::Critical);
    }

    #[test]
    fn tolerates_an_error_snapshot_and_a_bom() {
        let snapshot = parse("\u{feff}{\"updatedAt\": 5, \"error\": \"nope\"}").unwrap();
        assert_eq!(snapshot.error.as_deref(), Some("nope"));
        assert!(snapshot.plan.is_none());
        assert!(parse("{ half a fi").is_none());
    }
}
