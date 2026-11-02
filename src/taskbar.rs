//! Living inside the Windows taskbar: finding it and becoming its child.

use windows::core::{Result, BOOL};
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetClientRect, GetParent, GetWindowLongW, GetWindowRect, IsWindow,
    SetParent, SetWindowLongW, SetWindowPos, GWL_EXSTYLE, GWL_STYLE, HWND_TOP, SWP_FRAMECHANGED,
    SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_CHILD, WS_CLIPSIBLINGS, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const PRIMARY_CLASS: &str = "Shell_TrayWnd";
const SECONDARY_CLASS: &str = "Shell_SecondaryTrayWnd";

struct Taskbar {
    hwnd: HWND,
    left: i32,
    primary: bool,
}

unsafe extern "system" fn collect(hwnd: HWND, found: LPARAM) -> BOOL {
    let bars = &mut *(found.0 as *mut Vec<Taskbar>);
    let mut class = [0u16; 32];
    let length = GetClassNameW(hwnd, &mut class) as usize;
    let name = String::from_utf16_lossy(&class[..length]);
    let mut rect = RECT::default();
    if (name == PRIMARY_CLASS || name == SECONDARY_CLASS) && GetWindowRect(hwnd, &mut rect).is_ok()
    {
        bars.push(Taskbar {
            hwnd,
            left: rect.left,
            primary: name == PRIMARY_CLASS,
        });
    }
    BOOL(1)
}

fn taskbars() -> Vec<Taskbar> {
    let mut bars: Vec<Taskbar> = Vec::new();
    let found = LPARAM(&mut bars as *mut Vec<Taskbar> as isize);
    let _ = unsafe { EnumWindows(Some(collect), found) };
    bars.sort_by_key(|bar| bar.left);
    bars
}

/// How many monitors show a taskbar, numbered from left to right.
pub fn count() -> usize {
    taskbars().len()
}

/// The chosen monitor's taskbar, or the main one when it has none.
pub fn find(chosen: Option<usize>) -> Option<HWND> {
    let bars = taskbars();
    chosen
        .and_then(|index| bars.get(index))
        .or_else(|| bars.iter().find(|bar| bar.primary))
        .map(|bar| bar.hwnd)
}

/// Where `taskbar` falls in that left-to-right numbering.
pub fn index_of(taskbar: HWND) -> Option<usize> {
    taskbars().iter().position(|bar| bar.hwnd == taskbar)
}

pub fn is_alive(hwnd: HWND) -> bool {
    unsafe { IsWindow(Some(hwnd)) }.as_bool()
}

/// The taskbar's size, in the coordinates its children are placed in.
pub fn size(taskbar: HWND) -> Option<(i32, i32)> {
    let mut rect = RECT::default();
    unsafe { GetClientRect(taskbar, &mut rect) }.ok()?;
    Some((rect.right - rect.left, rect.bottom - rect.top))
}

pub fn is_child_of(hwnd: HWND, taskbar: HWND) -> bool {
    unsafe { GetParent(hwnd) }.ok() == Some(taskbar)
}

/// Fails now and then for a process the Start menu has just launched, and works
/// when tried again, so the caller keeps trying.
pub fn embed(hwnd: HWND, taskbar: HWND) -> Result<()> {
    unsafe {
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        SetWindowLongW(
            hwnd,
            GWL_EXSTYLE,
            (ex_style | WS_EX_TOOLWINDOW.0 as i32 | WS_EX_NOACTIVATE.0 as i32)
                & !(WS_EX_TOPMOST.0 as i32),
        );
        let style = GetWindowLongW(hwnd, GWL_STYLE) as u32;
        SetWindowLongW(
            hwnd,
            GWL_STYLE,
            ((style & !WS_POPUP.0) | WS_CHILD.0 | WS_CLIPSIBLINGS.0) as i32,
        );

        SetParent(hwnd, Some(taskbar))?;
        // Windows 11 can leave a reparented layered surface beneath the
        // taskbar's composition visual; resetting the style rebinds it.
        let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style & !(WS_EX_LAYERED.0 as i32));
        SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED.0 as i32);
        SetWindowPos(
            hwnd,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        )
    }
}
