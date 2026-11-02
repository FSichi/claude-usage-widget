//! The windows, and the state they share on the UI thread.
//!
//! The capsule is a child of the taskbar, so its thread shares input with
//! Explorer: nothing here may block, which is why the panel is a plain window
//! and not a system menu.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use windows::core::{w, Result, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, ReleaseCapture, SetCapture, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT,
    VK_LBUTTON, VK_RBUTTON,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    GetWindowRect, KillTimer, LoadCursorW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW,
    SetCursor, SetTimer, SetWindowPos, ShowWindow, TranslateMessage, HTCLIENT, HWND_TOPMOST,
    IDC_ARROW, IDC_HAND, MA_NOACTIVATE, MSG, SWP_NOACTIVATE, SWP_NOZORDER, SW_SHOWNOACTIVATE,
    WM_DESTROY, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE, WM_MOUSEMOVE, WM_NCHITTEST,
    WM_RBUTTONUP, WM_SETCURSOR, WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::alerts;
use crate::bridge;
use crate::capsule;
use crate::gfx::Gfx;
use crate::history::History;
use crate::i18n::LocalTime;
use crate::notify;
use crate::panel::{self, Action, Hit, Page};
use crate::settings::{self, usage_path, Settings};
use crate::startup;
use crate::taskbar;
use crate::theme;
use crate::usage::{self, Kind, Limit, Snapshot, STALE_AFTER_MS};

const HOST_CLASS: PCWSTR = w!("ClaudeUsageWidgetHost");
const CAPSULE_CLASS: PCWSTR = w!("ClaudeUsageWidgetCapsule");
const PANEL_CLASS: PCWSTR = w!("ClaudeUsageWidgetPanel");

const TIMER_TICK: usize = 1;
const TIMER_PANEL: usize = 2;
const TICK_MS: u32 = 2000;
const PANEL_POLL_MS: u32 = 60;
/// Countdowns and ages move slowly; every 15 ticks keeps them honest.
const REDRAW_EVERY_TICKS: u32 = 15;

const DEFAULT_OFFSET_DIP: f32 = 8.0;
const DRAG_THRESHOLD_PX: i32 = 4;
const STALE_OPACITY: u8 = 150;
const WM_MOUSELEAVE: u32 = 0x02A3;

static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

thread_local! {
    static APP: RefCell<Option<App>> = const { RefCell::new(None) };
}

/// Win32 can re-enter a window procedure while `App` is borrowed; such a
/// nested message just takes the default handling.
fn with_app<R>(f: impl FnOnce(&mut App) -> R) -> Option<R> {
    APP.with(|cell| cell.try_borrow_mut().ok()?.as_mut().map(f))
}

struct Drag {
    cursor_x: i32,
    offset: i32,
    moved: bool,
}

struct App {
    gfx: Gfx,
    instance: HINSTANCE,
    host: HWND,
    taskbar: Option<HWND>,
    capsule: Option<HWND>,
    embed_failed: bool,
    panel: Option<HWND>,
    page: Page,
    hits: Vec<Hit>,
    panel_hover: Option<usize>,
    panel_tracking: bool,
    settings: Settings,
    history: History,
    snapshot: Option<Snapshot>,
    modified: Option<SystemTime>,
    hover: bool,
    tracking: bool,
    drag: Option<Drag>,
    ticks: u32,
}

