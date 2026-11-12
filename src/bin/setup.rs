#![windows_subsystem = "windows"]

use modern_inverted_cursor::{
    app_paths, app_paths_for_root, balanced_cursor_size, current_accessibility_step,
    draw_cursor_glyph, export_backup, find_tray_window, load_saved_preset, parse_preset_arg,
    prepare_assets_parallel, reg_delete_value, register_and_apply, register_uninstall_entry,
    restore_backup, save_preset, set_startup_enabled, stop_any_running_guardian,
    unregister_uninstall_entry, wide_z, write_guardian_binary, BoundedLogger, Preset,
    ACCESSIBILITY_STEPS, APP_VERSION, CONFIG_SUBKEY, GUARDIAN_EXE_NAME, GUARDIAN_RUN_VALUE,
    PACK_VERSION, RUN_SUBKEY,
};

use std::{
    env,
    ffi::c_void,
    fs,
    io::Write,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

const GUARDIAN_EXE_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/embedded_guardian.exe"));
const WND_CLASS: &str = "ModernInvertedDarkSetupWnd";

const LOGICAL_W: i32 = 620;
const LOGICAL_H: i32 = 540;

const WM_DESTROY: u32 = 0x0002;
const WM_PAINT: u32 = 0x000F;
const WM_CLOSE: u32 = 0x0010;
const WM_ERASEBKGND: u32 = 0x0014;
const WM_SETCURSOR: u32 = 0x0020;
const WM_SETICON: u32 = 0x0080;
const WM_KEYDOWN: u32 = 0x0100;
const WM_TIMER: u32 = 0x0113;
const WM_MOUSEMOVE: u32 = 0x0200;
const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_MOUSELEAVE: u32 = 0x02A3;
const WM_DPICHANGED: u32 = 0x02E0;
const WM_APP: u32 = 0x8000;
const WM_APP_WORKER_UPDATE: u32 = WM_APP + 10;

const VK_RETURN: usize = 0x0D;
const VK_ESCAPE: usize = 0x1B;

const TIMER_ANIM: usize = 1;
const GWLP_USERDATA: i32 = -21;

const WS_OVERLAPPEDWINDOW_FIXED: u32 = 0x00CA_0000; // WS_OVERLAPPED|WS_CAPTION|WS_SYSMENU|WS_MINIMIZEBOX
const WS_VISIBLE: u32 = 0x1000_0000;
const SWP_NOZORDER: u32 = 0x0004;
const SWP_NOMOVE: u32 = 0x0002;

const TME_LEAVE: u32 = 0x0000_0002;
const IDC_ARROW: usize = 32512;
const IDC_HAND: usize = 32649;
const MONITOR_DEFAULTTONEAREST: u32 = 2;

const DT_LEFT: u32 = 0x0000;
const DT_CENTER: u32 = 0x0001;
const DT_RIGHT: u32 = 0x0002;
const DT_VCENTER: u32 = 0x0004;
const DT_WORDBREAK: u32 = 0x0010;
const DT_SINGLELINE: u32 = 0x0020;
const DT_NOPREFIX: u32 = 0x0800;
const DT_PATH_ELLIPSIS: u32 = 0x4000;

const SRCCOPY: u32 = 0x00CC_0020;
const TRANSPARENT: i32 = 1;
const CLEARTYPE_QUALITY: u32 = 5;
const DEFAULT_CHARSET: u32 = 1;
const FW_NORMAL: i32 = 400;
const FW_SEMIBOLD: i32 = 600;
const FW_BOLD: i32 = 700;

const SW_SHOW: i32 = 5;

const MB_YESNO: u32 = 0x0004;
const MB_ICONQUESTION: u32 = 0x0020;
const MB_SETFOREGROUND: u32 = 0x0001_0000;
const IDYES: i32 = 6;

type WndProc = unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize;

#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct Msg {
    hwnd: *mut c_void,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    pt: Point,
}

#[repr(C)]
struct PaintStruct {
    hdc: *mut c_void,
    f_erase: i32,
    rc_paint: Rect,
    f_restore: i32,
    f_inc_update: i32,
    rgb_reserved: [u8; 32],
}

#[repr(C)]
struct TrackMouseEvent {
    cb_size: u32,
    dw_flags: u32,
    hwnd_track: *mut c_void,
    dw_hover_time: u32,
}

#[repr(C)]
struct MonitorInfo {
    cb_size: u32,
    rc_monitor: Rect,
    rc_work: Rect,
    dw_flags: u32,
}

#[repr(C)]
struct WndClassExW {
    cb_size: u32,
    style: u32,
    lpfn_wnd_proc: WndProc,
    cb_cls_extra: i32,
    cb_wnd_extra: i32,
    h_instance: *mut c_void,
    h_icon: *mut c_void,
    h_cursor: *mut c_void,
    hbr_background: *mut c_void,
    lpsz_menu_name: *const u16,
    lpsz_class_name: *const u16,
    h_icon_sm: *mut c_void,
}

#[repr(C)]
struct BitmapInfoHeader {
    bi_size: u32,
    bi_width: i32,
    bi_height: i32,
    bi_planes: u16,
    bi_bit_count: u16,
    bi_compression: u32,
    bi_size_image: u32,
    bi_x_pels_per_meter: i32,
    bi_y_pels_per_meter: i32,
    bi_clr_used: u32,
    bi_clr_important: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lp_module_name: *const u16) -> *mut c_void;
    fn LoadLibraryW(lp_lib_file_name: *const u16) -> *mut c_void;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const u8) -> *mut c_void;
}

#[link(name = "user32")]
extern "system" {
    fn RegisterClassExW(lpwcx: *const WndClassExW) -> u16;
    fn CreateWindowExW(
        dw_ex_style: u32,
        lp_class_name: *const u16,
        lp_window_name: *const u16,
        dw_style: u32,
        x: i32,
        y: i32,
        n_width: i32,
        n_height: i32,
        h_wnd_parent: *mut c_void,
        h_menu: *mut c_void,
        h_instance: *mut c_void,
        lp_param: *mut c_void,
    ) -> *mut c_void;
    fn DefWindowProcW(h_wnd: *mut c_void, msg: u32, w_param: usize, l_param: isize) -> isize;
    fn DestroyWindow(h_wnd: *mut c_void) -> i32;
    fn PostQuitMessage(n_exit_code: i32);
    fn PostMessageW(h_wnd: *mut c_void, msg: u32, w_param: usize, l_param: isize) -> i32;
    fn SendMessageW(h_wnd: *mut c_void, msg: u32, w_param: usize, l_param: isize) -> isize;
    fn GetMessageW(
        lp_msg: *mut Msg,
        h_wnd: *mut c_void,
        w_msg_filter_min: u32,
        w_msg_filter_max: u32,
    ) -> i32;
    fn TranslateMessage(lp_msg: *const Msg) -> i32;
    fn DispatchMessageW(lp_msg: *const Msg) -> isize;
    fn InvalidateRect(h_wnd: *mut c_void, lp_rect: *const Rect, b_erase: i32) -> i32;
    fn BeginPaint(h_wnd: *mut c_void, lp_paint: *mut PaintStruct) -> *mut c_void;
    fn EndPaint(h_wnd: *mut c_void, lp_paint: *const PaintStruct) -> i32;
    fn GetDC(h_wnd: *mut c_void) -> *mut c_void;
    fn ReleaseDC(h_wnd: *mut c_void, h_dc: *mut c_void) -> i32;
    #[cfg(target_pointer_width = "64")]
    fn SetWindowLongPtrW(h_wnd: *mut c_void, n_index: i32, dw_new_long: isize) -> isize;
    #[cfg(target_pointer_width = "64")]
    fn GetWindowLongPtrW(h_wnd: *mut c_void, n_index: i32) -> isize;
    #[cfg(target_pointer_width = "32")]
    fn SetWindowLongW(h_wnd: *mut c_void, n_index: i32, dw_new_long: isize) -> isize;
    #[cfg(target_pointer_width = "32")]
    fn GetWindowLongW(h_wnd: *mut c_void, n_index: i32) -> isize;
    fn SetWindowPos(
        h_wnd: *mut c_void,
        h_wnd_insert_after: *mut c_void,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        u_flags: u32,
    ) -> i32;
    fn AdjustWindowRectEx(lp_rect: *mut Rect, dw_style: u32, b_menu: i32, dw_ex_style: u32) -> i32;
    fn MonitorFromWindow(hwnd: *mut c_void, dw_flags: u32) -> *mut c_void;
    fn GetMonitorInfoW(h_monitor: *mut c_void, lpmi: *mut MonitorInfo) -> i32;
    fn LoadCursorW(h_instance: *mut c_void, lp_cursor_name: *const u16) -> *mut c_void;
    fn SetCursor(h_cursor: *mut c_void) -> *mut c_void;
    fn TrackMouseEvent(lp_event_track: *mut TrackMouseEvent) -> i32;
    fn SetTimer(
        h_wnd: *mut c_void,
        n_id_event: usize,
        u_elapse: u32,
        lp_timer_func: *mut c_void,
    ) -> usize;
    fn KillTimer(h_wnd: *mut c_void, u_id_event: usize) -> i32;
    fn DrawTextW(
        hdc: *mut c_void,
        lpch_text: *const u16,
        cch_text: i32,
        lprc: *mut Rect,
        format: u32,
    ) -> i32;
    fn DestroyIcon(h_icon: *mut c_void) -> i32;
    fn MessageBoxW(
        h_wnd: *mut c_void,
        lp_text: *const u16,
        lp_caption: *const u16,
        u_type: u32,
    ) -> i32;
}

#[cfg(target_pointer_width = "32")]
#[allow(non_snake_case)]
#[inline]
unsafe fn SetWindowLongPtrW(h_wnd: *mut c_void, n_index: i32, dw_new_long: isize) -> isize {
    SetWindowLongW(h_wnd, n_index, dw_new_long)
}

