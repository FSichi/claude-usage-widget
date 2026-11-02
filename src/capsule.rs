//! The capsule on the taskbar: up to two rows per column, one column per pair.

use windows::core::Result;

use crate::gfx::{Align, Canvas, Font, Rect};
use crate::i18n::Lang;
use crate::theme::{bar_fill, Palette};
use crate::usage::{Kind, Limit};

const HEIGHT: f32 = 40.0;
const COMPACT_HEIGHT: f32 = 30.0;

const RADIUS: f32 = 9.0;
const PAD_X: f32 = 11.0;
const ROWS_PER_COLUMN: usize = 2;
const ROW_H: f32 = 17.0;
const ROW_GAP: f32 = 1.0;
const COLUMN_GAP: f32 = 16.0;
const MESSAGE_W: f32 = 150.0;

const LABEL_W: f32 = 56.0;
const BAR_X: f32 = 62.0;
const BAR_W: f32 = 84.0;
const BAR_H: f32 = 5.0;
const VALUE_X: f32 = 152.0;
const VALUE_W: f32 = 34.0;
const TRAILING_X: f32 = 192.0;
const TRAILING_W: f32 = 50.0;
const ROW_W: f32 = TRAILING_X + TRAILING_W;

const COMPACT_ITEM_W: f32 = 54.0;
const DOT: f32 = 8.0;
const DOT_GAP: f32 = 6.0;

const LABEL: Font = Font::new(12.0, 400, Align::Start);
const VALUE: Font = Font::new(12.0, 600, Align::End);
const COMPACT_VALUE: Font = Font::new(13.0, 600, Align::Start);
const MESSAGE: Font = Font::new(13.0, 400, Align::Center);

pub fn width(rows: usize, has_message: bool, compact: bool) -> f32 {
    if has_message || rows == 0 {
        return MESSAGE_W;
    }
    if compact {
        return 2.0 * PAD_X + rows as f32 * COMPACT_ITEM_W;
    }
    let columns = rows.div_ceil(ROWS_PER_COLUMN) as f32;
    2.0 * PAD_X + columns * ROW_W + (columns - 1.0) * COLUMN_GAP
}

pub const fn height(compact: bool) -> f32 {
    if compact {
        COMPACT_HEIGHT
    } else {
        HEIGHT
    }
}

/// Where each row goes, centring a column that holds a single row.
fn row_origins(rows: usize) -> Vec<(f32, f32)> {
    (0..rows)
        .map(|index| {
            let column = index / ROWS_PER_COLUMN;
            let in_column = ROWS_PER_COLUMN.min(rows - column * ROWS_PER_COLUMN);
            let block = in_column as f32 * ROW_H + (in_column - 1) as f32 * ROW_GAP;
            let top = (HEIGHT - block) / 2.0;
            let row = (index % ROWS_PER_COLUMN) as f32;
            (
                PAD_X + column as f32 * (ROW_W + COLUMN_GAP),
                top + row * (ROW_H + ROW_GAP),
            )
        })
        .collect()
}

pub struct View<'a> {
    pub limits: &'a [&'a Limit],
    pub message: Option<&'a str>,
    pub now: i64,
    pub lang: Lang,
    pub hover: bool,
    pub compact: bool,
}

pub fn draw(canvas: &Canvas, view: &View, palette: &Palette) -> Result<()> {
    let total = width(view.limits.len(), view.message.is_some(), view.compact);
    let tall = height(view.compact);
    let frame = Rect::new(0.5, 0.5, total - 1.0, tall - 1.0);
    let background = if view.hover {
        palette.capsule_hover
    } else {
        palette.capsule
    };
    canvas.fill(frame, RADIUS, background)?;
    canvas.stroke(frame, RADIUS, palette.capsule_border, 1.0)?;

    if view.message.is_some() || view.limits.is_empty() {
        let text = view.message.unwrap_or("Claude");
        let area = Rect::new(0.0, 0.0, total, tall);
        return canvas.text(text, area, MESSAGE, palette.muted);
    }
    if view.compact {
        for (index, limit) in view.limits.iter().enumerate() {
            let x = PAD_X + index as f32 * COMPACT_ITEM_W;
            draw_compact_item(canvas, limit, x, view, palette)?;
        }
        return Ok(());
    }
    for (limit, origin) in view.limits.iter().zip(row_origins(view.limits.len())) {
        draw_row(canvas, limit, origin, view, palette)?;
    }
    Ok(())
}