pub fn run() -> Result<()> {
    unsafe {
        let instance: HINSTANCE = GetModuleHandleW(None)?.into();
        for (class, proc) in [
            (HOST_CLASS, host_proc as WindowProc),
            (CAPSULE_CLASS, capsule_proc),
            (PANEL_CLASS, panel_proc),
        ] {
            RegisterClassW(&WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: instance,
                lpszClassName: class,
                hCursor: LoadCursorW(None, IDC_ARROW)?,
                ..Default::default()
            });
        }
        TASKBAR_CREATED.store(
            RegisterWindowMessageW(w!("TaskbarCreated")),
            Ordering::Relaxed,
        );

        // A hidden top-level window: it outlives Explorer, which takes the
        // capsule down with it, and it hears the TaskbarCreated broadcast.
        let host = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            HOST_CLASS,
            w!("Claude usage"),
            WS_POPUP,
            0,
            0,
            0,
            0,
            None,
            None,
            Some(instance),
            None,
        )?;

        let app = App {
            gfx: Gfx::new()?,
            instance,
            host,
            taskbar: None,
            capsule: None,
            embed_failed: false,
            panel: None,
            page: Page::Usage,
            hits: Vec::new(),
            panel_hover: None,
            panel_tracking: false,
            settings: load_settings(),
            history: History::load(),
            snapshot: None,
            modified: None,
            hover: false,
            tracking: false,
            drag: None,
            ticks: 0,
        };
        APP.with(|cell| *cell.borrow_mut() = Some(app));
        with_app(App::tick);
        SetTimer(Some(host), TIMER_TICK, TICK_MS, None);
        bridge::ensure_installed();
        startup::ensure_menu_entry();
        std::thread::spawn(notify::register);

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    Ok(())
}

/// The first run turns on starting with Windows; from then on it is the
/// person's toggle.
fn load_settings() -> Settings {
    let mut settings = Settings::load();
    if !settings.initialized {
        settings.initialized = true;
        startup::set_enabled(true);
        settings.save();
    }
    settings
}

type WindowProc = unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT;