#[cfg(target_pointer_width = "32")]
#[allow(non_snake_case)]
#[inline]
unsafe fn GetWindowLongPtrW(h_wnd: *mut c_void, n_index: i32) -> isize {
    GetWindowLongW(h_wnd, n_index)
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateCompatibleDC(hdc: *mut c_void) -> *mut c_void;
    fn DeleteDC(hdc: *mut c_void) -> i32;
    fn CreateDIBSection(
        hdc: *mut c_void,
        pbmi: *const BitmapInfoHeader,
        usage: u32,
        ppv_bits: *mut *mut c_void,
        h_section: *mut c_void,
        offset: u32,
    ) -> *mut c_void;
    fn SelectObject(hdc: *mut c_void, h: *mut c_void) -> *mut c_void;
    fn DeleteObject(ho: *mut c_void) -> i32;
    fn BitBlt(
        hdc: *mut c_void,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        hdc_src: *mut c_void,
        x1: i32,
        y1: i32,
        rop: u32,
    ) -> i32;
    fn CreateFontW(
        c_height: i32,
        c_width: i32,
        c_escapement: i32,
        c_orientation: i32,
        c_weight: i32,
        b_italic: u32,
        b_underline: u32,
        b_strike_out: u32,
        i_char_set: u32,
        i_out_precision: u32,
        i_clip_precision: u32,
        i_quality: u32,
        i_pitch_and_family: u32,
        psz_face_name: *const u16,
    ) -> *mut c_void;
    fn SetBkMode(hdc: *mut c_void, mode: i32) -> i32;
    fn SetTextColor(hdc: *mut c_void, color: u32) -> u32;
    fn GdiFlush() -> i32;
    fn GetDeviceCaps(hdc: *mut c_void, index: i32) -> i32;
}

#[link(name = "shell32")]
extern "system" {
    fn ShellExecuteW(
        hwnd: *mut c_void,
        lp_operation: *const u16,
        lp_file: *const u16,
        lp_parameters: *const u16,
        lp_directory: *const u16,
        n_show_cmd: i32,
    ) -> isize;
}

// Dynamic Win7–Win11 DPI + DWM Dark Mode helpers (zero extra .lib requirements)
unsafe fn enable_per_monitor_dpi_v2() {
    let u32_mod = LoadLibraryW(wide_z("user32.dll").as_ptr());
    if !u32_mod.is_null() {
        type SetDpiCtxFn = unsafe extern "system" fn(isize) -> i32;
        let p = GetProcAddress(u32_mod, b"SetProcessDpiAwarenessContext\0".as_ptr());
        if !p.is_null() {
            let f: SetDpiCtxFn = std::mem::transmute(p);
            if f(-4isize) != 0 {
                return;
            }
        }
    }
    let shcore = LoadLibraryW(wide_z("shcore.dll").as_ptr());
    if !shcore.is_null() {
        type SetDpiAwarenessFn = unsafe extern "system" fn(i32) -> i32;
        let p = GetProcAddress(shcore, b"SetProcessDpiAwareness\0".as_ptr());
        if !p.is_null() {
            let f: SetDpiAwarenessFn = std::mem::transmute(p);
            if f(2) == 0 {
                return;
            }
        }
    }
    if !u32_mod.is_null() {
        type SetDpiAwareFn = unsafe extern "system" fn() -> i32;
        let p = GetProcAddress(u32_mod, b"SetProcessDPIAware\0".as_ptr());
        if !p.is_null() {
            let f: SetDpiAwareFn = std::mem::transmute(p);
            let _ = f();
        }
    }
}

unsafe fn window_dpi(hwnd: *mut c_void) -> u32 {
    if !hwnd.is_null() {
        let u32_mod = LoadLibraryW(wide_z("user32.dll").as_ptr());
        if !u32_mod.is_null() {
            type GetDpiFn = unsafe extern "system" fn(*mut c_void) -> u32;
            let p = GetProcAddress(u32_mod, b"GetDpiForWindow\0".as_ptr());
            if !p.is_null() {
                let f: GetDpiFn = std::mem::transmute(p);
                let dpi = f(hwnd);
                if dpi >= 96 {
                    return dpi;
                }
            }
        }
    }
    // Windows 7 / 8 / 8.1 fallback via GDI LOGPIXELSX (88)
    let hdc = GetDC(hwnd);
    if !hdc.is_null() {
        let dpi = GetDeviceCaps(hdc, 88);
        ReleaseDC(hwnd, hdc);
        if dpi >= 96 {
            return dpi as u32;
        }
    }
    96
}

unsafe fn adjust_rect_for_dpi(rc: &mut Rect, style: u32, dpi: u32) {
    let u32_mod = LoadLibraryW(wide_z("user32.dll").as_ptr());
    if !u32_mod.is_null() {
        type AdjDpiFn = unsafe extern "system" fn(*mut Rect, u32, i32, u32, u32) -> i32;
        let p = GetProcAddress(u32_mod, b"AdjustWindowRectExForDpi\0".as_ptr());
        if !p.is_null() {
            let f: AdjDpiFn = std::mem::transmute(p);
            if f(rc as *mut Rect, style, 0, 0, dpi) != 0 {
                return;
            }
        }
    }
    AdjustWindowRectEx(rc as *mut Rect, style, 0, 0);
}

unsafe fn apply_dwm_dark_chrome(hwnd: *mut c_void) -> bool {
    let dwm = LoadLibraryW(wide_z("dwmapi.dll").as_ptr());
    if dwm.is_null() {
        return false;
    }
    type DwmSetAttrFn = unsafe extern "system" fn(*mut c_void, u32, *const c_void, u32) -> i32;
    let proc = GetProcAddress(dwm, b"DwmSetWindowAttribute\0".as_ptr());
    if proc.is_null() {
        return false;
    }
    let set_attr: DwmSetAttrFn = std::mem::transmute(proc);
    let dark: i32 = 1;
    // Try Win10 20H1+/Win11 attribute 20, then Win10 1809–1909 attribute 19 (Win7/8 ignore cleanly).
    let mut r1 = set_attr(hwnd, 20, &dark as *const i32 as *const c_void, 4);
    if r1 != 0 {
        r1 = set_attr(hwnd, 19, &dark as *const i32 as *const c_void, 4);
    }
    let round: i32 = 2; // DWMWCP_ROUND (Win11)
    let _ = set_attr(hwnd, 33, &round as *const i32 as *const c_void, 4);
    let border: u32 = 0x003D_312B; // COLORREF BGR for #2B313D (Win11)
    let _ = set_attr(hwnd, 34, &border as *const u32 as *const c_void, 4);
    r1 == 0
}

fn s(dip: i32, dpi: u32) -> i32 {
    (dip * dpi as i32 + 48) / 96
}

fn colorref(rgb: u32) -> u32 {
    let r = (rgb >> 16) & 0xFF;
    let g = (rgb >> 8) & 0xFF;
    let b = rgb & 0xFF;
    (b << 16) | (g << 8) | r
}

fn blend_px(bg: u32, fg: u32, alpha: f32) -> u32 {
    let a = alpha.clamp(0.0, 1.0);
    let inv = 1.0 - a;
    let r = (((fg >> 16) & 0xFF) as f32 * a + ((bg >> 16) & 0xFF) as f32 * inv).round() as u32;
    let g = (((fg >> 8) & 0xFF) as f32 * a + ((bg >> 8) & 0xFF) as f32 * inv).round() as u32;
    let b = ((fg & 0xFF) as f32 * a + (bg & 0xFF) as f32 * inv).round() as u32;
    0xFF00_0000 | (r.min(255) << 16) | (g.min(255) << 8) | b.min(255)
}

fn sdf_round_box(px: f32, py: f32, cx: f32, cy: f32, hw: f32, hh: f32, r: f32) -> f32 {
    let qx = (px - cx).abs() - (hw - r).max(0.0);
    let qy = (py - cy).abs() - (hh - r).max(0.0);
    let out_len = (qx.max(0.0) * qx.max(0.0) + qy.max(0.0) * qy.max(0.0)).sqrt();
    let in_len = qx.max(qy).min(0.0);
    out_len + in_len - r
}

fn fill_round_rect(
    buf: &mut [u32],
    w: usize,
    h: usize,
    x: i32,
    y: i32,
    rw: i32,
    rh: i32,
    radius: f32,
    fill_rgb: u32,
    border_rgb: u32,
    border_px: f32,
) {
    if rw <= 0 || rh <= 0 {
        return;
    }
    let hw = rw as f32 * 0.5;
    let hh = rh as f32 * 0.5;
    let cx = x as f32 + hw;
    let cy = y as f32 + hh;
    let r = radius.min(hw).min(hh);

    let x0 = (x - 2).max(0) as usize;
    let y0 = (y - 2).max(0) as usize;
    let x1 = (x + rw + 2).min(w as i32) as usize;
    let y1 = (y + rh + 2).min(h as i32) as usize;

    for py in y0..y1 {
        let row = py * w;
        for px in x0..x1 {
            let d = sdf_round_box(px as f32 + 0.5, py as f32 + 0.5, cx, cy, hw, hh, r);
            if d >= 0.75 {
                continue;
            }
            let cov = (0.75 - d).clamp(0.0, 1.0);
            let col = if border_px > 0.0 && d > -border_px - 0.5 {
                let border_mix = ((d + border_px + 0.5) / 1.0).clamp(0.0, 1.0);
                blend_px(fill_rgb, border_rgb, border_mix)
            } else {
                0xFF00_0000 | fill_rgb
            };
            buf[row + px] = blend_px(buf[row + px], col, cov);
        }
    }
}

fn fill_circle_aa(buf: &mut [u32], w: usize, h: usize, cx: f32, cy: f32, radius: f32, fill: u32, border: Option<(u32, f32)>) {
    let x0 = (cx - radius - 2.0).floor().max(0.0) as usize;
    let y0 = (cy - radius - 2.0).floor().max(0.0) as usize;
    let x1 = (cx + radius + 2.0).ceil().min(w as f32) as usize;
    let y1 = (cy + radius + 2.0).ceil().min(h as f32) as usize;
    for py in y0..y1 {
        let row = py * w;
        for px in x0..x1 {
            let dx = px as f32 + 0.5 - cx;
            let dy = py as f32 + 0.5 - cy;
            let d = (dx * dx + dy * dy).sqrt() - radius;
            if d >= 0.75 {
                continue;
            }
            let cov = (0.75 - d).clamp(0.0, 1.0);
            let col = if let Some((bcol, bpx)) = border {
                if d > -bpx - 0.5 {
                    let mix = (d + bpx + 0.5).clamp(0.0, 1.0);
                    blend_px(fill, bcol, mix)
                } else {
                    0xFF00_0000 | fill
                }
            } else {
                0xFF00_0000 | fill
            };
            buf[row + px] = blend_px(buf[row + px], col, cov);
        }
    }
}

