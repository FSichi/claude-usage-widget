//! Living inside the Windows taskbar: finding it and becoming its child.

use windows::core::w;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GetClientRect, GetParent, GetWindowLongW, IsWindow, SetParent, SetWindowLongW,
    SetWindowPos, GWL_EXSTYLE, GWL_STYLE, HWND_TOP, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, WS_CHILD, WS_CLIPSIBLINGS, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};

pub fn find() -> Option<HWND> {
    unsafe { FindWindowW(w!("Shell_TrayWnd"), None) }
        .ok()
        .filter(|hwnd| !hwnd.0.is_null())
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

pub fn embed(hwnd: HWND, taskbar: HWND) {
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

        let _ = SetParent(hwnd, Some(taskbar));
        // Windows 11 can leave a reparented layered surface beneath the
        // taskbar's composition visual; resetting the style rebinds it.
        if GetParent(hwnd).ok() == Some(taskbar) {
            let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style & !(WS_EX_LAYERED.0 as i32));
            SetWindowLongW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED.0 as i32);
        }
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOP),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
    }
}