unsafe extern "system" fn host_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_TIMER if wp.0 == TIMER_TICK => {
            with_app(App::tick);
            LRESULT(0)
        }
        WM_TIMER if wp.0 == TIMER_PANEL => {
            with_app(App::dismiss_panel_on_outside_click);
            LRESULT(0)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ if msg == TASKBAR_CREATED.load(Ordering::Relaxed) => {
            with_app(App::ensure_capsule);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe extern "system" fn capsule_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let handled = match msg {
        WM_NCHITTEST => return LRESULT(HTCLIENT as isize),
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
        WM_MOUSEMOVE => with_app(App::capsule_mouse_move),
        WM_MOUSELEAVE => with_app(App::capsule_mouse_leave),
        WM_LBUTTONDOWN => with_app(App::capsule_press),
        WM_LBUTTONUP => with_app(App::capsule_release),
        WM_RBUTTONUP => with_app(App::toggle_panel),
        _ => None,
    };
    match handled {
        Some(()) => LRESULT(0),
        None => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe extern "system" fn panel_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let handled = match msg {
        WM_NCHITTEST => return LRESULT(HTCLIENT as isize),
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
        WM_SETCURSOR => with_app(App::panel_cursor),
        WM_MOUSEMOVE => with_app(|app| app.panel_mouse_move(hwnd, lp)),
        WM_MOUSELEAVE => with_app(App::panel_mouse_leave),
        WM_LBUTTONUP => with_app(|app| app.panel_click(hwnd, lp)),
        _ => None,
    };
    match handled {
        Some(()) => LRESULT(0),
        None => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

fn scale_of(hwnd: HWND) -> f32 {
    unsafe { GetDpiForWindow(hwnd) as f32 / 96.0 }
}

fn cursor() -> POINT {
    let mut point = POINT::default();
    let _ = unsafe { GetCursorPos(&mut point) };
    point
}

fn window_rect(hwnd: HWND) -> RECT {
    let mut rect = RECT::default();
    let _ = unsafe { GetWindowRect(hwnd, &mut rect) };
    rect
}

fn contains(rect: &RECT, point: POINT) -> bool {
    point.x >= rect.left && point.x < rect.right && point.y >= rect.top && point.y < rect.bottom
}

fn track_leave(hwnd: HWND) {
    let mut event = TRACKMOUSEEVENT {
        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
        dwFlags: TME_LEAVE,
        hwndTrack: hwnd,
        dwHoverTime: 0,
    };
    let _ = unsafe { TrackMouseEvent(&mut event) };
}

fn dip_position(hwnd: HWND, lp: LPARAM) -> (f32, f32) {
    let scale = scale_of(hwnd);
    let x = (lp.0 & 0xffff) as i16 as f32;
    let y = ((lp.0 >> 16) & 0xffff) as i16 as f32;
    (x / scale, y / scale)
}

impl App {
    fn tick(&mut self) {
        self.ensure_capsule();
        let changed = self.reload();
        if changed && !self.is_stale() {
            self.check_alerts();
            self.record_history();
        }
        self.ticks = self.ticks.wrapping_add(1);
        if changed || self.ticks.is_multiple_of(REDRAW_EVERY_TICKS) {
            self.redraw();
        }
    }

    /// Reads the usage file when it changed. A write caught halfway does not
    /// parse: the last good snapshot stays and the next tick retries.
    fn reload(&mut self) -> bool {
        let path = usage_path();
        let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
        if modified == self.modified {
            return false;
        }
        let parsed = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| usage::parse(&text));
        match parsed {
            Some(snapshot) => {
                self.modified = modified;
                self.snapshot = Some(snapshot);
                true
            }
            None => false,
        }
    }

    fn record_history(&mut self) {
        let limits = self.limits();
        let weekly = limits.iter().find(|limit| limit.kind == Kind::Weekly);
        let today = LocalTime::from_ms(now_ms());
        if let (Some(weekly), Some(today)) = (weekly, today) {
            let day = today.day_number();
            if self.history.record(day, weekly.percent, weekly.resets_at) {
                self.history.save();
            }
        }
    }

    /// Marks stay current with alerts off, so turning them on does not announce
    /// what already happened.
    fn check_alerts(&mut self) {
        let limits = self.limits();
        let warn_at = f64::from(self.settings.alert_at);
        let mut events = Vec::new();
        let mut changed = false;
        for limit in &limits {
            let mark = self.settings.marks.get(&limit.id).copied();
            let (next, event) = alerts::check(limit, mark.unwrap_or_default(), warn_at);
            changed |= mark != Some(next);
            self.settings.marks.insert(limit.id.clone(), next);
            events.extend(event.map(|event| (limit, event)));
        }
        if changed {
            self.settings.save();
        }
        if self.settings.alerts {
            if let Some((title, body)) = alerts::message(&events, now_ms(), self.settings.lang()) {
                notify::show(title, body);
            }
        }
    }

    fn limits(&self) -> Vec<Limit> {
        self.snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.plan.as_ref())
            .map(|plan| usage::limits(plan, self.settings.lang()))
            .unwrap_or_default()
    }

    fn age_ms(&self) -> Option<i64> {
        self.snapshot.as_ref().map(|s| now_ms() - s.updated_at)
    }

    fn is_stale(&self) -> bool {
        self.age_ms().is_some_and(|age| age > STALE_AFTER_MS)
    }

    /// The capsule's text when it has no bars to show.
    fn capsule_message(&self, limits: &[Limit]) -> Option<&'static str> {
        let lang = self.settings.lang();
        if self.snapshot.is_none() {
            return Some(lang.waiting());
        }
        limits.is_empty().then_some(lang.no_data())
    }

    /// What the panel warns about, if anything.
    fn panel_notice(&self, limits: &[Limit]) -> Option<&'static str> {
        let lang = self.settings.lang();
        let Some(snapshot) = &self.snapshot else {
            return Some(if bridge::claude_is_missing() {
                lang.notice_no_claude()
            } else {
                lang.notice_first_run()
            });
        };
        if self.is_stale() {
            return Some(lang.notice_stale());
        }
        match snapshot.plan.as_ref().map(|plan| plan.status.as_str()) {
            Some("not_applicable") => Some(lang.notice_no_limits()),
            Some("ok") if !limits.is_empty() => None,
            _ => Some(lang.notice_unavailable()),
        }
    }

    fn ensure_capsule(&mut self) {
        let Some(found) = taskbar::find(self.settings.monitor) else {
            return;
        };
        let alive = self.capsule.is_some_and(taskbar::is_alive);
        if alive && self.taskbar == Some(found) {
            if let Some(hwnd) = self.capsule {
                if !taskbar::is_child_of(hwnd, found) {
                    self.embed_capsule(hwnd, found);
                }
            }
            return;
        }
        self.close_panel();
        if let Some(old) = self.capsule.take().filter(|hwnd| taskbar::is_alive(*hwnd)) {
            let _ = unsafe { DestroyWindow(old) };
        }
        let created = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                CAPSULE_CLASS,
                w!("Claude usage"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                Some(self.instance),
                None,
            )
        };
        let Ok(hwnd) = created else {
            return;
        };
        self.taskbar = Some(found);
        self.capsule = Some(hwnd);
        self.hover = false;
        self.tracking = false;
        self.embed_failed = false;
        self.embed_capsule(hwnd, found);
    }

    /// Stays hidden until it is inside the taskbar: loose, it would sit at the
    /// top of the screen, placed by taskbar coordinates.
    fn embed_capsule(&mut self, hwnd: HWND, found: HWND) {
        match taskbar::embed(hwnd, found) {
            Ok(()) => {
                self.redraw_capsule();
                let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };
            }
            Err(error) if !self.embed_failed => {
                self.embed_failed = true;
                settings::log(&format!("{}: taskbar embed failed: {error}", now_ms()));
            }
            Err(_) => {}
        }
    }

    fn redraw(&mut self) {
        self.redraw_capsule();
        self.redraw_panel();
    }

    fn capsule_size(&self, hwnd: HWND, limits: &[Limit]) -> (i32, i32) {
        let visible = limits
            .iter()
            .filter(|l| self.settings.is_visible(l))
            .count();
        let scale = scale_of(hwnd);
        let compact = self.settings.compact;
        let message = self.capsule_message(limits).is_some();
        (
            (capsule::width(visible, message, compact) * scale).ceil() as i32,
            (capsule::height(compact) * scale).ceil() as i32,
        )
    }

    fn offset_x(&self, hwnd: HWND) -> i32 {
        self.settings
            .offset_x
            .unwrap_or((DEFAULT_OFFSET_DIP * scale_of(hwnd)).round() as i32)
    }

    /// Puts the capsule at `offset`, or as near as the taskbar allows, and
    /// answers where it landed.
    fn place_capsule(&self, hwnd: HWND, offset: i32, (width, height): (i32, i32)) -> i32 {
        let Some((bar_width, bar_height)) = self.taskbar.and_then(taskbar::size) else {
            return offset;
        };
        let x = offset.clamp(0, (bar_width - width).max(0));
        let y = ((bar_height - height) / 2).max(0);
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                None,
                x,
                y,
                width,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            )
        };
        x
    }

    fn redraw_capsule(&mut self) {
        let Some(hwnd) = self.capsule else {
            return;
        };
        let limits = self.limits();
        let size = self.capsule_size(hwnd, &limits);
        let offset = self
            .drag
            .as_ref()
            .map_or_else(|| self.offset_x(hwnd), |drag| drag.offset);
        self.place_capsule(hwnd, offset, size);

        let visible: Vec<&Limit> = limits
            .iter()
            .filter(|l| self.settings.is_visible(l))
            .collect();
        let view = capsule::View {
            limits: &visible,
            message: self.capsule_message(&limits),
            now: now_ms(),
            lang: self.settings.lang(),
            hover: self.hover,
            compact: self.settings.compact,
        };
        let dpi = unsafe { GetDpiForWindow(hwnd) };
        let opacity = if self.is_stale() { STALE_OPACITY } else { 255 };
        let palette = theme::palette(&self.settings.theme);
        let _ = self.gfx.present(hwnd, size, dpi, opacity, |canvas| {
            capsule::draw(canvas, &view, &palette)
        });
    }

    fn capsule_mouse_move(&mut self) {
        let Some(hwnd) = self.capsule else {
            return;
        };
        if !self.tracking {
            self.tracking = true;
            track_leave(hwnd);
        }
        if !self.hover {
            self.hover = true;
            self.redraw_capsule();
        }
        let x = cursor().x;
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        let delta = x - drag.cursor_x;
        drag.moved |= delta.abs() >= DRAG_THRESHOLD_PX;
        if !drag.moved {
            return;
        }
        let offset = drag.offset + delta;
        drag.cursor_x = x;
        self.close_panel();
        let limits = self.limits();
        let size = self.capsule_size(hwnd, &limits);
        let placed = self.place_capsule(hwnd, offset, size);
        if let Some(drag) = self.drag.as_mut() {
            drag.offset = placed;
        }
    }

    fn capsule_mouse_leave(&mut self) {
        self.tracking = false;
        self.hover = false;
        self.redraw_capsule();
    }

    fn capsule_press(&mut self) {
        let Some(hwnd) = self.capsule else {
            return;
        };
        unsafe { SetCapture(hwnd) };
        self.drag = Some(Drag {
            cursor_x: cursor().x,
            offset: self.offset_x(hwnd),
            moved: false,
        });
    }

    fn capsule_release(&mut self) {
        let _ = unsafe { ReleaseCapture() };
        let Some(drag) = self.drag.take() else {
            return;
        };
        if drag.moved {
            // Saved as placed, so the clamp at the taskbar's ends sticks.
            if let (Some(hwnd), Some(taskbar)) = (self.capsule, self.taskbar) {
                let left = window_rect(hwnd).left;
                let bar_left = window_rect(taskbar).left;
                self.settings.offset_x = Some(left - bar_left);
                self.settings.save();
            }
        } else {
            self.toggle_panel();
        }
    }

    fn toggle_panel(&mut self) {
        if self.panel.is_some() {
            self.close_panel();
        } else {
            self.open_panel();
        }
    }

    fn open_panel(&mut self) {
        let created = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                PANEL_CLASS,
                w!("Uso de Claude"),
                WS_POPUP,
                0,
                0,
                1,
                1,
                Some(self.host),
                None,
                Some(self.instance),
                None,
            )
        };
        let Ok(hwnd) = created else {
            return;
        };
        self.panel = Some(hwnd);
        self.page = Page::Usage;
        self.panel_hover = None;
        self.panel_tracking = false;
        self.redraw_panel();
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            SetTimer(Some(self.host), TIMER_PANEL, PANEL_POLL_MS, None);
        }
    }

    fn close_panel(&mut self) {
        if let Some(hwnd) = self.panel.take() {
            unsafe {
                let _ = KillTimer(Some(self.host), TIMER_PANEL);
                let _ = DestroyWindow(hwnd);
            }
        }
    }

    /// The panel never takes focus, so a click elsewhere cannot deactivate it.
    fn dismiss_panel_on_outside_click(&mut self) {
        let (Some(panel), Some(capsule)) = (self.panel, self.capsule) else {
            return;
        };
        let pressed = unsafe {
            GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0 || GetAsyncKeyState(VK_RBUTTON.0 as i32) < 0
        };
        let point = cursor();
        let inside = contains(&window_rect(panel), point) || contains(&window_rect(capsule), point);
        if pressed && !inside {
            self.close_panel();
        }
    }

    fn redraw_panel(&mut self) {
        let (Some(hwnd), Some(capsule)) = (self.panel, self.capsule) else {
            return;
        };
        let limits = self.limits();
        let plan = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.plan.as_ref())
            .and_then(|plan| plan.plan.as_deref());
        let today = LocalTime::from_ms(now_ms());
        let view = panel::View {
            page: self.page,
            limits: &limits,
            settings: &self.settings,
            lang: self.settings.lang(),
            plan,
            age_ms: self.age_ms(),
            notice: self.panel_notice(&limits),
            startup: startup::is_enabled(),
            now: now_ms(),
            monitors: taskbar::count(),
            monitor: self.taskbar.and_then(taskbar::index_of).unwrap_or(0),
            history: &self.history,
            today: today.map_or(0, |today| today.day_number()),
            weekday: today.map_or(0, |today| today.weekday),
        };
        let scale = scale_of(capsule);
        let width = (panel::WIDTH * scale).ceil() as i32;
        let height = (view.height() * scale).ceil() as i32;
        let (x, y) = panel_origin(capsule, (width, height), scale);
        let _ = unsafe {
            SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                width,
                height,
                SWP_NOACTIVATE,
            )
        };

        let hits = view.hits();
        let hover = self.panel_hover;
        let palette = theme::palette(&self.settings.theme);
        let dpi = unsafe { GetDpiForWindow(capsule) };
        let _ = self.gfx.present(hwnd, (width, height), dpi, 255, |canvas| {
            panel::draw(canvas, &view, &hits, hover, &palette)
        });
        self.hits = hits;
    }

    fn panel_cursor(&mut self) {
        let shape = if self.panel_hover.is_some() {
            IDC_HAND
        } else {
            IDC_ARROW
        };
        if let Ok(cursor) = unsafe { LoadCursorW(None, shape) } {
            unsafe { SetCursor(Some(cursor)) };
        }
    }

    fn hit_at(&self, hwnd: HWND, lp: LPARAM) -> Option<usize> {
        let (x, y) = dip_position(hwnd, lp);
        self.hits.iter().position(|hit| hit.rect.contains(x, y))
    }

    fn panel_mouse_move(&mut self, hwnd: HWND, lp: LPARAM) {
        if !self.panel_tracking {
            self.panel_tracking = true;
            track_leave(hwnd);
        }
        let hover = self.hit_at(hwnd, lp);
        if hover != self.panel_hover {
            self.panel_hover = hover;
            self.redraw_panel();
        }
    }

    fn panel_mouse_leave(&mut self) {
        self.panel_tracking = false;
        self.panel_hover = None;
        self.redraw_panel();
    }

    fn panel_click(&mut self, hwnd: HWND, lp: LPARAM) {
        let Some(index) = self.hit_at(hwnd, lp) else {
            return;
        };
        match self.hits[index].action.clone() {
            Action::Toggle(id) => {
                if let Some(limit) = self.limits().iter().find(|l| l.id == id) {
                    self.settings.toggle(limit);
                    self.settings.save();
                }
                self.redraw();
            }
            Action::Page(page) => {
                self.page = page;
                self.panel_hover = None;
                self.redraw_panel();
            }
            Action::Theme(id) => {
                self.settings.theme = id.to_string();
                self.settings.save();
                self.redraw();
            }
            Action::Lang(lang) => {
                self.settings.lang = Some(lang);
                self.settings.save();
                self.redraw();
            }
            Action::Monitor(index) => {
                // A position on one taskbar means nothing on another.
                self.settings.monitor = Some(index);
                self.settings.offset_x = None;
                self.settings.save();
                self.ensure_capsule();
            }
            Action::Compact => {
                self.settings.compact = !self.settings.compact;
                self.settings.save();
                self.redraw();
            }
            Action::Alerts => {
                self.settings.alerts = !self.settings.alerts;
                self.settings.save();
                self.redraw_panel();
            }
            Action::AlertAt(level) => {
                self.settings.alert_at = level;
                self.settings.save();
                self.redraw_panel();
            }
            Action::Startup => {
                startup::set_enabled(!startup::is_enabled());
                self.redraw_panel();
            }
            Action::Quit => {
                let _ = unsafe { DestroyWindow(self.host) };
            }
        }
    }
}

/// Above the capsule, kept on screen, on whichever edge the taskbar sits.
fn panel_origin(capsule: HWND, (width, height): (i32, i32), scale: f32) -> (i32, i32) {
    let anchor = window_rect(capsule);
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    let monitor = unsafe { MonitorFromWindow(capsule, MONITOR_DEFAULTTONEAREST) };
    let _ = unsafe { GetMonitorInfoW(monitor, &mut info) };
    let work = info.rcWork;
    let gap = (panel::MARGIN * scale).round() as i32;
    let x = anchor
        .left
        .clamp(work.left + gap, (work.right - width - gap).max(work.left));
    let y = if anchor.top >= work.bottom - gap {
        work.bottom - height - gap
    } else {
        work.top + gap
    };
    (x, y)
}