/// The dot carries what the bar would: which kind of limit, and how it stands.
fn draw_compact_item(
    canvas: &Canvas,
    limit: &Limit,
    x: f32,
    view: &View,
    palette: &Palette,
) -> Result<()> {
    let (color, _) = bar_fill(limit, view.now, palette);
    let dot = Rect::new(x, (COMPACT_HEIGHT - DOT) / 2.0, DOT, DOT);
    canvas.fill(dot, DOT / 2.0, color)?;
    let value_x = x + DOT + DOT_GAP;
    let value = Rect::new(value_x, 0.0, COMPACT_ITEM_W - DOT - DOT_GAP, COMPACT_HEIGHT);
    canvas.text(&limit.value, value, COMPACT_VALUE, palette.text)
}

fn draw_row(
    canvas: &Canvas,
    limit: &Limit,
    (x, y): (f32, f32),
    view: &View,
    palette: &Palette,
) -> Result<()> {
    let label = Rect::new(x, y, LABEL_W, ROW_H);
    canvas.text(&limit.short, label, LABEL, palette.muted)?;

    let bar = Rect::new(x + BAR_X, y + (ROW_H - BAR_H) / 2.0, BAR_W, BAR_H);
    let pace = limit.pace(view.now).filter(|_| limit.kind != Kind::Model);
    draw_bar(canvas, bar, limit, view.now, pace, palette)?;

    let value = Rect::new(x + VALUE_X, y, VALUE_W, ROW_H);
    canvas.text(&limit.value, value, VALUE, palette.text)?;

    let trailing = Rect::new(x + TRAILING_X, y, TRAILING_W, ROW_H);
    canvas.text(
        &limit.trailing(view.now, view.lang),
        trailing,
        LABEL,
        palette.muted,
    )
}

pub fn draw_bar(
    canvas: &Canvas,
    bar: Rect,
    limit: &Limit,
    now: i64,
    pace: Option<f64>,
    palette: &Palette,
) -> Result<()> {
    let radius = bar.h / 2.0;
    canvas.fill(bar, radius, palette.track)?;
    let filled = bar.w * limit.percent.min(100.0) as f32 / 100.0;
    if filled > 0.0 {
        // A sliver shorter than its radius would draw as a misshapen dot.
        let shown = Rect::new(bar.x, bar.y, filled.max(bar.h), bar.h);
        let (from, to) = bar_fill(limit, now, palette);
        canvas.fill_gradient(shown, radius, from, to)?;
    }
    if let Some(pace) = pace {
        let x = bar.x + bar.w * pace as f32 / 100.0 - 1.0;
        let tick = Rect::new(x, bar.y - 2.0, 2.0, bar.h + 4.0);
        canvas.fill(tick, 1.0, palette.pace)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grows_a_column_for_every_pair_of_rows() {
        let one_column = width(1, false, false);
        assert_eq!(width(2, false, false), one_column);
        assert_eq!(width(3, false, false), one_column + ROW_W + COLUMN_GAP);
        assert_eq!(width(4, false, false), width(3, false, false));
    }

    #[test]
    fn compact_grows_item_by_item_and_takes_less_room() {
        assert_eq!(
            width(3, false, true) - width(2, false, true),
            COMPACT_ITEM_W
        );
        assert!(width(2, false, true) < width(1, false, false));
        assert!(height(true) < height(false));
    }

    #[test]
    fn a_message_or_nothing_to_show_stays_small() {
        assert_eq!(width(2, true, false), MESSAGE_W);
        assert_eq!(width(0, false, false), MESSAGE_W);
        assert_eq!(width(2, true, true), MESSAGE_W);
    }

    #[test]
    fn a_lone_row_sits_centred_and_a_pair_stays_inside() {
        let (_, top) = row_origins(1)[0];
        assert_eq!(top, (HEIGHT - ROW_H) / 2.0);

        let pair = row_origins(2);
        assert_eq!(pair[0].0, pair[1].0);
        assert!(pair[0].1 > 0.0);
        assert!(pair[1].1 + ROW_H < HEIGHT);
    }

    #[test]
    fn the_third_row_starts_a_column_and_centres_alone() {
        let rows = row_origins(3);
        assert_eq!(rows[2].0, PAD_X + ROW_W + COLUMN_GAP);
        assert_eq!(rows[2].1, (HEIGHT - ROW_H) / 2.0);
    }
}
