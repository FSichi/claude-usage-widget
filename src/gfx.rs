//! Direct2D drawing onto per-pixel-alpha layered windows.

use std::cell::RefCell;
use std::collections::HashMap;

use windows::core::{w, Result};
use windows::Win32::Foundation::{COLORREF, HWND, POINT, RECT, SIZE};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_GRADIENT_STOP, D2D1_PIXEL_FORMAT, D2D_RECT_F,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory, D2D1_DRAW_TEXT_OPTIONS_CLIP,
    D2D1_EXTEND_MODE_CLAMP, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_GAMMA_2_2,
    D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES, D2D1_RENDER_TARGET_PROPERTIES,
    D2D1_RENDER_TARGET_TYPE_SOFTWARE, D2D1_ROUNDED_RECT, D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT,
    DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
    DWRITE_WORD_WRAPPING_NO_WRAP,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
    HDC,
};
use windows::Win32::UI::WindowsAndMessaging::{UpdateLayeredWindow, ULW_ALPHA};
use windows_numerics::Vector2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const fn hex(rgb: u32) -> Self {
        Self {
            r: ((rgb >> 16) & 0xff) as f32 / 255.0,
            g: ((rgb >> 8) & 0xff) as f32 / 255.0,
            b: (rgb & 0xff) as f32 / 255.0,
            a: 1.0,
        }
    }

    pub const fn alpha(self, a: f32) -> Self {
        Self { a, ..self }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    fn d2d(&self) -> D2D_RECT_F {
        D2D_RECT_F {
            left: self.x,
            top: self.y,
            right: self.x + self.w,
            bottom: self.y + self.h,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Align {
    Start,
    Center,
    End,
}

impl Align {
    fn dwrite(self) -> DWRITE_TEXT_ALIGNMENT {
        match self {
            Align::Start => DWRITE_TEXT_ALIGNMENT_LEADING,
            Align::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
            Align::End => DWRITE_TEXT_ALIGNMENT_TRAILING,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Family {
    Text,
    Icons,
}

#[derive(Clone, Copy, Debug)]
pub struct Font {
    pub size: f32,
    pub weight: i32,
    pub align: Align,
    pub family: Family,
}

impl Font {
    pub const fn new(size: f32, weight: i32, align: Align) -> Self {
        Self {
            size,
            weight,
            align,
            family: Family::Text,
        }
    }

    pub const fn icon(size: f32) -> Self {
        Self {
            size,
            weight: 400,
            align: Align::Center,
            family: Family::Icons,
        }
    }
}

pub struct Gfx {
    target: ID2D1DCRenderTarget,
    dwrite: IDWriteFactory,
    formats: RefCell<HashMap<(u32, i32, Align, Family), IDWriteTextFormat>>,
}

impl Gfx {
    pub fn new() -> Result<Self> {
        unsafe {
            let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            // Software: the frame is read back into a bitmap on every present.
            let properties = D2D1_RENDER_TARGET_PROPERTIES {
                r#type: D2D1_RENDER_TARGET_TYPE_SOFTWARE,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                ..Default::default()
            };
            let target = factory.CreateDCRenderTarget(&properties)?;
            // ClearType needs an opaque backdrop, which a layered window lacks.
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            Ok(Self {
                target,
                dwrite: DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?,
                formats: RefCell::default(),
            })
        }
    }

    fn format(&self, font: Font) -> Result<IDWriteTextFormat> {
        let key = (
            (font.size * 10.0) as u32,
            font.weight,
            font.align,
            font.family,
        );
        if let Some(found) = self.formats.borrow().get(&key) {
            return Ok(found.clone());
        }
        let family = match font.family {
            Family::Text => w!("Segoe UI Variable Text"),
            Family::Icons => w!("Segoe Fluent Icons"),
        };
        let format = unsafe {
            let format = self.dwrite.CreateTextFormat(
                family,
                None,
                DWRITE_FONT_WEIGHT(font.weight),
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font.size,
                w!("es-ar"),
            )?;
            format.SetTextAlignment(font.align.dwrite())?;
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            format
        };
        self.formats.borrow_mut().insert(key, format.clone());
        Ok(format)
    }

    /// Draws a frame of `width` x `height` physical pixels and shows it.
    pub fn present(
        &self,
        hwnd: HWND,
        (width, height): (i32, i32),
        dpi: u32,
        opacity: u8,
        draw: impl FnOnce(&Canvas) -> Result<()>,
    ) -> Result<()> {
        unsafe {
            let screen = GetDC(None);
            let memory = CreateCompatibleDC(Some(screen));
            let result = self.present_on(hwnd, screen, memory, (width, height), dpi, opacity, draw);
            let _ = DeleteDC(memory);
            ReleaseDC(None, screen);
            result
        }
    }

    #[allow(clippy::too_many_arguments)]
    unsafe fn present_on(
        &self,
        hwnd: HWND,
        screen: HDC,
        memory: HDC,
        (width, height): (i32, i32),
        dpi: u32,
        opacity: u8,
        draw: impl FnOnce(&Canvas) -> Result<()>,
    ) -> Result<()> {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(memory), &info, DIB_RGB_COLORS, &mut bits, None, 0)?;
        let previous = SelectObject(memory, bitmap.into());

        let result = (|| {
            let area = RECT {
                left: 0,
                top: 0,
                right: width,
                bottom: height,
            };
            self.target.BindDC(memory, &area)?;
            self.target.SetDpi(dpi as f32, dpi as f32);
            self.target.BeginDraw();
            self.target.Clear(Some(&D2D1_COLOR_F::default()));
            let drawn = draw(&Canvas { gfx: self });
            self.target.EndDraw(None, None)?;
            drawn?;

            // Windows lets clicks through zero-alpha pixels; this keeps the
            // whole surface clickable without changing how it looks.
            let pixels =
                std::slice::from_raw_parts_mut(bits as *mut u32, (width * height) as usize);
            for pixel in pixels.iter_mut().filter(|pixel| **pixel >> 24 == 0) {
                *pixel = 0x0100_0000;
            }

            let size = SIZE {
                cx: width,
                cy: height,
            };
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: opacity,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            UpdateLayeredWindow(
                hwnd,
                Some(screen),
                None,
                Some(&size),
                Some(memory),
                Some(&POINT::default()),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
        })();

        SelectObject(memory, previous);
        let _ = DeleteObject(bitmap.into());
        result
    }
}

/// The frame being drawn, in device-independent pixels.
pub struct Canvas<'a> {
    gfx: &'a Gfx,
}

impl Canvas<'_> {
    pub fn fill(&self, rect: Rect, radius: f32, color: Color) -> Result<()> {
        let target = &self.gfx.target;
        unsafe {
            let brush = target.CreateSolidColorBrush(&d2d_color(color), None)?;
            target.FillRoundedRectangle(&rounded(rect, radius), &brush);
        }
        Ok(())
    }

    pub fn fill_gradient(&self, rect: Rect, radius: f32, from: Color, to: Color) -> Result<()> {
        let target = &self.gfx.target;
        let stops = [
            D2D1_GRADIENT_STOP {
                position: 0.0,
                color: d2d_color(from),
            },
            D2D1_GRADIENT_STOP {
                position: 1.0,
                color: d2d_color(to),
            },
        ];
        let line = D2D1_LINEAR_GRADIENT_BRUSH_PROPERTIES {
            startPoint: Vector2 {
                X: rect.x,
                Y: rect.y,
            },
            endPoint: Vector2 {
                X: rect.x + rect.w,
                Y: rect.y,
            },
        };
        unsafe {
            let collection = target.CreateGradientStopCollection(
                &stops,
                D2D1_GAMMA_2_2,
                D2D1_EXTEND_MODE_CLAMP,
            )?;
            let brush = target.CreateLinearGradientBrush(&line, None, &collection)?;
            target.FillRoundedRectangle(&rounded(rect, radius), &brush);
        }
        Ok(())
    }

    pub fn stroke(&self, rect: Rect, radius: f32, color: Color, width: f32) -> Result<()> {
        let target = &self.gfx.target;
        unsafe {
            let brush = target.CreateSolidColorBrush(&d2d_color(color), None)?;
            target.DrawRoundedRectangle(&rounded(rect, radius), &brush, width, None);
        }
        Ok(())
    }

    pub fn text(&self, text: &str, rect: Rect, font: Font, color: Color) -> Result<()> {
        let target = &self.gfx.target;
        let wide: Vec<u16> = text.encode_utf16().collect();
        unsafe {
            let brush = target.CreateSolidColorBrush(&d2d_color(color), None)?;
            target.DrawText(
                &wide,
                &self.gfx.format(font)?,
                &rect.d2d(),
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
                DWRITE_MEASURING_MODE_NATURAL,
            );
        }
        Ok(())
    }
}

fn rounded(rect: Rect, radius: f32) -> D2D1_ROUNDED_RECT {
    D2D1_ROUNDED_RECT {
        rect: rect.d2d(),
        radiusX: radius,
        radiusY: radius,
    }
}

fn d2d_color(color: Color) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: color.r,
        g: color.g,
        b: color.b,
        a: color.a,
    }
}
