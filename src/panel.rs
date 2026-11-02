//! The panel a click opens: the limits, and a settings page behind the gear.

use windows::core::Result;

use crate::capsule::draw_bar;
use crate::gfx::{Align, Canvas, Color, Font, Rect};
use crate::i18n::Lang;
use crate::settings::Settings;
use crate::theme::{system_palette, Palette, Theme, AUTO, THEMES, WARNING};
use crate::usage::{Kind, Limit};

pub const WIDTH: f32 = 372.0;
/// Gap between the panel and the taskbar.
pub const MARGIN: f32 = 8.0;

const PAD: f32 = 16.0;
const CONTENT_W: f32 = WIDTH - 2.0 * PAD;
const HEADER_H: f32 = 64.0;
const NOTICE_H: f32 = 46.0;
const ROW_H: f32 = 70.0;
const MISSING_H: f32 = 32.0;
const BOTTOM_PAD: f32 = 8.0;

const ICON_SIZE: f32 = 30.0;
const TOGGLE_W: f32 = 34.0;
const TOGGLE_H: f32 = 19.0;

const SECTION_H: f32 = 26.0;
const THEME_COLUMNS: usize = 4;
const THEME_GAP: f32 = 8.0;
const THEME_W: f32 = (CONTENT_W - 3.0 * THEME_GAP) / THEME_COLUMNS as f32;
const THEME_PREVIEW_H: f32 = 42.0;
const THEME_H: f32 = THEME_PREVIEW_H + 22.0;
const OPTION_H: f32 = 46.0;
const SEGMENT_W: f32 = 82.0;
const SEGMENT_H: f32 = 30.0;

const GEAR: &str = "\u{E713}";
const BACK: &str = "\u{E72B}";

const TITLE: Font = Font::new(16.0, 600, Align::Start);
const LABEL: Font = Font::new(14.0, 400, Align::Start);
const VALUE: Font = Font::new(14.0, 600, Align::End);
const SMALL: Font = Font::new(12.0, 400, Align::Start);
const SMALL_CENTER: Font = Font::new(12.0, 400, Align::Center);
const CHIP: Font = Font::new(12.0, 600, Align::Center);
const SEGMENT: Font = Font::new(13.0, 500, Align::Center);
const NOTICE: Font = Font::new(13.0, 400, Align::Start);
const ICON: Font = Font::icon(15.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Usage,
    Settings,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Toggle(String),
    Page(Page),
    Theme(&'static str),
    Lang(Lang),
    Startup,
    Quit,
}

impl Action {
    /// Whole rows light up under the pointer; cards and buttons draw their own.
    fn is_row(&self) -> bool {
        matches!(self, Action::Toggle(_) | Action::Startup | Action::Quit)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub rect: Rect,
    pub action: Action,
}

pub struct View<'a> {
    pub page: Page,
    pub limits: &'a [Limit],
    pub settings: &'a Settings,
    pub lang: Lang,
    pub plan: Option<&'a str>,
    pub age_ms: Option<i64>,
    pub notice: Option<&'a str>,
    pub startup: bool,
    pub now: i64,
}

impl View<'_> {
    fn misses_session(&self) -> bool {
        !self.limits.is_empty() && self.limits.iter().all(|l| l.kind != Kind::Session)
    }

    fn rows_top(&self) -> f32 {
        HEADER_H + if self.notice.is_some() { NOTICE_H } else { 0.0 }
    }

    fn rows_bottom(&self) -> f32 {
        let missing = if self.misses_session() {
            MISSING_H
        } else {
            0.0
        };
        self.rows_top() + self.limits.len() as f32 * ROW_H + missing
    }

    pub fn height(&self) -> f32 {
        let content = match self.page {
            Page::Usage => self.rows_bottom(),
            Page::Settings => settings_bottom(),
        };
        content + BOTTOM_PAD
    }

    pub fn hits(&self) -> Vec<Hit> {
        match self.page {
            Page::Usage => self.usage_hits(),
            Page::Settings => settings_hits(),
        }
    }

    fn usage_hits(&self) -> Vec<Hit> {
        let mut hits: Vec<Hit> = self
            .limits
            .iter()
            .enumerate()
            .map(|(index, limit)| Hit {
                rect: Rect::new(0.0, self.rows_top() + index as f32 * ROW_H, WIDTH, ROW_H),
                action: Action::Toggle(limit.id.clone()),
            })
            .collect();
        hits.push(Hit {
            rect: gear_rect(),
            action: Action::Page(Page::Settings),
        });
        hits
    }
}

