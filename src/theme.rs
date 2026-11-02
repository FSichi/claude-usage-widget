//! The themes, and the automatic one that follows Windows.

use windows::core::w;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

use crate::gfx::Color;
use crate::i18n::Lang;
use crate::usage::{Kind, Limit, Severity};

pub const AUTO: &str = "auto";

const AMBER: Color = Color::hex(0xe9b949);
const RED: Color = Color::hex(0xef5350);
pub const WARNING: Color = AMBER;

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub capsule: Color,
    pub capsule_hover: Color,
    pub capsule_border: Color,
    pub text: Color,
    pub muted: Color,
    pub track: Color,
    pub pace: Color,
    pub panel: Color,
    pub panel_border: Color,
    pub divider: Color,
    pub toggle_off: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub hover: Color,
    pub bar: (Color, Color),
    pub bar_model: (Color, Color),
}

pub struct Theme {
    pub id: &'static str,
    es: &'static str,
    en: &'static str,
    pub palette: Palette,
}

impl Theme {
    pub const fn name(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::Es => self.es,
            Lang::En => self.en,
        }
    }
}

/// A dark palette from its surfaces, its text and its two bar gradients.
const fn dark(
    capsule: u32,
    panel: u32,
    text: u32,
    muted: u32,
    accent: u32,
    bar: (u32, u32),
    bar_model: (u32, u32),
) -> Palette {
    let white = Color::hex(0xffffff);
    Palette {
        capsule: Color::hex(capsule),
        capsule_hover: Color::hex(panel),
        capsule_border: white.alpha(0.10),
        text: Color::hex(text),
        muted: Color::hex(muted),
        track: white.alpha(0.15),
        pace: Color::hex(text),
        panel: Color::hex(panel),
        panel_border: white.alpha(0.11),
        divider: white.alpha(0.08),
        toggle_off: white.alpha(0.2),
        accent: Color::hex(accent),
        on_accent: Color::hex(capsule),
        hover: white.alpha(0.06),
        bar: (Color::hex(bar.0), Color::hex(bar.1)),
        bar_model: (Color::hex(bar_model.0), Color::hex(bar_model.1)),
    }
}

/// A light palette, the same recipe with dark washes over the surfaces.
const fn light(
    capsule: u32,
    panel: u32,
    text: u32,
    muted: u32,
    accent: u32,
    bar: (u32, u32),
    bar_model: (u32, u32),
) -> Palette {
    let black = Color::hex(0x000000);
    Palette {
        capsule: Color::hex(capsule),
        capsule_hover: Color::hex(panel),
        capsule_border: black.alpha(0.09),
        text: Color::hex(text),
        muted: Color::hex(muted),
        track: black.alpha(0.13),
        pace: Color::hex(text),
        panel: Color::hex(panel),
        panel_border: black.alpha(0.1),
        divider: black.alpha(0.08),
        toggle_off: black.alpha(0.24),
        accent: Color::hex(accent),
        on_accent: Color::hex(0xffffff),
        hover: black.alpha(0.05),
        bar: (Color::hex(bar.0), Color::hex(bar.1)),
        bar_model: (Color::hex(bar_model.0), Color::hex(bar_model.1)),
    }
}

const SYSTEM_DARK: Palette = Palette {
    capsule: Color::hex(0xffffff).alpha(0.07),
    capsule_hover: Color::hex(0xffffff).alpha(0.12),
    capsule_border: Color::hex(0xffffff).alpha(0.05),
    ..dark(
        0x1b2233,
        0x1e2433,
        0xf2f4f8,
        0xaab3c5,
        0x4c8dff,
        (0xe0683f, 0xf0a070),
        (0x8f7cf0, 0xc2b5fa),
    )
};

const SYSTEM_LIGHT: Palette = Palette {
    capsule: Color::hex(0x000000).alpha(0.05),
    capsule_hover: Color::hex(0x000000).alpha(0.09),
    capsule_border: Color::hex(0x000000).alpha(0.05),
    ..light(
        0xf1f3f7,
        0xf8f9fb,
        0x1b2233,
        0x566074,
        0x2f6fed,
        (0xcf5a33, 0xe98a5c),
        (0x6d57dd, 0x9c8bee),
    )
};

