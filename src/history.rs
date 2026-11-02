//! What each day used of the weekly limit, kept across runs.
//!
//! The plan only says where a limit stands now, so a day's share is the rise
//! between readings. Use from a stretch with no session lands on the day the
//! next reading comes in.

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::settings::data_dir;

pub const CHART_DAYS: usize = 14;

const WEEK: i64 = 7;
const KEEP_DAYS: i64 = 35;
/// The app reports the same reset some milliseconds apart between readings.
const NEW_WINDOW_GAP_MS: i64 = 60 * 60 * 1000;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct Day {
    day: i64,
    used: f64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct History {
    days: Vec<Day>,
    percent: Option<f64>,
    resets_at: Option<i64>,
}

fn path() -> PathBuf {
    data_dir().join("history.json")
}

impl History {
    pub fn load() -> Self {
        fs::read_to_string(path())
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path(), text);
        }
    }

    /// Takes a reading of the weekly limit, and answers whether anything
    /// worth saving changed. The first reading only sets the starting point:
    /// what came before it belongs to days the widget never saw.
    pub fn record(&mut self, day: i64, percent: f64, resets_at: Option<i64>) -> bool {
        let new_window = match (self.resets_at, resets_at) {
            (Some(known), Some(reported)) => reported - known > NEW_WINDOW_GAP_MS,
            _ => false,
        };
        let gained = match self.percent {
            None => 0.0,
            Some(_) if new_window => percent,
            Some(last) => (percent - last).max(0.0),
        };
        let is_new_day = self.days.last().is_none_or(|last| last.day != day);
        let changed = is_new_day || self.percent != Some(percent);

        if new_window || self.resets_at.is_none() {
            self.resets_at = resets_at;
        }
        self.percent = Some(percent);
        if is_new_day {
            self.days.push(Day { day, used: gained });
        } else if let Some(today) = self.days.last_mut() {
            today.used += gained;
        }
        self.days.retain(|entry| entry.day > day - KEEP_DAYS);
        changed
    }

    fn used_on(&self, day: i64) -> f64 {
        self.days
            .iter()
            .find(|entry| entry.day == day)
            .map_or(0.0, |entry| entry.used)
    }

    /// The last `CHART_DAYS` days, oldest first, ending today.
    pub fn recent(&self, today: i64) -> [f64; CHART_DAYS] {
        std::array::from_fn(|index| self.used_on(today - (CHART_DAYS - 1 - index) as i64))
    }

    fn total(&self, from: i64, to: i64) -> f64 {
        (from..=to).map(|day| self.used_on(day)).sum()
    }

    /// The last seven days, and the seven before them if the widget saw them.
    pub fn week_totals(&self, today: i64) -> (f64, Option<f64>) {
        let current = self.total(today - WEEK + 1, today);
        let boundary = today - WEEK;
        let saw_previous = self.days.iter().any(|entry| entry.day <= boundary);
        let previous = saw_previous.then(|| self.total(boundary - WEEK + 1, boundary));
        (current, previous)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY_MS: i64 = 24 * 60 * 60 * 1000;

    #[test]
    fn the_first_reading_is_only_a_starting_point() {
        let mut history = History::default();
        assert!(history.record(100, 40.0, Some(DAY_MS)));
        assert_eq!(history.used_on(100), 0.0);

        history.record(100, 43.0, Some(DAY_MS));
        assert_eq!(history.used_on(100), 3.0);
    }

    #[test]
    fn each_day_gets_only_its_own_rise() {
        let mut history = History::default();
        history.record(100, 10.0, Some(DAY_MS));
        history.record(100, 12.0, Some(DAY_MS));
        history.record(101, 15.0, Some(DAY_MS));
        history.record(101, 20.0, Some(DAY_MS));

        assert_eq!(history.used_on(100), 2.0);
        assert_eq!(history.used_on(101), 8.0);
    }

    #[test]
    fn a_new_window_counts_from_zero() {
        let mut history = History::default();
        history.record(100, 60.0, Some(DAY_MS));
        history.record(101, 4.0, Some(8 * DAY_MS));
        assert_eq!(history.used_on(101), 4.0);
    }

    #[test]
    fn the_jitter_in_the_reported_reset_is_not_a_new_window() {
        let mut history = History::default();
        history.record(100, 60.0, Some(DAY_MS));
        history.record(100, 61.0, Some(DAY_MS + 800));
        assert_eq!(history.used_on(100), 1.0);
    }

    #[test]
    fn the_same_reading_is_not_worth_saving_again() {
        let mut history = History::default();
        history.record(100, 10.0, Some(DAY_MS));
        assert!(!history.record(100, 10.0, Some(DAY_MS)));
        assert!(history.record(100, 11.0, Some(DAY_MS)));
        assert!(history.record(101, 11.0, Some(DAY_MS)));
    }

    #[test]
    fn the_chart_ends_today_and_shows_gaps_as_nothing() {
        let mut history = History::default();
        history.record(100, 10.0, Some(DAY_MS));
        history.record(100, 12.0, Some(DAY_MS));
        history.record(103, 13.0, Some(DAY_MS));

        let recent = history.recent(103);
        assert_eq!(recent[CHART_DAYS - 1], 1.0);
        assert_eq!(recent[CHART_DAYS - 2], 0.0);
        assert_eq!(recent[CHART_DAYS - 4], 2.0);
    }

    #[test]
    fn totals_compare_with_the_previous_week_once_it_was_seen() {
        let mut history = History::default();
        history.record(100, 10.0, Some(DAY_MS));
        history.record(100, 15.0, Some(DAY_MS));
        assert_eq!(history.week_totals(100), (5.0, None));

        history.record(107, 20.0, Some(DAY_MS));
        assert_eq!(history.week_totals(107), (5.0, Some(5.0)));
    }

    #[test]
    fn old_days_are_let_go() {
        let mut history = History::default();
        history.record(100, 10.0, Some(DAY_MS));
        history.record(100 + KEEP_DAYS, 10.0, Some(DAY_MS));
        assert_eq!(history.days.len(), 1);
    }
}