fn gear_rect() -> Rect {
    Rect::new(WIDTH - PAD - ICON_SIZE, 13.0, ICON_SIZE, ICON_SIZE)
}

fn back_rect() -> Rect {
    Rect::new(PAD - 6.0, 13.0, ICON_SIZE, ICON_SIZE)
}

fn theme_grid_top() -> f32 {
    HEADER_H + SECTION_H
}

fn theme_rect(index: usize) -> Rect {
    let column = (index % THEME_COLUMNS) as f32;
    let row = (index / THEME_COLUMNS) as f32;
    Rect::new(
        PAD + column * (THEME_W + THEME_GAP),
        theme_grid_top() + row * (THEME_H + THEME_GAP),
        THEME_W,
        THEME_H,
    )
}

fn language_top() -> f32 {
    let rows = THEMES.len().div_ceil(THEME_COLUMNS) as f32;
    theme_grid_top() + rows * (THEME_H + THEME_GAP)
}

fn segment_rect(index: usize) -> Rect {
    let x = WIDTH - PAD - (2 - index) as f32 * SEGMENT_W;
    let y = language_top() + (OPTION_H - SEGMENT_H) / 2.0;
    Rect::new(x, y, SEGMENT_W, SEGMENT_H)
}

fn settings_bottom() -> f32 {
    language_top() + 3.0 * OPTION_H
}

fn settings_hits() -> Vec<Hit> {
    let mut hits = vec![Hit {
        rect: back_rect(),
        action: Action::Page(Page::Usage),
    }];
    hits.extend(THEMES.iter().enumerate().map(|(index, theme)| Hit {
        rect: theme_rect(index),
        action: Action::Theme(theme.id),
    }));
    for (index, lang) in [Lang::Es, Lang::En].into_iter().enumerate() {
        hits.push(Hit {
            rect: segment_rect(index),
            action: Action::Lang(lang),
        });
    }
    let options = language_top() + OPTION_H;
    hits.push(Hit {
        rect: Rect::new(0.0, options, WIDTH, OPTION_H),
        action: Action::Startup,
    });
    hits.push(Hit {
        rect: Rect::new(0.0, options + OPTION_H, WIDTH, OPTION_H),
        action: Action::Quit,
    });
    hits
}

pub fn draw(
    canvas: &Canvas,
    view: &View,
    hits: &[Hit],
    hover: Option<usize>,
    palette: &Palette,
) -> Result<()> {
    let frame = Rect::new(0.5, 0.5, WIDTH - 1.0, view.height() - 1.0);
    canvas.fill(frame, 12.0, palette.panel)?;
    canvas.stroke(frame, 12.0, palette.panel_border, 1.0)?;

    let hovered = hover.and_then(|index| hits.get(index));
    match hovered {
        Some(hit) if hit.action.is_row() => {
            let rect = Rect::new(
                hit.rect.x + 6.0,
                hit.rect.y + 2.0,
                hit.rect.w - 12.0,
                hit.rect.h - 4.0,
            );
            canvas.fill(rect, 6.0, palette.hover)?;
        }
        Some(hit) if matches!(hit.action, Action::Page(_)) => {
            canvas.fill(hit.rect, 6.0, palette.hover)?;
        }
        _ => {}
    }

    match view.page {
        Page::Usage => draw_usage(canvas, view, palette),
        Page::Settings => draw_settings(canvas, view, hovered, palette),
    }
}