/// The automatic entry comes first; its card shows the current Windows mode.
pub const THEMES: [Theme; 8] = [
    Theme {
        id: AUTO,
        es: "Sistema",
        en: "System",
        palette: SYSTEM_DARK,
    },
    Theme {
        id: "midnight",
        es: "Medianoche",
        en: "Midnight",
        palette: dark(
            0x10162a,
            0x151c33,
            0xeef2ff,
            0x9aa6c9,
            0x6f9dff,
            (0x4f7dff, 0x8fc0ff),
            (0x9b7bff, 0xd0b6ff),
        ),
    },
    Theme {
        id: "graphite",
        es: "Grafito",
        en: "Graphite",
        palette: dark(
            0x18181b,
            0x1f1f23,
            0xf4f4f5,
            0xa1a1aa,
            0xe4e4e7,
            (0xa1a1aa, 0xfafafa),
            (0x71717a, 0xd4d4d8),
        ),
    },
    Theme {
        id: "aurora",
        es: "Aurora",
        en: "Aurora",
        palette: dark(
            0x0b2024,
            0x0e292e,
            0xe8fbf7,
            0x8fbdb6,
            0x2dd4bf,
            (0x14b8a6, 0x5eead4),
            (0x4ade80, 0xbbf7d0),
        ),
    },
    Theme {
        id: "ember",
        es: "Brasa",
        en: "Ember",
        palette: dark(
            0x21130e,
            0x2a1912,
            0xfff2ea,
            0xc9a896,
            0xff8a4c,
            (0xff6a3d, 0xffb26b),
            (0xf5b83d, 0xffe08a),
        ),
    },
    Theme {
        id: "orchid",
        es: "Orquídea",
        en: "Orchid",
        palette: dark(
            0x1f1230,
            0x27173b,
            0xf8efff,
            0xb7a3cc,
            0xd08cff,
            (0xd946ef, 0xf0abfc),
            (0x8b5cf6, 0xc4b5fd),
        ),
    },
    Theme {
        id: "sand",
        es: "Arena",
        en: "Sand",
        palette: light(
            0xf3ece0,
            0xfaf6ee,
            0x2b2620,
            0x6f6557,
            0xb9532f,
            (0xc9623f, 0xe39a6e),
            (0x7a62c9, 0xa995e6),
        ),
    },
    Theme {
        id: "ice",
        es: "Hielo",
        en: "Ice",
        palette: light(
            0xeaf1fb,
            0xf6f9ff,
            0x18263d,
            0x56657d,
            0x2563eb,
            (0x2563eb, 0x60a5fa),
            (0x0d9488, 0x5eead4),
        ),
    },
];

pub fn palette(id: &str) -> Palette {
    if id == AUTO {
        return system_palette();
    }
    THEMES
        .iter()
        .find(|theme| theme.id == id)
        .map_or_else(system_palette, |theme| theme.palette)
}

pub fn system_palette() -> Palette {
    if taskbar_is_light() {
        SYSTEM_LIGHT
    } else {
        SYSTEM_DARK
    }
}

/// The fill of a limit's bar: the theme's until the limit nears its cap.
pub fn bar_fill(limit: &Limit, palette: &Palette) -> (Color, Color) {
    match limit.severity() {
        Severity::Critical => (RED, RED),
        Severity::High => (AMBER, AMBER),
        Severity::Normal if limit.kind == Kind::Model => palette.bar_model,
        Severity::Normal => palette.bar,
    }
}

fn taskbar_is_light() -> bool {
    let mut value = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    result.is_ok() && value == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(color: Color) -> f32 {
        let linear = |c: f32| {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
    }

    fn contrast(a: Color, b: Color) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    #[test]
    fn every_theme_stays_readable() {
        for theme in &THEMES {
            let palette = &theme.palette;
            assert!(
                contrast(palette.text, palette.panel) >= 7.0,
                "{} text",
                theme.id
            );
            assert!(
                contrast(palette.muted, palette.panel) >= 4.5,
                "{} muted text",
                theme.id
            );
        }
    }

    #[test]
    fn ids_are_unique_and_both_languages_named() {
        for (index, theme) in THEMES.iter().enumerate() {
            assert!(THEMES[index + 1..].iter().all(|other| other.id != theme.id));
            assert!(!theme.name(Lang::Es).is_empty());
            assert!(!theme.name(Lang::En).is_empty());
        }
    }

    #[test]
    fn an_unknown_theme_falls_back_to_the_system() {
        let found = palette("gone");
        assert!(found.capsule.a < 1.0);
    }

    #[test]
    fn a_limit_near_its_cap_overrides_the_theme() {
        let mut limit = Limit {
            id: "weekly".into(),
            short: String::new(),
            title: String::new(),
            kind: Kind::Weekly,
            percent: 10.0,
            value: String::new(),
            resets_at: None,
            window_ms: None,
            cap: None,
        };
        let palette = THEMES[1].palette;
        assert_eq!(bar_fill(&limit, &palette), palette.bar);
        limit.percent = 99.0;
        assert_eq!(bar_fill(&limit, &palette), (RED, RED));
    }
}
