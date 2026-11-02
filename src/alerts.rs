//! Deciding when a limit deserves a notification, once per window and level.

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;
use crate::usage::Limit;

const CRITICAL_PERCENT: f64 = 95.0;
/// The app reports the same reset some milliseconds apart between readings.
const NEW_WINDOW_GAP_MS: i64 = 60 * 60 * 1000;

/// How far a limit has already been announced, kept across restarts so a
/// crossing is not announced twice.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mark {
    level: u8,
    resets_at: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Near,
    Critical,
    Reset,
}

fn level(percent: f64, warn_at: f64) -> u8 {
    match percent {
        p if p >= CRITICAL_PERCENT => 2,
        p if p >= warn_at => 1,
        _ => 0,
    }
}

pub fn check(limit: &Limit, mark: Mark, warn_at: f64) -> (Mark, Option<Event>) {
    let current = level(limit.percent, warn_at);
    let new_window = match (mark.resets_at, limit.resets_at) {
        (Some(known), Some(reported)) => reported - known > NEW_WINDOW_GAP_MS,
        _ => false,
    };
    let event = if new_window && mark.level > 0 {
        Some(Event::Reset)
    } else if current > mark.level || (new_window && current > 0) {
        Some(if current == 2 {
            Event::Critical
        } else {
            Event::Near
        })
    } else {
        None
    };
    let resets_at = if new_window || mark.resets_at.is_none() {
        limit.resets_at
    } else {
        mark.resets_at
    };
    let next = Mark {
        level: current,
        resets_at,
    };
    (next, event)
}

/// One notification for everything a reading brought, led by the worst of it.
pub fn message(events: &[(&Limit, Event)], now: i64, lang: Lang) -> Option<(String, String)> {
    let has = |wanted| events.iter().any(|(_, event)| *event == wanted);
    let title = match events.first()? {
        _ if has(Event::Critical) => lang.alert_critical(),
        _ if has(Event::Near) => lang.alert_near(),
        _ => lang.alert_reset(),
    };
    let lines: Vec<String> = events
        .iter()
        .map(|(limit, event)| match event {
            Event::Reset => lang.alert_reset_line(&limit.title),
            _ => lang.alert_line(
                &limit.title,
                &limit.value,
                limit.resets_at.map(|at| at - now),
            ),
        })
        .collect();
    Some((title.to_string(), lines.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::usage::Kind;

    const DAY: i64 = 24 * 60 * 60 * 1000;

    fn limit(percent: f64, resets_at: i64) -> Limit {
        Limit {
            id: "weekly".into(),
            short: "Semanal".into(),
            title: "Semanal, todos los modelos".into(),
            kind: Kind::Weekly,
            percent,
            value: format!("{percent}%"),
            resets_at: Some(resets_at),
            window_ms: Some(7 * DAY),
            cap: None,
        }
    }

    #[test]
    fn nothing_happens_below_the_threshold() {
        let (_, event) = check(&limit(79.0, DAY), Mark::default(), 80.0);
        assert_eq!(event, None);
    }

    #[test]
    fn each_level_is_announced_once() {
        let (mark, event) = check(&limit(82.0, DAY), Mark::default(), 80.0);
        assert_eq!(event, Some(Event::Near));

        let (mark, event) = check(&limit(90.0, DAY), mark, 80.0);
        assert_eq!(event, None);

        let (mark, event) = check(&limit(96.0, DAY), mark, 80.0);
        assert_eq!(event, Some(Event::Critical));

        let (_, event) = check(&limit(99.0, DAY), mark, 80.0);
        assert_eq!(event, None);
    }

    #[test]
    fn the_jitter_in_the_reported_reset_is_not_a_new_window() {
        let (mark, _) = check(&limit(82.0, DAY), Mark::default(), 80.0);
        let (next, event) = check(&limit(82.0, DAY + 800), mark, 80.0);
        assert_eq!(event, None);
        assert_eq!(next, mark);
    }

    #[test]
    fn a_new_window_is_announced_only_after_a_warning() {
        let (quiet, _) = check(&limit(10.0, DAY), Mark::default(), 80.0);
        let (_, event) = check(&limit(0.0, 8 * DAY), quiet, 80.0);
        assert_eq!(event, None);

        let (warned, _) = check(&limit(82.0, DAY), Mark::default(), 80.0);
        let (fresh, event) = check(&limit(0.0, 8 * DAY), warned, 80.0);
        assert_eq!(event, Some(Event::Reset));

        let (_, event) = check(&limit(81.0, 8 * DAY), fresh, 80.0);
        assert_eq!(event, Some(Event::Near));
    }

    #[test]
    fn a_limit_without_a_reset_warns_again_after_falling() {
        let mut extra = limit(85.0, 0);
        extra.resets_at = None;
        let (mark, event) = check(&extra, Mark::default(), 80.0);
        assert_eq!(event, Some(Event::Near));

        extra.percent = 0.0;
        let (mark, event) = check(&extra, mark, 80.0);
        assert_eq!(event, None);

        extra.percent = 85.0;
        let (_, event) = check(&extra, mark, 80.0);
        assert_eq!(event, Some(Event::Near));
    }

    #[test]
    fn the_message_leads_with_the_worst_event() {
        let near = limit(82.0, 2 * DAY);
        let critical = limit(96.0, DAY);
        let events = [(&near, Event::Near), (&critical, Event::Critical)];
        let (title, body) = message(&events, 0, Lang::Es).unwrap();
        assert_eq!(title, Lang::Es.alert_critical());
        assert_eq!(body.lines().count(), 2);
        assert!(body.contains("reinicia en 2d 0h"));
        assert!(message(&[], 0, Lang::Es).is_none());
    }
}