fn draw_usage(canvas: &Canvas, view: &View, palette: &Palette) -> Result<()> {
    let lang = view.lang;
    canvas.text(
        lang.panel_title(),
        Rect::new(PAD, 13.0, 220.0, 24.0),
        TITLE,
        palette.text,
    )?;
    let updated = view
        .age_ms
        .map_or(lang.waiting().to_string(), |age| lang.updated(age));
    canvas.text(
        &updated,
        Rect::new(PAD, 37.0, 240.0, 17.0),
        SMALL,
        palette.muted,
    )?;
    if let Some(plan) = view.plan {
        let chip = Rect::new(WIDTH - PAD - ICON_SIZE - 66.0, 17.0, 60.0, 23.0);
        canvas.fill(chip, 11.5, palette.hover)?;
        canvas.text(plan, chip, CHIP, palette.text)?;
    }
    canvas.text(GEAR, gear_rect(), ICON, palette.muted)?;

    if let Some(notice) = view.notice {
        let area = Rect::new(PAD, HEADER_H, CONTENT_W, NOTICE_H - 10.0);
        canvas.fill(area, 6.0, WARNING.alpha(0.14))?;
        let text = Rect::new(PAD + 10.0, HEADER_H, CONTENT_W - 20.0, NOTICE_H - 10.0);
        canvas.text(notice, text, NOTICE, palette.text)?;
    }
    for (index, limit) in view.limits.iter().enumerate() {
        let top = view.rows_top() + index as f32 * ROW_H;
        draw_row(canvas, view, limit, top, palette)?;
    }
    if view.misses_session() {
        let top = view.rows_top() + view.limits.len() as f32 * ROW_H;
        let area = Rect::new(PAD, top, CONTENT_W, MISSING_H);
        canvas.text(lang.no_session_limit(), area, SMALL, palette.muted)?;
    }
    Ok(())
}

fn draw_row(
    canvas: &Canvas,
    view: &View,
    limit: &Limit,
    top: f32,
    palette: &Palette,
) -> Result<()> {
    let visible = view.settings.is_visible(limit);
    let title = Rect::new(PAD, top + 8.0, CONTENT_W - 110.0, 22.0);
    canvas.text(&limit.title, title, LABEL, palette.text)?;

    let toggle_x = WIDTH - PAD - TOGGLE_W;
    let value = Rect::new(toggle_x - 72.0, top + 8.0, 60.0, 22.0);
    canvas.text(&limit.value, value, VALUE, palette.text)?;
    draw_toggle(canvas, toggle_x, top + 9.0, visible, palette)?;

    let bar = Rect::new(PAD, top + 35.0, CONTENT_W - TOGGLE_W - 12.0, 6.0);
    draw_bar(canvas, bar, limit, limit.pace(view.now), palette)?;

    let detail = Rect::new(PAD, top + 46.0, CONTENT_W, 17.0);
    let text = row_detail(limit, view.now, view.lang);
    canvas.text(&text, detail, SMALL, palette.muted)
}

fn row_detail(limit: &Limit, now: i64, lang: Lang) -> String {
    let pace = limit.pace(now).map(|pace| pace.round() as i64);
    match (limit.cap, limit.resets_at, pace) {
        (Some(cap), _, _) => lang.of_amount_month(cap),
        (None, Some(at), Some(pace)) => lang.resets_in_with_pace(&lang.left(at - now), pace),
        (None, Some(at), None) => lang.resets_in(&lang.left(at - now)),
        (None, None, _) => String::new(),
    }
}

fn draw_toggle(canvas: &Canvas, x: f32, y: f32, on: bool, palette: &Palette) -> Result<()> {
    let color = if on {
        palette.accent
    } else {
        palette.toggle_off
    };
    canvas.fill(Rect::new(x, y, TOGGLE_W, TOGGLE_H), TOGGLE_H / 2.0, color)?;
    let knob_x = if on { x + TOGGLE_W - 17.0 } else { x + 2.0 };
    let knob = if on {
        palette.on_accent
    } else {
        Color::hex(0xffffff)
    };
    canvas.fill(Rect::new(knob_x, y + 2.0, 15.0, 15.0), 7.5, knob)
}