fn draw_line_aa(buf: &mut [u32], w: usize, h: usize, ax: f32, ay: f32, bx: f32, by: f32, half_thick: f32, col: u32) {
    let x0 = (ax.min(bx) - half_thick - 2.0).floor().max(0.0) as usize;
    let y0 = (ay.min(by) - half_thick - 2.0).floor().max(0.0) as usize;
    let x1 = (ax.max(bx) + half_thick + 2.0).ceil().min(w as f32) as usize;
    let y1 = (ay.max(by) + half_thick + 2.0).ceil().min(h as f32) as usize;
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    for py in y0..y1 {
        let row = py * w;
        for px in x0..x1 {
            let fx = px as f32 + 0.5;
            let fy = py as f32 + 0.5;
            let t = if len_sq > 0.0 {
                (((fx - ax) * dx + (fy - ay) * dy) / len_sq).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let cx = ax + t * dx;
            let cy = ay + t * dy;
            let d = ((fx - cx) * (fx - cx) + (fy - cy) * (fy - cy)).sqrt() - half_thick;
            if d < 0.75 {
                let cov = (0.75 - d).clamp(0.0, 1.0);
                buf[row + px] = blend_px(buf[row + px], col, cov);
            }
        }
    }
}

fn fill_hline(buf: &mut [u32], w: usize, h: usize, x0: i32, x1: i32, y: i32, col: u32) {
    if y < 0 || y >= h as i32 {
        return;
    }
    let row = y as usize * w;
    let s = x0.max(0) as usize;
    let e = x1.min(w as i32) as usize;
    for px in s..e {
        buf[row + px] = 0xFF00_0000 | col;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ControlId {
    PresetCard(usize),
    ToggleStartup,
    ToggleUninstall,
    BtnInstall,
    BtnRestore,
    BtnOpenFolder,
    BtnFinish,
}

#[derive(Clone)]
struct HitZone {
    id: ControlId,
    rect: Rect,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Setup,
    Working,
    Done,
}

#[derive(Clone)]
struct WorkerUpdate {
    pct: f32,
    stage: usize, // 0..4 checklist step
    status: String,
    finished: bool,
    is_restore: bool,
    error: Option<String>,
    applied_step: u32,
    applied_px: u32,
    install_root: PathBuf,
}

struct FontSet {
    title: *mut c_void,
    section: *mut c_void,
    card_title: *mut c_void,
    body: *mut c_void,
    small: *mut c_void,
    btn: *mut c_void,
}

impl FontSet {
    unsafe fn create(dpi: u32) -> Self {
        let make = |dip: i32, weight: i32| -> *mut c_void {
            let face = wide_z("Segoe UI");
            CreateFontW(
                -s(dip, dpi),
                0,
                0,
                0,
                weight,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                0,
                0,
                CLEARTYPE_QUALITY,
                0,
                face.as_ptr(),
            )
        };
        Self {
            title: make(20, FW_BOLD),
            section: make(11, FW_BOLD),
            card_title: make(15, FW_SEMIBOLD),
            body: make(13, FW_NORMAL),
            small: make(12, FW_NORMAL),
            btn: make(14, FW_SEMIBOLD),
        }
    }

    unsafe fn destroy(&mut self) {
        for f in [
            self.title,
            self.section,
            self.card_title,
            self.body,
            self.small,
            self.btn,
        ] {
            if !f.is_null() {
                DeleteObject(f);
            }
        }
    }
}

struct Backbuffer {
    hdc: *mut c_void,
    hbm: *mut c_void,
    bits: *mut u32,
    w: usize,
    h: usize,
}

impl Backbuffer {
    unsafe fn create(ref_dc: *mut c_void, w: usize, h: usize) -> Option<Self> {
        let hdc = CreateCompatibleDC(ref_dc);
        if hdc.is_null() {
            return None;
        }
        let bmi = BitmapInfoHeader {
            bi_size: std::mem::size_of::<BitmapInfoHeader>() as u32,
            bi_width: w as i32,
            bi_height: -(h as i32),
            bi_planes: 1,
            bi_bit_count: 32,
            bi_compression: 0,
            bi_size_image: 0,
            bi_x_pels_per_meter: 0,
            bi_y_pels_per_meter: 0,
            bi_clr_used: 0,
            bi_clr_important: 0,
        };
        let mut raw: *mut c_void = null_mut();
        let hbm = CreateDIBSection(hdc, &bmi, 0, &mut raw, null_mut(), 0);
        if hbm.is_null() || raw.is_null() {
            DeleteDC(hdc);
            return None;
        }
        SelectObject(hdc, hbm);
        Some(Self {
            hdc,
            hbm,
            bits: raw as *mut u32,
            w,
            h,
        })
    }

    unsafe fn pixels_mut(&mut self) -> &mut [u32] {
        std::slice::from_raw_parts_mut(self.bits, self.w * self.h)
    }

    unsafe fn destroy(&mut self) {
        if !self.hbm.is_null() {
            DeleteObject(self.hbm);
            self.hbm = null_mut();
        }
        if !self.hdc.is_null() {
            DeleteDC(self.hdc);
            self.hdc = null_mut();
        }
    }
}

struct GuiState {
    hwnd: *mut c_void,
    dpi: u32,
    screen: Screen,
    preset: Preset,
    startup: bool,
    register_uninstall: bool,
    has_existing_install: bool,
    current_step: u32,
    current_px: u32,
    hover: Option<ControlId>,
    pressed: Option<ControlId>,
    tracking_mouse: bool,
    hits: Vec<HitZone>,
    anim_pct: f32,
    target_pct: f32,
    worker_stage: usize,
    worker_status: String,
    worker_error: Option<String>,
    is_restore: bool,
    install_root: PathBuf,
    shared_update: Arc<Mutex<WorkerUpdate>>,
    fonts: FontSet,
    backbuffer: Option<Backbuffer>,
    app_icon: *mut c_void,
}

unsafe fn draw_text(
    hdc: *mut c_void,
    font: *mut c_void,
    rgb: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    flags: u32,
    text: &str,
) {
    SelectObject(hdc, font);
    SetTextColor(hdc, colorref(rgb));
    SetBkMode(hdc, TRANSPARENT);
    let wide = wide_z(text);
    let mut rc = Rect {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    };
    DrawTextW(
        hdc,
        wide.as_ptr(),
        (wide.len() - 1) as i32,
        &mut rc,
        flags | DT_NOPREFIX,
    );
}

fn draw_toggle_switch(
    buf: &mut [u32],
    bw: usize,
    bh: usize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    on: bool,
    hovered: bool,
) {
    let track_fill = if on {
        if hovered {
            0x3B82F6
        } else {
            0x2563EB
        }
    } else if hovered {
        0x364152
    } else {
        0x2A3241
    };
    let track_border = if on { 0x60A5FA } else { 0x475569 };
    let r = h as f32 * 0.5;
    fill_round_rect(buf, bw, bh, x, y, w, h, r, track_fill, track_border, 1.0);

    let knob_r = r - 3.0;
    let kx = if on {
        x as f32 + w as f32 - r
    } else {
        x as f32 + r
    };
    let ky = y as f32 + r;
    fill_circle_aa(buf, bw, bh, kx, ky, knob_r, 0xF8FAFC, None);
}

fn render_ui(state: &mut GuiState) {
    let dpi = state.dpi;
    let cw = s(LOGICAL_W, dpi) as usize;
    let ch = s(LOGICAL_H, dpi) as usize;

    let need_alloc = match state.backbuffer.as_ref() {
        Some(bb) => bb.w != cw || bb.h != ch,
        None => true,
    };
    if need_alloc {
        if let Some(mut old) = state.backbuffer.take() {
            unsafe {
                old.destroy();
            }
        }
        let dc = unsafe { GetDC(state.hwnd) };
        state.backbuffer = unsafe { Backbuffer::create(dc, cw, ch) };
        unsafe {
            ReleaseDC(state.hwnd, dc);
        }
    }

    let bb = match state.backbuffer.as_mut() {
        Some(b) => b,
        None => return,
    };
    state.hits.clear();

    // 1. Pixel-level SDF / vector background + cards + icons
    {
        let buf = unsafe { bb.pixels_mut() };
        buf.fill(0xFF0E_1117);

        // Header bar background + hairline divider
        let hdr_h = s(84, dpi);
        for row in 0..(hdr_h as usize).min(ch) {
            let start = row * cw;
            buf[start..start + cw].fill(0xFF13_1720);
        }
        fill_hline(buf, cw, ch, 0, cw as i32, hdr_h, 0x232A38);

        // Header icon box + split-contrast vector arrow
        fill_round_rect(
            buf,
            cw,
            ch,
            s(24, dpi),
            s(20, dpi),
            s(44, dpi),
            s(44, dpi),
            s(10, dpi) as f32,
            0x1A2130,
            0x2D374B,
            1.2,
        );
        draw_cursor_glyph(
            buf,
            cw,
            s(29, dpi),
            s(25, dpi),
            s(34, dpi),
            Preset::Regular,
            false,
            false,
        );

        // Version pill
        fill_round_rect(
            buf,
            cw,
            ch,
            s(334, dpi),
            s(21, dpi),
            s(138, dpi),
            s(22, dpi),
            s(11, dpi) as f32,
            0x18253E,
            0x2C4775,
            1.0,
        );

        // Footer background
        let ftr_y = s(460, dpi);
        for row in (ftr_y as usize)..ch {
            let start = row * cw;
            buf[start..start + cw].fill(0xFF12_161E);
        }
        fill_hline(buf, cw, ch, 0, cw as i32, ftr_y, 0x232A38);

        match state.screen {
            Screen::Setup => {
                // Live pointer size pill
                fill_round_rect(
                    buf,
                    cw,
                    ch,
                    s(364, dpi),
                    s(97, dpi),
                    s(232, dpi),
                    s(24, dpi),
                    s(12, dpi) as f32,
                    0x161E2B,
                    0x2A3446,
                    1.0,
                );

                // 2x2 Preset cards
                let card_w = s(279, dpi);
                let card_h = s(88, dpi);
                let col_x = [s(24, dpi), s(317, dpi)];
                let row_y = [s(130, dpi), s(232, dpi)];

                for (idx, preset) in Preset::ALL.iter().enumerate() {
                    let cx = col_x[idx % 2];
                    let cy = row_y[idx / 2];
                    let selected = state.preset == *preset;
                    let hovered = state.hover == Some(ControlId::PresetCard(idx));

                    let (bg, border, bw) = if selected {
                        (0x162235, 0x3B82F6, 1.8)
                    } else if hovered {
                        (0x1B222E, 0x414E66, 1.3)
                    } else {
                        (0x151A23, 0x252D3B, 1.1)
                    };

                    fill_round_rect(
                        buf,
                        cw,
                        ch,
                        cx,
                        cy,
                        card_w,
                        card_h,
                        s(12, dpi) as f32,
                        bg,
                        border,
                        bw,
                    );

                    // Circular preview badge inside card
                    let circle_r = s(23, dpi) as f32;
                    let ccx = cx as f32 + s(36, dpi) as f32;
                    let ccy = cy as f32 + (card_h as f32) * 0.5;
                    fill_circle_aa(
                        buf,
                        cw,
                        ch,
                        ccx,
                        ccy,
                        circle_r,
                        0x0D1117,
                        Some((if selected { 0x2F5AA8 } else { 0x273041 }, 1.1)),
                    );

                    let glyph_sz = s(34, dpi);
                    draw_cursor_glyph(
                        buf,
                        cw,
                        ccx as i32 - glyph_sz / 2,
                        ccy as i32 - glyph_sz / 2,
                        glyph_sz,
                        *preset,
                        false,
                        false,
                    );

                    if *preset == Preset::Compact {
                        fill_round_rect(
                            buf,
                            cw,
                            ch,
                            cx + s(152, dpi),
                            cy + s(15, dpi),
                            s(98, dpi),
                            s(18, dpi),
                            s(9, dpi) as f32,
                            0x132C22,
                            0x1E5E42,
                            1.0,
                        );
                    }

                    state.hits.push(HitZone {
                        id: ControlId::PresetCard(idx),
                        rect: Rect {
                            left: cx,
                            top: cy,
                            right: cx + card_w,
                            bottom: cy + card_h,
                        },
                    });
                }

                // Options card
                let opt_y = s(336, dpi);
                let opt_h = s(108, dpi);
                fill_round_rect(
                    buf,
                    cw,
                    ch,
                    s(24, dpi),
                    opt_y,
                    s(572, dpi),
                    opt_h,
                    s(12, dpi) as f32,
                    0x151A23,
                    0x252D3B,
                    1.1,
                );
                fill_hline(buf, cw, ch, s(40, dpi), s(580, dpi), opt_y + s(54, dpi), 0x222937);

                let sw_x = s(538, dpi);
                let sw_w = s(42, dpi);
                let sw_h = s(22, dpi);
                draw_toggle_switch(
                    buf,
                    cw,
                    ch,
                    sw_x,
                    opt_y + s(16, dpi),
                    sw_w,
                    sw_h,
                    state.startup,
                    state.hover == Some(ControlId::ToggleStartup),
                );
                state.hits.push(HitZone {
                    id: ControlId::ToggleStartup,
                    rect: Rect {
                        left: s(24, dpi),
                        top: opt_y,
                        right: s(596, dpi),
                        bottom: opt_y + s(54, dpi),
                    },
                });

                draw_toggle_switch(
                    buf,
                    cw,
                    ch,
                    sw_x,
                    opt_y + s(70, dpi),
                    sw_w,
                    sw_h,
                    state.register_uninstall,
                    state.hover == Some(ControlId::ToggleUninstall),
                );
                state.hits.push(HitZone {
                    id: ControlId::ToggleUninstall,
                    rect: Rect {
                        left: s(24, dpi),
                        top: opt_y + s(54, dpi),
                        right: s(596, dpi),
                        bottom: opt_y + opt_h,
                    },
                });

                // Footer buttons
                if state.has_existing_install {
                    let hov = state.hover == Some(ControlId::BtnRestore);
                    let prs = state.pressed == Some(ControlId::BtnRestore);
                    let bg = if prs { 0x1A2029 } else if hov { 0x293242 } else { 0x1F2633 };
                    fill_round_rect(
                        buf,
                        cw,
                        ch,
                        s(314, dpi),
                        s(480, dpi),
                        s(132, dpi),
                        s(40, dpi),
                        s(9, dpi) as f32,
                        bg,
                        0x364154,
                        1.1,
                    );
                    state.hits.push(HitZone {
                        id: ControlId::BtnRestore,
                        rect: Rect {
                            left: s(314, dpi),
                            top: s(480, dpi),
                            right: s(446, dpi),
                            bottom: s(520, dpi),
                        },
                    });
                }

                let hov_i = state.hover == Some(ControlId::BtnInstall);
                let prs_i = state.pressed == Some(ControlId::BtnInstall);
                let ibg = if prs_i { 0x1D4ED8 } else if hov_i { 0x3B82F6 } else { 0x2563EB };
                fill_round_rect(
                    buf,
                    cw,
                    ch,
                    s(458, dpi),
                    s(480, dpi),
                    s(138, dpi),
                    s(40, dpi),
                    s(9, dpi) as f32,
                    ibg,
                    0x60A5FA,
                    1.0,
                );
                state.hits.push(HitZone {
                    id: ControlId::BtnInstall,
                    rect: Rect {
                        left: s(458, dpi),
                        top: s(480, dpi),
                        right: s(596, dpi),
                        bottom: s(520, dpi),
                    },
                });
            }
            Screen::Working => {
                fill_round_rect(
                    buf,
                    cw,
                    ch,
                    s(44, dpi),
                    s(118, dpi),
                    s(532, dpi),
                    s(312, dpi),
                    s(14, dpi) as f32,
                    0x151A23,
                    0x252D3B,
                    1.2,
                );

                // Progress bar track
                let bar_x = s(84, dpi);
                let bar_y = s(222, dpi);
                let bar_w = s(452, dpi);
                let bar_h = s(12, dpi);
                fill_round_rect(
                    buf,
                    cw,
                    ch,
                    bar_x,
                    bar_y,
                    bar_w,
                    bar_h,
                    s(6, dpi) as f32,
                    0x0B0F17,
                    0x242D3D,
                    1.0,
                );

                let fill_w = ((bar_w as f32) * state.anim_pct.clamp(0.02, 1.0)).round() as i32;
                if fill_w >= bar_h {
                    fill_round_rect(
                        buf,
                        cw,
                        ch,
                        bar_x,
                        bar_y,
                        fill_w,
                        bar_h,
                        s(6, dpi) as f32,
                        if state.is_restore { 0xF59E0B } else { 0x3B82F6 },
                        if state.is_restore { 0xFBBF24 } else { 0x60A5FA },
                        0.8,
                    );
                }

                // 4 checklist step indicators
                let sc = dpi as f32 / 96.0;
                for i in 0..4usize {
                    let sy = s(276 + i as i32 * 34, dpi) as f32;
                    let sx = s(96, dpi) as f32;
                    let r = s(7, dpi) as f32;
                    if i < state.worker_stage {
                        fill_circle_aa(buf, cw, ch, sx, sy, r, 0x16A34A, Some((0x22C55E, 1.0)));
                        draw_line_aa(buf, cw, ch, sx - 3.2 * sc, sy, sx - 0.8 * sc, sy + 2.6 * sc, 1.1 * sc, 0xFFFF_FFFF);
                        draw_line_aa(buf, cw, ch, sx - 0.8 * sc, sy + 2.6 * sc, sx + 3.6 * sc, sy - 2.6 * sc, 1.1 * sc, 0xFFFF_FFFF);
                    } else if i == state.worker_stage {
                        fill_circle_aa(buf, cw, ch, sx, sy, r, 0x1E3A8A, Some((0x60A5FA, 1.3 * sc)));
                        fill_circle_aa(buf, cw, ch, sx, sy, r * 0.42, 0x93C5FD, None);
                    } else {
                        fill_circle_aa(buf, cw, ch, sx, sy, r, 0x1C222D, Some((0x374151, 1.0)));
                    }
                }
            }
            Screen::Done => {
                fill_round_rect(
                    buf,
                    cw,
                    ch,
                    s(44, dpi),
                    s(112, dpi),
                    s(532, dpi),
                    s(322, dpi),
                    s(14, dpi) as f32,
                    0x151A23,
                    0x252D3B,
                    1.2,
                );

                let sc = dpi as f32 / 96.0;
                let cx = cw as f32 * 0.5;
                let cy = s(156, dpi) as f32;
                let rad = s(24, dpi) as f32;
                if state.worker_error.is_some() {
                    fill_circle_aa(buf, cw, ch, cx, cy, rad, 0x3B181E, Some((0xEF4444, 1.6)));
                    draw_line_aa(buf, cw, ch, cx - 7.5 * sc, cy - 7.5 * sc, cx + 7.5 * sc, cy + 7.5 * sc, 1.8 * sc, 0xFFFF_FFFF);
                    draw_line_aa(buf, cw, ch, cx + 7.5 * sc, cy - 7.5 * sc, cx - 7.5 * sc, cy + 7.5 * sc, 1.8 * sc, 0xFFFF_FFFF);
                } else {
                    fill_circle_aa(buf, cw, ch, cx, cy, rad, 0x132E22, Some((0x22C55E, 1.6)));
                    draw_line_aa(buf, cw, ch, cx - 8.5 * sc, cy + 1.0 * sc, cx - 2.0 * sc, cy + 7.5 * sc, 1.8 * sc, 0xFFFF_FFFF);
                    draw_line_aa(buf, cw, ch, cx - 2.0 * sc, cy + 7.5 * sc, cx + 9.5 * sc, cy - 6.5 * sc, 1.8 * sc, 0xFFFF_FFFF);
                }

                for r_idx in 0..3 {
                    let div_y = s(276 + r_idx * 38, dpi);
                    fill_hline(buf, cw, ch, s(76, dpi), s(544, dpi), div_y, 0x222937);
                }

                // Open folder secondary button
                let hov_f = state.hover == Some(ControlId::BtnOpenFolder);
                let fbg = if hov_f { 0x293242 } else { 0x1F2633 };
                fill_round_rect(
                    buf,
                    cw,
                    ch,
                    s(328, dpi),
                    s(480, dpi),
                    s(128, dpi),
                    s(40, dpi),
                    s(9, dpi) as f32,
                    fbg,
                    0x364154,
                    1.1,
                );
                state.hits.push(HitZone {
                    id: ControlId::BtnOpenFolder,
                    rect: Rect {
                        left: s(328, dpi),
                        top: s(480, dpi),
                        right: s(456, dpi),
                        bottom: s(520, dpi),
                    },
                });

                let hov_done = state.hover == Some(ControlId::BtnFinish);
                let dbg = if hov_done { 0x3B82F6 } else { 0x2563EB };
                fill_round_rect(
                    buf,
                    cw,
                    ch,
                    s(468, dpi),
                    s(480, dpi),
                    s(128, dpi),
                    s(40, dpi),
                    s(9, dpi) as f32,
                    dbg,
                    0x60A5FA,
                    1.0,
                );
                state.hits.push(HitZone {
                    id: ControlId::BtnFinish,
                    rect: Rect {
                        left: s(468, dpi),
                        top: s(480, dpi),
                        right: s(596, dpi),
                        bottom: s(520, dpi),
                    },
                });
            }
        }
    }

    // 2. Crisp ClearType Segoe UI Text via GDI onto the same backbuffer DC
    unsafe {
        let hdc = bb.hdc;
        draw_text(
            hdc,
            state.fonts.title,
            0xF3F6FC,
            s(80, dpi),
            s(18, dpi),
            s(250, dpi),
            s(28, dpi),
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
            "Modern Inverted Cursors",
        );
        draw_text(
            hdc,
            state.fonts.section,
            0x7FB3FF,
            s(334, dpi),
            s(21, dpi),
            s(138, dpi),
            s(22, dpi),
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            &format!("v{APP_VERSION} • Pack {PACK_VERSION}"),
        );
        draw_text(
            hdc,
            state.fonts.body,
            0x8B96A8,
            s(80, dpi),
            s(46, dpi),
            s(516, dpi),
            s(22, dpi),
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
            "Dynamic XOR-inverting Windows 11 pointers + silent system-tray guardian",
        );

        match state.screen {
            Screen::Setup => {
                draw_text(
                    hdc,
                    state.fonts.section,
                    0x7D8799,
                    s(24, dpi),
                    s(98, dpi),
                    s(220, dpi),
                    s(20, dpi),
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                    "CHOOSE CURSOR PRESET",
                );
                draw_text(
                    hdc,
                    state.fonts.small,
                    0x93C5FD,
                    s(364, dpi),
                    s(97, dpi),
                    s(232, dpi),
                    s(24, dpi),
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                    &format!(
                        "Pointer Size: {}/{ACCESSIBILITY_STEPS} → {} px (Balanced)",
                        state.current_step, state.current_px
                    ),
                );

                let card_w = s(279, dpi);
                let col_x = [s(24, dpi), s(317, dpi)];
                let row_y = [s(130, dpi), s(232, dpi)];
                for (idx, preset) in Preset::ALL.iter().enumerate() {
                    let cx = col_x[idx % 2];
                    let cy = row_y[idx / 2];
                    draw_text(
                        hdc,
                        state.fonts.card_title,
                        0xF3F6FC,
                        cx + s(70, dpi),
                        cy + s(13, dpi),
                        s(140, dpi),
                        s(22, dpi),
                        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                        preset.short_name(),
                    );
                    if *preset == Preset::Compact {
                        draw_text(
                            hdc,
                            state.fonts.section,
                            0x4ADE80,
                            cx + s(152, dpi),
                            cy + s(15, dpi),
                            s(98, dpi),
                            s(18, dpi),
                            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                            "RECOMMENDED",
                        );
                    }
                    draw_text(
                        hdc,
                        state.fonts.small,
                        0x8B96A8,
                        cx + s(70, dpi),
                        cy + s(40, dpi),
                        card_w - s(82, dpi),
                        s(36, dpi),
                        DT_LEFT | DT_WORDBREAK,
                        preset.description(),
                    );
                }

                let opt_y = s(336, dpi);
                draw_text(
                    hdc,
                    state.fonts.card_title,
                    0xE5EAF2,
                    s(42, dpi),
                    opt_y + s(9, dpi),
                    s(460, dpi),
                    s(20, dpi),
                    DT_LEFT | DT_SINGLELINE,
                    "Start silently in Show hidden icons tray at Windows sign-in",
                );
                draw_text(
                    hdc,
                    state.fonts.small,
                    0x7D8799,
                    s(42, dpi),
                    opt_y + s(29, dpi),
                    s(460, dpi),
                    s(18, dpi),
                    DT_LEFT | DT_SINGLELINE,
                    "Runs invisibly (~0.5 MB RAM, 0% idle CPU) with no terminal window",
                );
                draw_text(
                    hdc,
                    state.fonts.card_title,
                    0xE5EAF2,
                    s(42, dpi),
                    opt_y + s(63, dpi),
                    s(460, dpi),
                    s(20, dpi),
                    DT_LEFT | DT_SINGLELINE,
                    "Register in Windows Installed Apps",
                );
                draw_text(
                    hdc,
                    state.fonts.small,
                    0x7D8799,
                    s(42, dpi),
                    opt_y + s(83, dpi),
                    s(460, dpi),
                    s(18, dpi),
                    DT_LEFT | DT_SINGLELINE,
                    "Allows one-click uninstall & cursor restoration from Windows Settings",
                );

                draw_text(
                    hdc,
                    state.fonts.small,
                    0x7D8799,
                    s(24, dpi),
                    s(488, dpi),
                    s(280, dpi),
                    s(24, dpi),
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                    &format!("Pure Rust • Safe backup • {} KB guardian", GUARDIAN_EXE_BYTES.len() / 1024),
                );

                if state.has_existing_install {
                    draw_text(
                        hdc,
                        state.fonts.btn,
                        0xD6DEE9,
                        s(314, dpi),
                        s(480, dpi),
                        s(132, dpi),
                        s(40, dpi),
                        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                        "Restore Original",
                    );
                }

                let cta_text = if state.has_existing_install {
                    "Update & Apply"
                } else {
                    "Quick Install"
                };
                draw_text(
                    hdc,
                    state.fonts.btn,
                    0xFFFFFF,
                    s(458, dpi),
                    s(480, dpi),
                    s(138, dpi),
                    s(40, dpi),
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                    cta_text,
                );
            }
            Screen::Working => {
                let heading = if state.is_restore {
                    "Restoring Original Windows Cursors..."
                } else {
                    "Installing Modern Inverted Cursors..."
                };
                draw_text(
                    hdc,
                    state.fonts.title,
                    0xF3F6FC,
                    s(64, dpi),
                    s(146, dpi),
                    s(492, dpi),
                    s(30, dpi),
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                    heading,
                );
                draw_text(
                    hdc,
                    state.fonts.body,
                    0x93C5FD,
                    s(64, dpi),
                    s(184, dpi),
                    s(492, dpi),
                    s(22, dpi),
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                    &state.worker_status,
                );
                draw_text(
                    hdc,
                    state.fonts.small,
                    0x8B96A8,
                    s(84, dpi),
                    s(240, dpi),
                    s(452, dpi),
                    s(20, dpi),
                    DT_RIGHT | DT_VCENTER | DT_SINGLELINE,
                    &format!("{}%", (state.anim_pct * 100.0).round() as i32),
                );

                let guardian_kb = GUARDIAN_EXE_BYTES.len() / 1024;
                let step3_label = format!("Install silent {guardian_kb} KB tray guardian & Windows integration");
                let steps = if state.is_restore {
                    [
                        "Stop background tray guardian cleanly",
                        "Remove startup & Installed Apps entries",
                        "Restore original backup-original.reg scheme",
                        "Reload native Windows cursor handles",
                    ]
                } else {
                    [
                        "Back up pre-existing Windows cursor scheme",
                        "Download & SHA-1 verify 32 upstream v3.6.2 assets",
                        step3_label.as_str(),
                        "Apply live balanced cursors & start tray icon",
                    ]
                };
                for (i, label) in steps.iter().enumerate() {
                    let col = if i < state.worker_stage {
                        0x86EFAC
                    } else if i == state.worker_stage {
                        0xF3F6FC
                    } else {
                        0x64748B
                    };
                    draw_text(
                        hdc,
                        state.fonts.body,
                        col,
                        s(114, dpi),
                        s(265 + i as i32 * 34, dpi),
                        s(440, dpi),
                        s(22, dpi),
                        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                        label,
                    );
                }

                draw_text(
                    hdc,
                    state.fonts.small,
                    0x7D8799,
                    s(24, dpi),
                    s(488, dpi),
                    s(572, dpi),
                    s(24, dpi),
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                    "Zero external runtimes • Verifying every file by Git blob SHA-1",
                );
            }
            Screen::Done => {
                let heading = if let Some(ref err) = state.worker_error {
                    let _ = err;
                    "Setup Encountered an Error"
                } else if state.is_restore {
                    "Original Windows Cursors Restored"
                } else {
                    "Ready — Active in Show Hidden Icons"
                };
                draw_text(
                    hdc,
                    state.fonts.title,
                    0xF3F6FC,
                    s(64, dpi),
                    s(192, dpi),
                    s(492, dpi),
                    s(28, dpi),
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                    heading,
                );

                if let Some(ref err) = state.worker_error {
                    draw_text(
                        hdc,
                        state.fonts.body,
                        0xFCA5A5,
                        s(76, dpi),
                        s(246, dpi),
                        s(468, dpi),
                        s(150, dpi),
                        DT_CENTER | DT_WORDBREAK,
                        err,
                    );
                } else if state.is_restore {
                    draw_text(
                        hdc,
                        state.fonts.body,
                        0x9CA3AF,
                        s(76, dpi),
                        s(248, dpi),
                        s(468, dpi),
                        s(60, dpi),
                        DT_CENTER | DT_WORDBREAK,
                        "Your pre-install Windows cursor settings have been restored and startup launch has been removed.",
                    );
                } else {
                    let rows = [
                        ("Active Preset", state.preset.short_name().to_string()),
                        (
                            "Balanced Size",
                            format!(
                                "{} px (Accessibility {}/{ACCESSIBILITY_STEPS})",
                                state.current_px, state.current_step
                            ),
                        ),
                        (
                            "Background Guardian",
                            "Running silently in Show hidden icons".to_string(),
                        ),
                        (
                            "Install Folder",
                            state.install_root.to_string_lossy().into_owned(),
                        ),
                    ];
                    for (idx, (k, v)) in rows.iter().enumerate() {
                        let ry = s(244 + idx as i32 * 38, dpi);
                        draw_text(
                            hdc,
                            state.fonts.body,
                            0x8B96A8,
                            s(76, dpi),
                            ry,
                            s(160, dpi),
                            s(24, dpi),
                            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                            k,
                        );
                        draw_text(
                            hdc,
                            state.fonts.body,
                            0xF3F6FC,
                            s(236, dpi),
                            ry,
                            s(308, dpi),
                            s(24, dpi),
                            DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_PATH_ELLIPSIS,
                            v,
                        );
                    }
                }

                draw_text(
                    hdc,
                    state.fonts.small,
                    0x7D8799,
                    s(24, dpi),
                    s(488, dpi),
                    s(290, dpi),
                    s(24, dpi),
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                    "Right-click the tray icon anytime to switch presets",
                );

                draw_text(
                    hdc,
                    state.fonts.btn,
                    0xD6DEE9,
                    s(328, dpi),
                    s(480, dpi),
                    s(128, dpi),
                    s(40, dpi),
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                    "Open Folder",
                );
                draw_text(
                    hdc,
                    state.fonts.btn,
                    0xFFFFFF,
                    s(468, dpi),
                    s(480, dpi),
                    s(128, dpi),
                    s(40, dpi),
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                    "Finish",
                );
            }
        }
        GdiFlush();
    }
}

fn save_backbuffer_bmp(bb: &Backbuffer, out_path: &Path) -> Result<(), String> {
    let w = bb.w as u32;
    let h = bb.h as u32;
    let row_bytes = (w * 3) as usize;
    let pad = (4 - (row_bytes % 4)) % 4;
    let img_size = (row_bytes + pad) * h as usize;
    let file_size = 54 + img_size;

    let mut bmp = Vec::with_capacity(file_size);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&54u32.to_le_bytes());
    bmp.extend_from_slice(&40u32.to_le_bytes());
    bmp.extend_from_slice(&(w as i32).to_le_bytes());
    bmp.extend_from_slice(&(h as i32).to_le_bytes());
    bmp.extend_from_slice(&1u16.to_le_bytes());
    bmp.extend_from_slice(&24u16.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&(img_size as u32).to_le_bytes());
    bmp.extend_from_slice(&0i32.to_le_bytes());
    bmp.extend_from_slice(&0i32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());
    bmp.extend_from_slice(&0u32.to_le_bytes());

    let pixels = unsafe { std::slice::from_raw_parts(bb.bits, (w * h) as usize) };
    for y in (0..h as usize).rev() {
        let row = &pixels[y * w as usize..(y + 1) * w as usize];
        for &px in row {
            bmp.push((px & 0xFF) as u8);
            bmp.push(((px >> 8) & 0xFF) as u8);
            bmp.push(((px >> 16) & 0xFF) as u8);
        }
        for _ in 0..pad {
            bmp.push(0);
        }
    }

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir failed: {e}"))?;
    }
    fs::write(out_path, &bmp).map_err(|e| format!("write bmp failed: {e}"))
}

fn push_worker_update(hwnd_raw: usize, shared: &Arc<Mutex<WorkerUpdate>>, u: WorkerUpdate) {
    if let Ok(mut guard) = shared.lock() {
        *guard = u;
    }
    if hwnd_raw != 0 {
        unsafe {
            PostMessageW(hwnd_raw as *mut c_void, WM_APP_WORKER_UPDATE, 0, 0);
        }
    }
}

fn execute_install_pipeline(
    root: &Path,
    install_dir: &Path,
    backup_path: &Path,
    preset: Preset,
    startup: bool,
    register_uninstall: bool,
    sandbox_only: bool,
    hwnd_raw: usize,
    shared: Option<&Arc<Mutex<WorkerUpdate>>>,
) -> Result<(u32, u32), String> {
    let log = BoundedLogger::new(root.join("guardian.log"), false);
    let report = |pct: f32, stage: usize, msg: String| {
        if let Some(sh) = shared {
            push_worker_update(
                hwnd_raw,
                sh,
                WorkerUpdate {
                    pct,
                    stage,
                    status: msg,
                    finished: false,
                    is_restore: false,
                    error: None,
                    applied_step: 1,
                    applied_px: 32,
                    install_root: root.to_path_buf(),
                },
            );
        }
    };

    report(0.08, 0, "Backing up pre-existing Windows cursor scheme...".into());
    if !sandbox_only {
        export_backup(backup_path, Some(&log))?;
    } else {
        fs::create_dir_all(root).map_err(|e| format!("Could not create {}: {e}", root.display()))?;
    }

    report(0.18, 1, "Preparing 32 upstream v3.6.2 cursor files...".into());
    prepare_assets_parallel(install_dir, Some(&log), |done, total, name| {
        let pct = 0.18 + 0.52 * (done as f32 / total as f32);
        report(
            pct,
            1,
            format!("Verified {done}/{total} cursor assets ({name})"),
        );
    })?;

    report(0.76, 2, "Installing ultra-lightweight tray guardian...".into());
    let guardian_exe = root.join(GUARDIAN_EXE_NAME);
    if !sandbox_only {
        stop_any_running_guardian();
    }
    write_guardian_binary(&guardian_exe, GUARDIAN_EXE_BYTES)?;

    if !sandbox_only {
        save_preset(preset)?;
        set_startup_enabled(&guardian_exe, startup)?;
        if register_uninstall {
            register_uninstall_entry(root, &guardian_exe)?;
        } else {
            unregister_uninstall_entry();
        }
    }

    report(
        0.90,
        3,
        "Applying balanced cursors & starting tray guardian...".into(),
    );
    let (step, px) = if sandbox_only {
        let st = current_accessibility_step();
        (st, balanced_cursor_size(st))
    } else {
        std::process::Command::new(&guardian_exe)
            .args(["--tray", "--preset", preset.cli_name()])
            .spawn()
            .map_err(|e| format!("Could not launch tray guardian: {e}"))?;

        for _ in 0..40 {
            thread::sleep(Duration::from_millis(50));
            if !find_tray_window().is_null() {
                break;
            }
        }
        let st = current_accessibility_step();
        (st, balanced_cursor_size(st))
    };

    log.line(format!(
        "Setup v{APP_VERSION} installed {} ({}px, sandbox={sandbox_only})",
        preset.short_name(),
        px
    ));
    Ok((step, px))
}

fn start_install_worker(state: &mut GuiState, is_restore: bool) {
    state.screen = Screen::Working;
    state.is_restore = is_restore;
    state.anim_pct = 0.04;
    state.target_pct = 0.08;
    state.worker_stage = 0;
    state.worker_error = None;
    state.worker_status = if is_restore {
        "Closing tray guardian...".into()
    } else {
        "Initializing installer...".into()
    };

    let hwnd = state.hwnd as usize;
    let shared = Arc::clone(&state.shared_update);
    let preset = state.preset;
    let startup = state.startup;
    let reg_un = state.register_uninstall;

    unsafe {
        SetTimer(state.hwnd, TIMER_ANIM, 30, null_mut());
        InvalidateRect(state.hwnd, null(), 0);
    }

    thread::spawn(move || {
        let hwnd_raw = hwnd;
        let paths = app_paths();
        let res = match paths {
            Ok((root, install_dir, backup_path)) => {
                if is_restore {
                    let log = BoundedLogger::new(root.join("guardian.log"), false);
                    push_worker_update(
                        hwnd_raw,
                        &shared,
                        WorkerUpdate {
                            pct: 0.25,
                            stage: 0,
                            status: "Stopping background tray guardian...".into(),
                            finished: false,
                            is_restore: true,
                            error: None,
                            applied_step: 1,
                            applied_px: 32,
                            install_root: root.clone(),
                        },
                    );
                    stop_any_running_guardian();
                    push_worker_update(
                        hwnd_raw,
                        &shared,
                        WorkerUpdate {
                            pct: 0.55,
                            stage: 1,
                            status: "Cleaning startup & Installed Apps entries...".into(),
                            finished: false,
                            is_restore: true,
                            error: None,
                            applied_step: 1,
                            applied_px: 32,
                            install_root: root.clone(),
                        },
                    );
                    reg_delete_value(RUN_SUBKEY, GUARDIAN_RUN_VALUE);
                    reg_delete_value(CONFIG_SUBKEY, "Preset");
                    unregister_uninstall_entry();
                    let gexe = root.join(GUARDIAN_EXE_NAME);
                    if gexe.exists() {
                        let _ = fs::remove_file(&gexe);
                    }
                    push_worker_update(
                        hwnd_raw,
                        &shared,
                        WorkerUpdate {
                            pct: 0.85,
                            stage: 2,
                            status: "Restoring original Windows cursor scheme...".into(),
                            finished: false,
                            is_restore: true,
                            error: None,
                            applied_step: 1,
                            applied_px: 32,
                            install_root: root.clone(),
                        },
                    );
                    restore_backup(&backup_path, Some(&log)).map(|()| (root, 1u32, 32u32))
                } else {
                    execute_install_pipeline(
                        &root,
                        &install_dir,
                        &backup_path,
                        preset,
                        startup,
                        reg_un,
                        false,
                        hwnd_raw,
                        Some(&shared),
                    )
                    .map(|(st, px)| (root, st, px))
                }
            }
            Err(e) => Err(e),
        };

        match res {
            Ok((root, st, px)) => {
                push_worker_update(
                    hwnd_raw,
                    &shared,
                    WorkerUpdate {
                        pct: 1.0,
                        stage: 4,
                        status: "Complete!".into(),
                        finished: true,
                        is_restore,
                        error: None,
                        applied_step: st,
                        applied_px: px,
                        install_root: root,
                    },
                );
            }
            Err(err) => {
                push_worker_update(
                    hwnd_raw,
                    &shared,
                    WorkerUpdate {
                        pct: 1.0,
                        stage: 4,
                        status: "Failed".into(),
                        finished: true,
                        is_restore,
                        error: Some(err),
                        applied_step: 1,
                        applied_px: 32,
                        install_root: PathBuf::new(),
                    },
                );
            }
        }
    });
}

fn hit_test(hits: &[HitZone], x: i32, y: i32) -> Option<ControlId> {
    for z in hits {
        if x >= z.rect.left && x < z.rect.right && y >= z.rect.top && y < z.rect.bottom {
            return Some(z.id);
        }
    }
    None
}

unsafe extern "system" fn wnd_proc(
    hwnd: *mut c_void,
    msg: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if msg == WM_DESTROY {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        PostQuitMessage(0);
        return 0;
    }

    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut GuiState;
    if ptr.is_null() {
        return DefWindowProcW(hwnd, msg, w_param, l_param);
    }

    match msg {
        WM_ERASEBKGND => return 1,
        WM_PAINT => {
            let state = &mut *ptr;
            let mut ps: PaintStruct = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            render_ui(state);
            if let Some(bb) = state.backbuffer.as_ref() {
                BitBlt(
                    hdc,
                    0,
                    0,
                    bb.w as i32,
                    bb.h as i32,
                    bb.hdc,
                    0,
                    0,
                    SRCCOPY,
                );
            }
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_SETCURSOR => {
            if (l_param as u32 & 0xFFFF) == 1 {
                let id = if (*ptr).hover.is_some() { IDC_HAND } else { IDC_ARROW };
                SetCursor(LoadCursorW(null_mut(), id as *const u16));
                return 1;
            }
        }
        WM_MOUSEMOVE => {
            let state = &mut *ptr;
            if !state.tracking_mouse {
                let mut tme = TrackMouseEvent {
                    cb_size: std::mem::size_of::<TrackMouseEvent>() as u32,
                    dw_flags: TME_LEAVE,
                    hwnd_track: hwnd,
                    dw_hover_time: 0,
                };
                TrackMouseEvent(&mut tme);
                state.tracking_mouse = true;
            }
            let x = (l_param as i32) & 0xFFFF;
            let y = ((l_param as i32) >> 16) & 0xFFFF;
            let x = (x as i16) as i32;
            let y = (y as i16) as i32;
            let next_hover = hit_test(&state.hits, x, y);
            if next_hover != state.hover {
                state.hover = next_hover;
                InvalidateRect(hwnd, null(), 0);
            }
            return 0;
        }
        WM_MOUSELEAVE => {
            let state = &mut *ptr;
            state.tracking_mouse = false;
            if state.hover.is_some() || state.pressed.is_some() {
                state.hover = None;
                state.pressed = None;
                InvalidateRect(hwnd, null(), 0);
            }
            return 0;
        }
        WM_LBUTTONDOWN => {
            let state = &mut *ptr;
            let x = ((l_param as i32 & 0xFFFF) as i16) as i32;
            let y = (((l_param as i32 >> 16) & 0xFFFF) as i16) as i32;
            state.pressed = hit_test(&state.hits, x, y);
            InvalidateRect(hwnd, null(), 0);
            return 0;
        }
        WM_LBUTTONUP => {
            let x = ((l_param as i32 & 0xFFFF) as i16) as i32;
            let y = (((l_param as i32 >> 16) & 0xFFFF) as i16) as i32;
            let clicked = {
                let state = &mut *ptr;
                let h = hit_test(&state.hits, x, y);
                let match_click = if h == state.pressed { h } else { None };
                state.pressed = None;
                match_click
            };
            if let Some(ctrl) = clicked {
                match ctrl {
                    ControlId::PresetCard(i) => {
                        if let Some(p) = Preset::ALL.get(i) {
                            (*ptr).preset = *p;
                        }
                    }
                    ControlId::ToggleStartup => {
                        (*ptr).startup = !(*ptr).startup;
                    }
                    ControlId::ToggleUninstall => {
                        (*ptr).register_uninstall = !(*ptr).register_uninstall;
                    }
                    ControlId::BtnInstall => {
                        start_install_worker(&mut *ptr, false);
                        return 0;
                    }
                    ControlId::BtnRestore => {
                        let title = wide_z("Restore Original Cursors");
                        let text = wide_z(
                            "Restore your original Windows cursor backup and remove the startup tray guardian?",
                        );
                        let ans = MessageBoxW(
                            hwnd,
                            text.as_ptr(),
                            title.as_ptr(),
                            MB_YESNO | MB_ICONQUESTION | MB_SETFOREGROUND,
                        );
                        if ans == IDYES {
                            start_install_worker(&mut *ptr, true);
                        }
                        return 0;
                    }
                    ControlId::BtnOpenFolder => {
                        let dir = (*ptr).install_root.clone();
                        let op = wide_z("open");
                        let target = wide_z(&dir.to_string_lossy());
                        ShellExecuteW(hwnd, op.as_ptr(), target.as_ptr(), null(), null(), SW_SHOW);
                    }
                    ControlId::BtnFinish => {
                        PostMessageW(hwnd, WM_CLOSE, 0, 0);
                        return 0;
                    }
                }
            }
            InvalidateRect(hwnd, null(), 0);
            return 0;
        }
        WM_APP_WORKER_UPDATE => {
            let state = &mut *ptr;
            if let Ok(u) = state.shared_update.lock() {
                state.target_pct = u.pct;
                state.worker_stage = u.stage;
                state.worker_status = u.status.clone();
                if !u.install_root.as_os_str().is_empty() {
                    state.install_root = u.install_root.clone();
                }
                if u.finished {
                    KillTimer(hwnd, TIMER_ANIM);
                    state.anim_pct = 1.0;
                    state.worker_error = u.error.clone();
                    state.is_restore = u.is_restore;
                    state.current_step = u.applied_step;
                    state.current_px = u.applied_px;
                    state.has_existing_install = !u.is_restore && u.error.is_none();
                    state.screen = Screen::Done;
                }
            }
            InvalidateRect(hwnd, null(), 0);
            return 0;
        }
        WM_TIMER if w_param == TIMER_ANIM => {
            let state = &mut *ptr;
            let diff = state.target_pct - state.anim_pct;
            if diff.abs() > 0.002 {
                state.anim_pct += diff * 0.25;
                InvalidateRect(hwnd, null(), 0);
            }
            return 0;
        }
        WM_KEYDOWN => {
            let state = &mut *ptr;
            if w_param == VK_ESCAPE && state.screen != Screen::Working {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
                return 0;
            }
            if w_param == VK_RETURN {
                match state.screen {
                    Screen::Setup => start_install_worker(state, false),
                    Screen::Done => {
                        PostMessageW(hwnd, WM_CLOSE, 0, 0);
                    }
                    Screen::Working => {}
                }
                return 0;
            }
        }
        WM_DPICHANGED => {
            let state = &mut *ptr;
            state.dpi = (w_param & 0xFFFF) as u32;
            state.fonts.destroy();
            state.fonts = FontSet::create(state.dpi);
            let mut rc = Rect {
                left: 0,
                top: 0,
                right: s(LOGICAL_W, state.dpi),
                bottom: s(LOGICAL_H, state.dpi),
            };
            adjust_rect_for_dpi(&mut rc, WS_OVERLAPPEDWINDOW_FIXED, state.dpi);
            SetWindowPos(
                hwnd,
                null_mut(),
                0,
                0,
                rc.right - rc.left,
                rc.bottom - rc.top,
                SWP_NOMOVE | SWP_NOZORDER,
            );
            InvalidateRect(hwnd, null(), 0);
            return 0;
        }
        WM_CLOSE => {
            if (*ptr).screen == Screen::Working {
                return 0;
            }
            DestroyWindow(hwnd);
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, w_param, l_param)
}

fn create_setup_window(state_ptr: *mut GuiState, show: bool) -> Result<*mut c_void, String> {
    unsafe {
        enable_per_monitor_dpi_v2();
        let h_inst = GetModuleHandleW(null());
        let w_cls = wide_z(WND_CLASS);
        let wc = WndClassExW {
            cb_size: std::mem::size_of::<WndClassExW>() as u32,
            style: 0,
            lpfn_wnd_proc: wnd_proc,
            cb_cls_extra: 0,
            cb_wnd_extra: 0,
            h_instance: h_inst,
            h_icon: (*state_ptr).app_icon,
            h_cursor: LoadCursorW(null_mut(), IDC_ARROW as *const u16),
            hbr_background: null_mut(),
            lpsz_menu_name: null(),
            lpsz_class_name: w_cls.as_ptr(),
            h_icon_sm: (*state_ptr).app_icon,
        };
        RegisterClassExW(&wc);

        let w_title = wide_z(&format!("Modern Inverted Cursors Setup v{APP_VERSION}"));
        let style = if show {
            WS_OVERLAPPEDWINDOW_FIXED | WS_VISIBLE
        } else {
            WS_OVERLAPPEDWINDOW_FIXED
        };
        let hwnd = CreateWindowExW(
            0,
            w_cls.as_ptr(),
            w_title.as_ptr(),
            style,
            100,
            100,
            LOGICAL_W,
            LOGICAL_H,
            null_mut(),
            null_mut(),
            h_inst,
            null_mut(),
        );
        if hwnd.is_null() {
            return Err("CreateWindowExW failed.".into());
        }

        let dpi = window_dpi(hwnd);
        (*state_ptr).hwnd = hwnd;
        (*state_ptr).dpi = dpi;
        (*state_ptr).fonts = FontSet::create(dpi);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);

        let _ = apply_dwm_dark_chrome(hwnd);
        SendMessageW(hwnd, WM_SETICON, 0, (*state_ptr).app_icon as isize);
        SendMessageW(hwnd, WM_SETICON, 1, (*state_ptr).app_icon as isize);

        let mut rc = Rect {
            left: 0,
            top: 0,
            right: s(LOGICAL_W, dpi),
            bottom: s(LOGICAL_H, dpi),
        };
        adjust_rect_for_dpi(&mut rc, WS_OVERLAPPEDWINDOW_FIXED, dpi);
        let win_w = rc.right - rc.left;
        let win_h = rc.bottom - rc.top;

        let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut mi: MonitorInfo = std::mem::zeroed();
        mi.cb_size = std::mem::size_of::<MonitorInfo>() as u32;
        let (x, y) = if GetMonitorInfoW(hmon, &mut mi) != 0 {
            let work_w = mi.rc_work.right - mi.rc_work.left;
            let work_h = mi.rc_work.bottom - mi.rc_work.top;
            (
                mi.rc_work.left + (work_w - win_w) / 2,
                mi.rc_work.top + (work_h - win_h) / 2,
            )
        } else {
            (160, 120)
        };
        SetWindowPos(hwnd, null_mut(), x, y, win_w, win_h, SWP_NOZORDER);
        Ok(hwnd)
    }
}

fn new_gui_state(
    preset: Preset,
    startup: bool,
    register_uninstall: bool,
    has_existing: bool,
    install_root: PathBuf,
) -> *mut GuiState {
    let step = current_accessibility_step();
    let px = balanced_cursor_size(step);
    let dummy_fonts = FontSet {
        title: null_mut(),
        section: null_mut(),
        card_title: null_mut(),
        body: null_mut(),
        small: null_mut(),
        btn: null_mut(),
    };
    Box::into_raw(Box::new(GuiState {
        hwnd: null_mut(),
        dpi: 96,
        screen: Screen::Setup,
        preset,
        startup,
        register_uninstall,
        has_existing_install: has_existing,
        current_step: step,
        current_px: px,
        hover: None,
        pressed: None,
        tracking_mouse: false,
        hits: Vec::new(),
        anim_pct: 0.0,
        target_pct: 0.0,
        worker_stage: 0,
        worker_status: String::new(),
        worker_error: None,
        is_restore: false,
        install_root,
        shared_update: Arc::new(Mutex::new(WorkerUpdate {
            pct: 0.0,
            stage: 0,
            status: String::new(),
            finished: false,
            is_restore: false,
            error: None,
            applied_step: step,
            applied_px: px,
            install_root: PathBuf::new(),
        })),
        fonts: dummy_fonts,
        backbuffer: None,
        app_icon: modern_inverted_cursor::create_split_cursor_hicon(32, Preset::Regular, false),
    }))
}

fn run_smoke_ui(out_dir: &Path) -> Result<(), String> {
    let state_ptr = new_gui_state(Preset::Compact, true, true, true, PathBuf::from(r"C:\Users\Demo\AppData\Local\ModernInvertedCursors"));
    let hwnd = create_setup_window(state_ptr, false)?;
    unsafe {
        let state = &mut *state_ptr;
        state.screen = Screen::Setup;
        render_ui(state);
        if let Some(bb) = state.backbuffer.as_ref() {
            save_backbuffer_bmp(bb, &out_dir.join("01_setup_dark.bmp"))?;
        }

        state.screen = Screen::Working;
        state.anim_pct = 0.64;
        state.target_pct = 0.64;
        state.worker_stage = 1;
        state.worker_status = "Verified 21/32 cursor assets (link_v2.cur)".into();
        render_ui(state);
        if let Some(bb) = state.backbuffer.as_ref() {
            save_backbuffer_bmp(bb, &out_dir.join("02_installing_dark.bmp"))?;
        }

        state.screen = Screen::Done;
        state.worker_error = None;
        state.is_restore = false;
        render_ui(state);
        if let Some(bb) = state.backbuffer.as_ref() {
            save_backbuffer_bmp(bb, &out_dir.join("03_done_dark.bmp"))?;
        }

        let dark_ok = apply_dwm_dark_chrome(hwnd);
        let report = format!(
            "SMOKE_UI_OK dpi={} dwm_dark={dark_ok} hits_cleared={} size={}x{}\n",
            state.dpi,
            state.hits.len(),
            state.backbuffer.as_ref().map_or(0, |b| b.w),
            state.backbuffer.as_ref().map_or(0, |b| b.h)
        );
        let _ = fs::write(out_dir.join("smoke-ui-report.txt"), report);

        if let Some(mut bb) = state.backbuffer.take() {
            bb.destroy();
        }
        state.fonts.destroy();
        if !state.app_icon.is_null() {
            DestroyIcon(state.app_icon);
        }
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        DestroyWindow(hwnd);
        let _ = Box::from_raw(state_ptr);
    }
    Ok(())
}

fn arg_value(args: &[String], flag: &str) -> Option<String> {
    for i in 0..args.len() {
        if args[i] == flag {
            return args.get(i + 1).cloned();
        }
        if let Some(v) = args[i].strip_prefix(&format!("{flag}=")) {
            return Some(v.to_string());
        }
    }
    None
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();

    if let Some(smoke_dir) = arg_value(&args, "--smoke-ui") {
        return run_smoke_ui(Path::new(&smoke_dir));
    }

    if let Some(sandbox_dir) = arg_value(&args, "--sandbox") {
        let preset = parse_preset_arg(&args)?.unwrap_or(Preset::Compact);
        let (root, install_dir, backup_path) = app_paths_for_root(PathBuf::from(sandbox_dir));
        let (step, px) = execute_install_pipeline(
            &root,
            &install_dir,
            &backup_path,
            preset,
            false,
            false,
            true,
            0,
            None,
        )?;
        let report = format!(
            "SANDBOX_OK preset={} step={} px={} assets=32 guardian_bytes={}\n",
            preset.cli_name(),
            step,
            px,
            GUARDIAN_EXE_BYTES.len()
        );
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("sandbox-report.txt"))
            .and_then(|mut f| f.write_all(report.as_bytes()));
        return Ok(());
    }

    if args.iter().any(|a| a == "--restore") {
        let (root, _, backup_path) = app_paths()?;
        let log = BoundedLogger::new(root.join("guardian.log"), false);
        stop_any_running_guardian();
        reg_delete_value(RUN_SUBKEY, GUARDIAN_RUN_VALUE);
        reg_delete_value(CONFIG_SUBKEY, "Preset");
        unregister_uninstall_entry();
        let gexe = root.join(GUARDIAN_EXE_NAME);
        if gexe.exists() {
            let _ = fs::remove_file(&gexe);
        }
        return restore_backup(&backup_path, Some(&log));
    }

    if args.iter().any(|a| a == "--silent") {
        let preset = parse_preset_arg(&args)?.unwrap_or(Preset::Compact);
        let no_startup = args.iter().any(|a| a == "--no-startup");
        let (root, install_dir, backup_path) = app_paths()?;
        let _ = execute_install_pipeline(
            &root,
            &install_dir,
            &backup_path,
            preset,
            !no_startup,
            true,
            false,
            0,
            None,
        )?;
        let _ = register_and_apply(preset, &install_dir, None);
        return Ok(());
    }

    let (root, _, backup_path) = app_paths()?;
    let has_existing = backup_path.exists() || root.join(GUARDIAN_EXE_NAME).exists();
    let initial_preset = parse_preset_arg(&args)?.unwrap_or_else(load_saved_preset);

    let state_ptr = new_gui_state(initial_preset, true, true, has_existing, root);
    let hwnd = create_setup_window(state_ptr, true)?;

    unsafe {
        let mut msg: Msg = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let mut state = Box::from_raw(state_ptr);
        if let Some(mut bb) = state.backbuffer.take() {
            bb.destroy();
        }
        state.fonts.destroy();
        if !state.app_icon.is_null() {
            DestroyIcon(state.app_icon);
        }
        let _ = hwnd;
    }
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        let title = wide_z("Modern Inverted Cursors Setup Error");
        let body = wide_z(&err);
        unsafe {
            MessageBoxW(null_mut(), body.as_ptr(), title.as_ptr(), 0x0001_0010);
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_win32_setup_structs_match_sdk() {
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(std::mem::size_of::<TrackMouseEvent>(), 24);
            assert_eq!(std::mem::size_of::<MonitorInfo>(), 40);
            assert_eq!(std::mem::size_of::<PaintStruct>(), 72);
            assert_eq!(std::mem::size_of::<BitmapInfoHeader>(), 40);
        }
        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(std::mem::size_of::<TrackMouseEvent>(), 16);
            assert_eq!(std::mem::size_of::<MonitorInfo>(), 40);
            assert_eq!(std::mem::size_of::<PaintStruct>(), 64);
            assert_eq!(std::mem::size_of::<BitmapInfoHeader>(), 40);
        }
    }

    #[test]
    fn test_embedded_guardian_is_nonempty_pe_binary() {
        assert!(GUARDIAN_EXE_BYTES.len() > 10_000);
        assert_eq!(&GUARDIAN_EXE_BYTES[..2], b"MZ");
        let pe_off = u32::from_le_bytes([
            GUARDIAN_EXE_BYTES[0x3C],
            GUARDIAN_EXE_BYTES[0x3D],
            GUARDIAN_EXE_BYTES[0x3E],
            GUARDIAN_EXE_BYTES[0x3F],
        ]) as usize;
        assert_eq!(&GUARDIAN_EXE_BYTES[pe_off..pe_off + 4], b"PE\0\0");
        let machine = u16::from_le_bytes([GUARDIAN_EXE_BYTES[pe_off + 4], GUARDIAN_EXE_BYTES[pe_off + 5]]);
        #[cfg(target_arch = "x86_64")]
        assert_eq!(machine, 0x8664);
        #[cfg(target_arch = "aarch64")]
        assert_eq!(machine, 0xAA64);
        #[cfg(target_arch = "x86")]
        assert_eq!(machine, 0x014C);
    }
}