fn draw_settings(
    canvas: &Canvas,
    view: &View,
    hovered: Option<&Hit>,
    palette: &Palette,
) -> Result<()> {
    let lang = view.lang;
    canvas.text(BACK, back_rect(), ICON, palette.muted)?;
    canvas.text(
        lang.settings_title(),
        Rect::new(PAD + 30.0, 16.0, 220.0, 24.0),
        TITLE,
        palette.text,
    )?;

    canvas.text(
        lang.theme(),
        Rect::new(PAD, HEADER_H, CONTENT_W, SECTION_H - 6.0),
        SMALL,
        palette.muted,
    )?;
    for (index, theme) in THEMES.iter().enumerate() {
        let rect = theme_rect(index);
        let selected = view.settings.theme == theme.id;
        let hover = hovered.is_some_and(|hit| hit.action == Action::Theme(theme.id));
        draw_theme_card(canvas, theme, rect, selected, hover, lang, palette)?;
    }

    let top = language_top();
    let label = Rect::new(PAD, top, 160.0, OPTION_H);
    canvas.text(lang.language(), label, LABEL, palette.text)?;
    for (index, (choice, name)) in [(Lang::Es, "Español"), (Lang::En, "English")]
        .into_iter()
        .enumerate()
    {
        draw_segment(canvas, segment_rect(index), name, lang == choice, palette)?;
    }

    let options = top + OPTION_H;
    canvas.fill(
        Rect::new(PAD, options, CONTENT_W, 1.0),
        0.0,
        palette.divider,
    )?;
    let startup = Rect::new(PAD, options, 240.0, OPTION_H);
    canvas.text(lang.start_with_windows(), startup, LABEL, palette.text)?;
    let toggle_x = WIDTH - PAD - TOGGLE_W;
    let toggle_y = options + (OPTION_H - TOGGLE_H) / 2.0;
    draw_toggle(canvas, toggle_x, toggle_y, view.startup, palette)?;

    let quit = Rect::new(PAD, options + OPTION_H, 240.0, OPTION_H);
    canvas.text(lang.quit(), quit, LABEL, palette.muted)
}

fn draw_theme_card(
    canvas: &Canvas,
    theme: &Theme,
    rect: Rect,
    selected: bool,
    hover: bool,
    lang: Lang,
    palette: &Palette,
) -> Result<()> {
    // The automatic theme's card shows what it resolves to right now, solid
    // so the wash it uses on the taskbar reads as a surface here.
    let shown = if theme.id == AUTO {
        system_palette()
    } else {
        theme.palette
    };
    let preview = Rect::new(rect.x, rect.y, rect.w, THEME_PREVIEW_H);
    canvas.fill(preview, 8.0, shown.panel)?;

    let line = |y: f32, fill: f32, colors: (Color, Color)| -> Result<()> {
        let track = Rect::new(rect.x + 10.0, y, rect.w - 20.0, 5.0);
        canvas.fill(track, 2.5, shown.track)?;
        let bar = Rect::new(track.x, y, track.w * fill, 5.0);
        canvas.fill_gradient(bar, 2.5, colors.0, colors.1)
    };
    line(rect.y + 12.0, 0.72, shown.bar)?;
    line(rect.y + 25.0, 0.42, shown.bar_model)?;

    if selected {
        canvas.stroke(preview, 8.0, palette.accent, 2.0)?;
    } else {
        let edge = if hover {
            palette.text
        } else {
            palette.panel_border
        };
        canvas.stroke(preview, 8.0, edge, 1.0)?;
    }
    let name = Rect::new(rect.x, rect.y + THEME_PREVIEW_H + 3.0, rect.w, 18.0);
    let color = if selected {
        palette.text
    } else {
        palette.muted
    };
    canvas.text(theme.name(lang), name, SMALL_CENTER, color)
}

fn draw_segment(
    canvas: &Canvas,
    rect: Rect,
    text: &str,
    selected: bool,
    palette: &Palette,
) -> Result<()> {
    if selected {
        canvas.fill(rect, 7.0, palette.accent)?;
        canvas.text(text, rect, SEGMENT, palette.on_accent)
    } else {
        canvas.fill(rect, 7.0, palette.hover)?;
        canvas.text(text, rect, SEGMENT, palette.muted)
    }
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
            percent: 7.0,
            value: "7%".into(),
            resets_at: Some(120_000),
            window_ms: Some(240_000),
            cap: None,
        }
    }

    fn view<'a>(
        page: Page,
        limits: &'a [Limit],
        settings: &'a Settings,
        notice: Option<&'a str>,
    ) -> View<'a> {
        View {
            page,
            limits,
            settings,
            lang: Lang::Es,
            plan: Some("Ultra"),
            age_ms: Some(1000),
            notice,
            startup: false,
            now: 0,
        }
    }

    #[test]
    fn the_usage_page_has_a_target_per_limit_and_the_gear() {
        let limits = [
            limit("weekly", Kind::Weekly),
            limit("weekly:fable", Kind::Model),
        ];
        let settings = Settings::default();
        let actions: Vec<_> = view(Page::Usage, &limits, &settings, None)
            .hits()
            .into_iter()
            .map(|hit| hit.action)
            .collect();
        assert_eq!(
            actions,
            [
                Action::Toggle("weekly".into()),
                Action::Toggle("weekly:fable".into()),
                Action::Page(Page::Settings),
            ]
        );
    }

    #[test]
    fn the_settings_page_offers_every_theme_and_language() {
        let settings = Settings::default();
        let hits = view(Page::Settings, &[], &settings, None).hits();
        for theme in &THEMES {
            assert!(hits.iter().any(|hit| hit.action == Action::Theme(theme.id)));
        }
        assert!(hits.iter().any(|hit| hit.action == Action::Lang(Lang::En)));
        assert!(hits.iter().any(|hit| hit.action == Action::Quit));
    }

    #[test]
    fn every_target_stays_inside_the_panel_on_both_pages() {
        let limits = [limit("weekly", Kind::Weekly)];
        let settings = Settings::default();
        for page in [Page::Usage, Page::Settings] {
            let view = view(page, &limits, &settings, Some("Sin datos nuevos"));
            for hit in view.hits() {
                assert!(hit.rect.y + hit.rect.h <= view.height(), "{:?}", hit.action);
                assert!(hit.rect.x + hit.rect.w <= WIDTH, "{:?}", hit.action);
            }
        }
    }

    #[test]
    fn says_so_only_when_the_plan_has_no_five_hour_limit() {
        let settings = Settings::default();
        let weekly = [limit("weekly", Kind::Weekly)];
        assert!(view(Page::Usage, &weekly, &settings, None).misses_session());

        let both = [
            limit("session", Kind::Session),
            limit("weekly", Kind::Weekly),
        ];
        assert!(!view(Page::Usage, &both, &settings, None).misses_session());
        assert!(!view(Page::Usage, &[], &settings, None).misses_session());
    }

    #[test]
    fn a_notice_pushes_the_rows_down() {
        let limits = [limit("weekly", Kind::Weekly)];
        let settings = Settings::default();
        let plain = view(Page::Usage, &limits, &settings, None).height();
        let noticed = view(Page::Usage, &limits, &settings, Some("Sin datos")).height();
        assert_eq!(noticed, plain + NOTICE_H);
    }

    #[test]
    fn row_detail_reads_for_each_kind_and_language() {
        let weekly = limit("weekly", Kind::Weekly);
        assert_eq!(
            row_detail(&weekly, 0, Lang::Es),
            "Reinicia en 2m · ritmo esperado 50%"
        );
        assert_eq!(
            row_detail(&weekly, 0, Lang::En),
            "Resets in 2m · expected pace 50%"
        );
        let mut extra = limit("extra", Kind::Extra);
        extra.cap = Some(50);
        assert_eq!(row_detail(&extra, 0, Lang::Es), "de $50 este mes");
    }
}
