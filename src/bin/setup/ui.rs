use crate::install::{start_install_worker, GUARDIAN_EXE_BYTES};
use crate::{ControlId, GuiState, HitZone, Screen, WorkerTarget, WorkerUpdate, TIMER_ANIM};
use modern_inverted_cursor::{
    balanced_cursor_size, create_split_cursor_hicon, current_accessibility_step, load_system_dll,
    trim_working_set, wide_z, CursorGlyphSprite, Preset, ACCESSIBILITY_STEPS, APP_VERSION,
    PACK_VERSION,
};
use std::{
    ffi::c_void,
    fs,
    path::{Path, PathBuf},
    ptr::{null, null_mut},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

const WND_CLASS: &str = "ModernInvertedDarkSetupWnd";

pub const LOGICAL_W: i32 = 620;
pub const LOGICAL_H: i32 = 540;

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

const VK_TAB: usize = 0x09;
const VK_RETURN: usize = 0x0D;
const VK_SHIFT: i32 = 0x10;
const VK_ESCAPE: usize = 0x1B;
const VK_SPACE: usize = 0x20;
const VK_LEFT: usize = 0x25;
const VK_UP: usize = 0x26;
const VK_RIGHT: usize = 0x27;
const VK_DOWN: usize = 0x28;
const GWLP_USERDATA: i32 = -21;

const WS_OVERLAPPEDWINDOW_FIXED: u32 = 0x00CA_0000;
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
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[repr(C)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[repr(C)]
pub struct Msg {
    pub hwnd: *mut c_void,
    pub message: u32,
    pub w_param: usize,
    pub l_param: isize,
    pub time: u32,
    pub pt: Point,
}

#[repr(C)]
pub struct PaintStruct {
    pub hdc: *mut c_void,
    pub f_erase: i32,
    pub rc_paint: Rect,
    pub f_restore: i32,
    pub f_inc_update: i32,
    pub rgb_reserved: [u8; 32],
}

#[repr(C)]
pub struct TrackMouseEvent {
    pub cb_size: u32,
    pub dw_flags: u32,
    pub hwnd_track: *mut c_void,
    pub dw_hover_time: u32,
}

#[repr(C)]
pub struct MonitorInfo {
    pub cb_size: u32,
    pub rc_monitor: Rect,
    pub rc_work: Rect,
    pub dw_flags: u32,
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
pub struct BitmapInfoHeader {
    pub bi_size: u32,
    pub bi_width: i32,
    pub bi_height: i32,
    pub bi_planes: u16,
    pub bi_bit_count: u16,
    pub bi_compression: u32,
    pub bi_size_image: u32,
    pub bi_x_pels_per_meter: i32,
    pub bi_y_pels_per_meter: i32,
    pub bi_clr_used: u32,
    pub bi_clr_important: u32,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lp_module_name: *const u16) -> *mut c_void;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const u8) -> *mut c_void;
    pub fn CreateMutexW(
        lp_mutex_attributes: *mut c_void,
        b_initial_owner: i32,
        lp_name: *const u16,
    ) -> *mut c_void;
    pub fn GetLastError() -> u32;
    pub fn CloseHandle(h_object: *mut c_void) -> i32;
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
    pub fn GetMessageW(
        lp_msg: *mut Msg,
        h_wnd: *mut c_void,
        w_msg_filter_min: u32,
        w_msg_filter_max: u32,
    ) -> i32;
    pub fn TranslateMessage(lp_msg: *const Msg) -> i32;
    pub fn DispatchMessageW(lp_msg: *const Msg) -> isize;
    pub fn InvalidateRect(h_wnd: *mut c_void, lp_rect: *const Rect, b_erase: i32) -> i32;
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
    fn GetKeyState(n_virt_key: i32) -> i16;
    pub fn SetTimer(
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
    pub fn DestroyIcon(h_icon: *mut c_void) -> i32;
    pub fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> *mut c_void;
    pub fn SetForegroundWindow(h_wnd: *mut c_void) -> i32;
    pub fn MessageBoxW(
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

unsafe fn enable_per_monitor_dpi_v2() {
    let u32_mod = load_system_dll("user32.dll");
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
    let shcore = load_system_dll("shcore.dll");
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
        let u32_mod = load_system_dll("user32.dll");
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
    let u32_mod = load_system_dll("user32.dll");
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
    let dwm = load_system_dll("dwmapi.dll");
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
    let mut r1 = set_attr(hwnd, 20, &dark as *const i32 as *const c_void, 4);
    if r1 != 0 {
        r1 = set_attr(hwnd, 19, &dark as *const i32 as *const c_void, 4);
    }
    let round: i32 = 2;
    let _ = set_attr(hwnd, 33, &round as *const i32 as *const c_void, 4);
    let border: u32 = 0x003D_312B;
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
    let g = (((fg >> 8) & 0xFF) as f32 * a + ((bg & 0xFF00) >> 8) as f32 * inv).round() as u32;
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

#[allow(clippy::too_many_arguments)]
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
                let border_mix = (d + border_px + 0.5).clamp(0.0, 1.0);
                blend_px(fill_rgb, border_rgb, border_mix)
            } else {
                0xFF00_0000 | fill_rgb
            };
            buf[row + px] = blend_px(buf[row + px], col, cov);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn stroke_round_rect(
    buf: &mut [u32],
    w: usize,
    h: usize,
    x: i32,
    y: i32,
    rw: i32,
    rh: i32,
    radius: f32,
    stroke_rgb: u32,
    stroke_px: f32,
) {
    if rw <= 0 || rh <= 0 || stroke_px <= 0.0 {
        return;
    }
    let hw = rw as f32 * 0.5;
    let hh = rh as f32 * 0.5;
    let cx = x as f32 + hw;
    let cy = y as f32 + hh;
    let r = radius.min(hw).min(hh);
    let half = stroke_px * 0.5;

    let pad = (half + 2.0).ceil() as i32;
    let x0 = (x - pad).max(0) as usize;
    let y0 = (y - pad).max(0) as usize;
    let x1 = (x + rw + pad).min(w as i32) as usize;
    let y1 = (y + rh + pad).min(h as i32) as usize;

    for py in y0..y1 {
        let row = py * w;
        for px in x0..x1 {
            let d = sdf_round_box(px as f32 + 0.5, py as f32 + 0.5, cx, cy, hw, hh, r);
            let ring_dist = d.abs() - half;
            if ring_dist >= 0.75 {
                continue;
            }
            let cov = (0.75 - ring_dist).clamp(0.0, 1.0);
            buf[row + px] = blend_px(buf[row + px], stroke_rgb, cov);
        }
    }
}

#[derive(Default)]
pub struct GlyphSpriteCache {
    size: i32,
    sprites: Option<[CursorGlyphSprite; 4]>,
}

impl GlyphSpriteCache {
    pub fn clear(&mut self) {
        self.size = 0;
        self.sprites = None;
    }

    fn sprites_for(&mut self, size: i32) -> &[CursorGlyphSprite; 4] {
        let sz = size.max(1);
        if self.size != sz || self.sprites.is_none() {
            let list = [
                CursorGlyphSprite::rasterize(sz, Preset::ALL[0], false),
                CursorGlyphSprite::rasterize(sz, Preset::ALL[1], false),
                CursorGlyphSprite::rasterize(sz, Preset::ALL[2], false),
                CursorGlyphSprite::rasterize(sz, Preset::ALL[3], false),
            ];
            self.size = sz;
            self.sprites = Some(list);
        }
        self.sprites.as_ref().unwrap()
    }
}

fn preset_glyph_index(preset: Preset) -> usize {
    Preset::ALL.iter().position(|&p| p == preset).unwrap_or(0)
}

#[allow(clippy::too_many_arguments)]
fn fill_circle_aa(
    buf: &mut [u32],
    w: usize,
    h: usize,
    cx: f32,
    cy: f32,
    radius: f32,
    fill: u32,
    border: Option<(u32, f32)>,
) {
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

#[allow(clippy::too_many_arguments)]
fn draw_line_aa(
    buf: &mut [u32],
    w: usize,
    h: usize,
    ax: f32,
    ay: f32,
    bx: f32,
    by: f32,
    half_thick: f32,
    col: u32,
) {
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
    let s_px = x0.max(0) as usize;
    let e_px = x1.min(w as i32) as usize;
    for px in s_px..e_px {
        buf[row + px] = 0xFF00_0000 | col;
    }
}

pub struct FontSet {
    pub title: *mut c_void,
    pub section: *mut c_void,
    pub card_title: *mut c_void,
    pub body: *mut c_void,
    pub small: *mut c_void,
    pub btn: *mut c_void,
}

impl FontSet {
    pub unsafe fn create(dpi: u32) -> Self {
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

    pub unsafe fn destroy(&mut self) {
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

pub struct Backbuffer {
    pub hdc: *mut c_void,
    pub hbm: *mut c_void,
    pub old_bm: *mut c_void,
    pub bits: *mut u32,
    pub w: usize,
    pub h: usize,
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
        let old_bm = SelectObject(hdc, hbm);
        Some(Self {
            hdc,
            hbm,
            old_bm,
            bits: raw as *mut u32,
            w,
            h,
        })
    }

    unsafe fn pixels_mut(&mut self) -> &mut [u32] {
        std::slice::from_raw_parts_mut(self.bits, self.w * self.h)
    }

    pub unsafe fn destroy(&mut self) {
        if !self.hdc.is_null() && !self.old_bm.is_null() {
            SelectObject(self.hdc, self.old_bm);
            self.old_bm = null_mut();
        }
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

#[allow(clippy::too_many_arguments)]
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

#[allow(clippy::too_many_arguments)]
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

pub fn render_ui(state: &mut GuiState) {
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

    let glyph_sz = s(34, dpi);
    let _ = state.glyph_cache.sprites_for(glyph_sz);
    let (sprites, bb_opt, hits) = (
        state.glyph_cache.sprites.as_ref().unwrap(),
        state.backbuffer.as_mut(),
        &mut state.hits,
    );
    let bb = match bb_opt {
        Some(b) => b,
        None => return,
    };
    hits.clear();

    {
        let buf = unsafe { bb.pixels_mut() };
        buf.fill(0xFF0E_1117);

        let hdr_h = s(84, dpi);
        for row in 0..(hdr_h as usize).min(ch) {
            let start = row * cw;
            buf[start..start + cw].fill(0xFF13_1720);
        }
        fill_hline(buf, cw, ch, 0, cw as i32, hdr_h, 0x232A38);

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
        sprites[preset_glyph_index(Preset::Regular)].blit_onto(buf, cw, s(29, dpi), s(25, dpi));

        fill_round_rect(
            buf,
            cw,
            ch,
            s(334, dpi),
            s(21, dpi),
            s(160, dpi),
            s(22, dpi),
            s(11, dpi) as f32,
            0x18253E,
            0x2C4775,
            1.0,
        );

        let ftr_y = s(460, dpi);
        for row in (ftr_y as usize)..ch {
            let start = row * cw;
            buf[start..start + cw].fill(0xFF12_161E);
        }
        fill_hline(buf, cw, ch, 0, cw as i32, ftr_y, 0x232A38);

        match state.screen {
            Screen::Setup => {
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

                    sprites[idx].blit_onto(
                        buf,
                        cw,
                        ccx as i32 - glyph_sz / 2,
                        ccy as i32 - glyph_sz / 2,
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
                    if state.focus == Some(ControlId::PresetCard(idx)) {
                        stroke_round_rect(
                            buf,
                            cw,
                            ch,
                            cx - 2,
                            cy - 2,
                            card_w + 4,
                            card_h + 4,
                            s(14, dpi) as f32,
                            0x93C5FD,
                            2.0,
                        );
                    }

                    hits.push(HitZone {
                        id: ControlId::PresetCard(idx),
                        rect: Rect {
                            left: cx,
                            top: cy,
                            right: cx + card_w,
                            bottom: cy + card_h,
                        },
                    });
                }

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
                fill_hline(
                    buf,
                    cw,
                    ch,
                    s(40, dpi),
                    s(580, dpi),
                    opt_y + s(54, dpi),
                    0x222937,
                );

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
                if state.focus == Some(ControlId::ToggleStartup) {
                    stroke_round_rect(
                        buf,
                        cw,
                        ch,
                        sw_x - 3,
                        opt_y + s(16, dpi) - 3,
                        sw_w + 6,
                        sw_h + 6,
                        sw_h as f32 * 0.5 + 3.0,
                        0x93C5FD,
                        2.0,
                    );
                }
                hits.push(HitZone {
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
                if state.focus == Some(ControlId::ToggleUninstall) {
                    stroke_round_rect(
                        buf,
                        cw,
                        ch,
                        sw_x - 3,
                        opt_y + s(70, dpi) - 3,
                        sw_w + 6,
                        sw_h + 6,
                        sw_h as f32 * 0.5 + 3.0,
                        0x93C5FD,
                        2.0,
                    );
                }
                hits.push(HitZone {
                    id: ControlId::ToggleUninstall,
                    rect: Rect {
                        left: s(24, dpi),
                        top: opt_y + s(54, dpi),
                        right: s(596, dpi),
                        bottom: opt_y + opt_h,
                    },
                });

                if state.has_existing_install {
                    let hov = state.hover == Some(ControlId::BtnRestore);
                    let prs = state.pressed == Some(ControlId::BtnRestore);
                    let bg = if prs {
                        0x1A2029
                    } else if hov {
                        0x293242
                    } else {
                        0x1F2633
                    };
                    fill_round_rect(
                        buf,
                        cw,
                        ch,
                        s(304, dpi),
                        s(480, dpi),
                        s(144, dpi),
                        s(40, dpi),
                        s(9, dpi) as f32,
                        bg,
                        0x364154,
                        1.1,
                    );
                    if state.focus == Some(ControlId::BtnRestore) {
                        stroke_round_rect(
                            buf,
                            cw,
                            ch,
                            s(304, dpi) - 2,
                            s(480, dpi) - 2,
                            s(144, dpi) + 4,
                            s(40, dpi) + 4,
                            s(11, dpi) as f32,
                            0x93C5FD,
                            2.0,
                        );
                    }
                    hits.push(HitZone {
                        id: ControlId::BtnRestore,
                        rect: Rect {
                            left: s(304, dpi),
                            top: s(480, dpi),
                            right: s(448, dpi),
                            bottom: s(520, dpi),
                        },
                    });
                }

                let hov_i = state.hover == Some(ControlId::BtnInstall);
                let prs_i = state.pressed == Some(ControlId::BtnInstall);
                let ibg = if prs_i {
                    0x1D4ED8
                } else if hov_i {
                    0x3B82F6
                } else {
                    0x2563EB
                };
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
                if state.focus == Some(ControlId::BtnInstall) {
                    stroke_round_rect(
                        buf,
                        cw,
                        ch,
                        s(458, dpi) - 2,
                        s(480, dpi) - 2,
                        s(138, dpi) + 4,
                        s(40, dpi) + 4,
                        s(11, dpi) as f32,
                        0xFFFFFF,
                        2.0,
                    );
                }
                hits.push(HitZone {
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

                let sc = dpi as f32 / 96.0;
                for i in 0..4usize {
                    let sy = s(276 + i as i32 * 34, dpi) as f32;
                    let sx = s(96, dpi) as f32;
                    let r = s(7, dpi) as f32;
                    if i < state.worker_stage {
                        fill_circle_aa(buf, cw, ch, sx, sy, r, 0x16A34A, Some((0x22C55E, 1.0)));
                        draw_line_aa(
                            buf,
                            cw,
                            ch,
                            sx - 3.2 * sc,
                            sy,
                            sx - 0.8 * sc,
                            sy + 2.6 * sc,
                            1.1 * sc,
                            0xFFFF_FFFF,
                        );
                        draw_line_aa(
                            buf,
                            cw,
                            ch,
                            sx - 0.8 * sc,
                            sy + 2.6 * sc,
                            sx + 3.6 * sc,
                            sy - 2.6 * sc,
                            1.1 * sc,
                            0xFFFF_FFFF,
                        );
                    } else if i == state.worker_stage {
                        fill_circle_aa(
                            buf,
                            cw,
                            ch,
                            sx,
                            sy,
                            r,
                            0x1E3A8A,
                            Some((0x60A5FA, 1.3 * sc)),
                        );
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
                    s(110, dpi),
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
                    draw_line_aa(
                        buf,
                        cw,
                        ch,
                        cx - 7.5 * sc,
                        cy - 7.5 * sc,
                        cx + 7.5 * sc,
                        cy + 7.5 * sc,
                        1.8 * sc,
                        0xFFFF_FFFF,
                    );
                    draw_line_aa(
                        buf,
                        cw,
                        ch,
                        cx + 7.5 * sc,
                        cy - 7.5 * sc,
                        cx - 7.5 * sc,
                        cy + 7.5 * sc,
                        1.8 * sc,
                        0xFFFF_FFFF,
                    );
                } else {
                    fill_circle_aa(buf, cw, ch, cx, cy, rad, 0x132E22, Some((0x22C55E, 1.6)));
                    draw_line_aa(
                        buf,
                        cw,
                        ch,
                        cx - 8.5 * sc,
                        cy + 1.0 * sc,
                        cx - 2.0 * sc,
                        cy + 7.5 * sc,
                        1.8 * sc,
                        0xFFFF_FFFF,
                    );
                    draw_line_aa(
                        buf,
                        cw,
                        ch,
                        cx - 2.0 * sc,
                        cy + 7.5 * sc,
                        cx + 9.5 * sc,
                        cy - 6.5 * sc,
                        1.8 * sc,
                        0xFFFF_FFFF,
                    );
                }

                for r_idx in 0..3 {
                    let div_y = s(276 + r_idx * 38, dpi);
                    fill_hline(buf, cw, ch, s(76, dpi), s(544, dpi), div_y, 0x222937);
                }

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
                if state.focus == Some(ControlId::BtnOpenFolder) {
                    stroke_round_rect(
                        buf,
                        cw,
                        ch,
                        s(328, dpi) - 2,
                        s(480, dpi) - 2,
                        s(128, dpi) + 4,
                        s(40, dpi) + 4,
                        s(11, dpi) as f32,
                        0x93C5FD,
                        2.0,
                    );
                }
                hits.push(HitZone {
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
                if state.focus == Some(ControlId::BtnFinish) {
                    stroke_round_rect(
                        buf,
                        cw,
                        ch,
                        s(468, dpi) - 2,
                        s(480, dpi) - 2,
                        s(128, dpi) + 4,
                        s(40, dpi) + 4,
                        s(11, dpi) as f32,
                        0xFFFFFF,
                        2.0,
                    );
                }
                hits.push(HitZone {
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
            s(160, dpi),
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
            "Dynamic XOR-inverting Windows pointers + silent system-tray guardian",
        );

        match state.screen {
            Screen::Setup => {
                let section_title = if state.is_modify_mode {
                    "MAINTENANCE — CHOOSE PRESET"
                } else {
                    "CHOOSE CURSOR PRESET"
                };
                draw_text(
                    hdc,
                    state.fonts.section,
                    0x7D8799,
                    s(24, dpi),
                    s(98, dpi),
                    s(240, dpi),
                    s(20, dpi),
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                    section_title,
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
                        "Pointer Size: {}/{ACCESSIBILITY_STEPS} -> {} px (Balanced)",
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
                    "Runs silently (~1-2 MB RAM, negligible idle CPU) with no terminal window",
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
                    "Enables Modify, Repair & Uninstall from Windows Settings",
                );

                draw_text(
                    hdc,
                    state.fonts.small,
                    0x7D8799,
                    s(24, dpi),
                    s(488, dpi),
                    s(270, dpi),
                    s(24, dpi),
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                    &format!(
                        "Pure Rust • Safe backup • {} KB guardian",
                        GUARDIAN_EXE_BYTES.len() / 1024
                    ),
                );

                if state.has_existing_install {
                    draw_text(
                        hdc,
                        state.fonts.btn,
                        0xD6DEE9,
                        s(304, dpi),
                        s(480, dpi),
                        s(144, dpi),
                        s(40, dpi),
                        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                        "Restore & Uninstall",
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
                let step3_label = format!("Install Setup & {guardian_kb} KB tray guardian");
                let steps = if state.is_restore {
                    [
                        "Validate original backup-original.reg scheme",
                        "Stop tray guardian & restore backup",
                        "Clean startup & Installed Apps entries",
                        "Reload native Windows cursor handles",
                    ]
                } else {
                    [
                        "Back up pre-existing Windows cursor scheme",
                        "Download & SHA-1 verify 32 upstream v3.6.2 assets",
                        step3_label.as_str(),
                        "Verify tray guardian startup & initial cursor apply",
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
                let heading = if state.worker_error.is_some() {
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
                            "Verified active in Show hidden icons".to_string(),
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

pub fn save_backbuffer_bmp(bb: &Backbuffer, out_path: &Path) -> Result<(), String> {
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
    let pad_zeros = [0u8; 3];
    for y in (0..h as usize).rev() {
        let row = &pixels[y * w as usize..(y + 1) * w as usize];
        for &px in row {
            bmp.push((px & 0xFF) as u8);
            bmp.push(((px >> 8) & 0xFF) as u8);
            bmp.push(((px >> 16) & 0xFF) as u8);
        }
        if pad > 0 {
            bmp.extend_from_slice(&pad_zeros[..pad]);
        }
    }

    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir failed: {e}"))?;
    }
    fs::write(out_path, &bmp).map_err(|e| format!("write bmp failed: {e}"))
}

pub fn push_worker_update(
    target: Option<&Arc<WorkerTarget>>,
    shared: &Arc<Mutex<WorkerUpdate>>,
    u: WorkerUpdate,
) {
    if let Ok(mut guard) = shared.lock() {
        *guard = u;
    }
    if let Some(t) = target {
        if t.alive.load(Ordering::Acquire) {
            let hwnd = t.hwnd.load(Ordering::Acquire);
            if hwnd != 0 {
                // SAFETY: `t.alive` is cleared in `WM_DESTROY` before `hwnd` becomes invalid.
                unsafe {
                    PostMessageW(hwnd as *mut c_void, WM_APP_WORKER_UPDATE, 0, 0);
                }
            }
        }
    }
}

fn hit_test(hits: &[HitZone], x: i32, y: i32) -> Option<ControlId> {
    for z in hits {
        if x >= z.rect.left && x < z.rect.right && y >= z.rect.top && y < z.rect.bottom {
            return Some(z.id);
        }
    }
    None
}

pub fn focus_order(
    screen: Screen,
    has_existing: bool,
    _worker_error: bool,
    _is_restore: bool,
) -> Vec<ControlId> {
    match screen {
        Screen::Setup => {
            let mut order = vec![
                ControlId::PresetCard(0),
                ControlId::PresetCard(1),
                ControlId::PresetCard(2),
                ControlId::PresetCard(3),
                ControlId::ToggleStartup,
                ControlId::ToggleUninstall,
            ];
            if has_existing {
                order.push(ControlId::BtnRestore);
            }
            order.push(ControlId::BtnInstall);
            order
        }
        Screen::Working => Vec::new(),
        Screen::Done => vec![ControlId::BtnOpenFolder, ControlId::BtnFinish],
    }
}

unsafe fn activate_control(hwnd: *mut c_void, ptr: *mut GuiState, ctrl: ControlId) -> bool {
    let state = &mut *ptr;
    match ctrl {
        ControlId::PresetCard(i) => {
            if let Some(p) = Preset::ALL.get(i) {
                state.preset = *p;
                state.dirty = true;
            }
        }
        ControlId::ToggleStartup => {
            state.startup = !state.startup;
            state.dirty = true;
        }
        ControlId::ToggleUninstall => {
            state.register_uninstall = !state.register_uninstall;
            state.dirty = true;
        }
        ControlId::BtnInstall => {
            start_install_worker(state, false);
            return true;
        }
        ControlId::BtnRestore => {
            let title = wide_z("Restore & Uninstall Modern Inverted Cursors");
            let text = wide_z(
                "Validate your original cursor backup, restore Windows cursors, remove startup launch, and uninstall?",
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
            return true;
        }
        ControlId::BtnOpenFolder => {
            let dir = state.install_root.clone();
            let op = wide_z("open");
            let target = wide_z(&dir.to_string_lossy());
            ShellExecuteW(hwnd, op.as_ptr(), target.as_ptr(), null(), null(), SW_SHOW);
        }
        ControlId::BtnFinish => {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
            return true;
        }
    }
    false
}

unsafe extern "system" fn wnd_proc(
    hwnd: *mut c_void,
    msg: u32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if msg == WM_DESTROY {
        let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut GuiState;
        if !p.is_null() {
            let state = &*p;
            state.worker_target.alive.store(false, Ordering::Release);
            state.worker_target.hwnd.store(0, Ordering::Release);
        }
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
            if state.dirty || state.backbuffer.is_none() {
                render_ui(state);
                state.dirty = false;
            }
            if let Some(bb) = state.backbuffer.as_ref() {
                BitBlt(hdc, 0, 0, bb.w as i32, bb.h as i32, bb.hdc, 0, 0, SRCCOPY);
            }
            EndPaint(hwnd, &ps);
            if !state.trimmed_initial {
                state.trimmed_initial = true;
                trim_working_set();
            }
            return 0;
        }
        WM_SETCURSOR => {
            if (l_param as u32 & 0xFFFF) == 1 {
                let id = if (*ptr).hover.is_some() {
                    IDC_HAND
                } else {
                    IDC_ARROW
                };
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
            let x = ((l_param as i32 & 0xFFFF) as i16) as i32;
            let y = (((l_param as i32 >> 16) & 0xFFFF) as i16) as i32;
            let next_hover = hit_test(&state.hits, x, y);
            if next_hover != state.hover {
                state.hover = next_hover;
                state.dirty = true;
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
                state.dirty = true;
                InvalidateRect(hwnd, null(), 0);
            }
            return 0;
        }
        WM_LBUTTONDOWN => {
            let state = &mut *ptr;
            let x = ((l_param as i32 & 0xFFFF) as i16) as i32;
            let y = (((l_param as i32 >> 16) & 0xFFFF) as i16) as i32;
            let hit = hit_test(&state.hits, x, y);
            if state.pressed != hit || state.focus.is_some() {
                state.pressed = hit;
                state.focus = None;
                state.dirty = true;
                InvalidateRect(hwnd, null(), 0);
            }
            return 0;
        }
        WM_LBUTTONUP => {
            let x = ((l_param as i32 & 0xFFFF) as i16) as i32;
            let y = (((l_param as i32 >> 16) & 0xFFFF) as i16) as i32;
            let clicked = {
                let state = &mut *ptr;
                let h = hit_test(&state.hits, x, y);
                let match_click = if h == state.pressed { h } else { None };
                if state.pressed.is_some() {
                    state.pressed = None;
                    state.dirty = true;
                }
                match_click
            };
            if let Some(ctrl) = clicked {
                if activate_control(hwnd, ptr, ctrl) {
                    return 0;
                }
            }
            if (*ptr).dirty {
                InvalidateRect(hwnd, null(), 0);
            }
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
                state.dirty = true;
                if u.finished {
                    KillTimer(hwnd, TIMER_ANIM);
                    state.anim_active = false;
                    state.anim_pct = 1.0;
                    state.worker_error = u.error.clone();
                    state.is_restore = u.is_restore;
                    state.current_step = u.applied_step;
                    state.current_px = u.applied_px;
                    state.has_existing_install = !u.is_restore && u.error.is_none();
                    state.focus = None;
                    state.screen = Screen::Done;
                    trim_working_set();
                } else if (state.target_pct - state.anim_pct).abs() > 0.002 && !state.anim_active {
                    SetTimer(hwnd, TIMER_ANIM, 30, null_mut());
                    state.anim_active = true;
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
                state.dirty = true;
                InvalidateRect(hwnd, null(), 0);
            } else {
                state.anim_pct = state.target_pct;
                KillTimer(hwnd, TIMER_ANIM);
                state.anim_active = false;
            }
            return 0;
        }
        WM_KEYDOWN => {
            let state = &mut *ptr;
            if w_param == VK_ESCAPE && state.screen != Screen::Working {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
                return 0;
            }
            if state.screen == Screen::Working {
                return 0;
            }
            if w_param == VK_TAB {
                let order = focus_order(
                    state.screen,
                    state.has_existing_install,
                    state.worker_error.is_some(),
                    state.is_restore,
                );
                if !order.is_empty() {
                    let shift = GetKeyState(VK_SHIFT) < 0;
                    let next = match state.focus.and_then(|f| order.iter().position(|&c| c == f)) {
                        Some(pos) => {
                            if shift {
                                order[(pos + order.len() - 1) % order.len()]
                            } else {
                                order[(pos + 1) % order.len()]
                            }
                        }
                        None => {
                            if shift {
                                order[order.len() - 1]
                            } else {
                                order[0]
                            }
                        }
                    };
                    state.focus = Some(next);
                    state.dirty = true;
                    InvalidateRect(hwnd, null(), 0);
                }
                return 0;
            }
            if matches!(w_param, VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN)
                && state.screen == Screen::Setup
            {
                let cur_idx = match state.focus {
                    Some(ControlId::PresetCard(i)) => i,
                    Some(ControlId::BtnRestore) if matches!(w_param, VK_LEFT | VK_RIGHT) => {
                        state.focus = Some(ControlId::BtnInstall);
                        state.dirty = true;
                        InvalidateRect(hwnd, null(), 0);
                        return 0;
                    }
                    Some(ControlId::BtnInstall)
                        if state.has_existing_install && matches!(w_param, VK_LEFT | VK_RIGHT) =>
                    {
                        state.focus = Some(ControlId::BtnRestore);
                        state.dirty = true;
                        InvalidateRect(hwnd, null(), 0);
                        return 0;
                    }
                    _ => Preset::ALL
                        .iter()
                        .position(|&p| p == state.preset)
                        .unwrap_or(0),
                };
                let next_idx = if w_param == VK_LEFT || w_param == VK_RIGHT {
                    cur_idx ^ 1
                } else {
                    cur_idx ^ 2
                };
                state.preset = Preset::ALL[next_idx];
                state.focus = Some(ControlId::PresetCard(next_idx));
                state.dirty = true;
                InvalidateRect(hwnd, null(), 0);
                return 0;
            }
            if matches!(w_param, VK_LEFT | VK_RIGHT) && state.screen == Screen::Done {
                state.focus = Some(match state.focus {
                    Some(ControlId::BtnOpenFolder) => ControlId::BtnFinish,
                    _ => ControlId::BtnOpenFolder,
                });
                state.dirty = true;
                InvalidateRect(hwnd, null(), 0);
                return 0;
            }
            if w_param == VK_SPACE {
                if let Some(ctrl) = state.focus {
                    if !activate_control(hwnd, ptr, ctrl) && (*ptr).dirty {
                        InvalidateRect(hwnd, null(), 0);
                    }
                    return 0;
                }
            }
            if w_param == VK_RETURN {
                match state.focus {
                    Some(
                        ctrl @ (ControlId::ToggleStartup
                        | ControlId::ToggleUninstall
                        | ControlId::BtnRestore
                        | ControlId::BtnInstall
                        | ControlId::BtnOpenFolder
                        | ControlId::BtnFinish),
                    ) => {
                        if !activate_control(hwnd, ptr, ctrl) && (*ptr).dirty {
                            InvalidateRect(hwnd, null(), 0);
                        }
                        return 0;
                    }
                    _ => match state.screen {
                        Screen::Setup => start_install_worker(state, false),
                        Screen::Done => {
                            PostMessageW(hwnd, WM_CLOSE, 0, 0);
                        }
                        Screen::Working => {}
                    },
                }
                return 0;
            }
        }
        WM_DPICHANGED => {
            let state = &mut *ptr;
            state.dpi = (w_param & 0xFFFF) as u32;
            state.fonts.destroy();
            state.fonts = FontSet::create(state.dpi);
            state.glyph_cache.clear();
            state.dirty = true;
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

pub fn create_setup_window(state_ptr: *mut GuiState, show: bool) -> Result<*mut c_void, String> {
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
        let st = &mut *state_ptr;
        st.hwnd = hwnd;
        st.worker_target
            .hwnd
            .store(hwnd as usize, Ordering::Release);
        st.worker_target.alive.store(true, Ordering::Release);
        st.dpi = dpi;
        st.fonts = FontSet::create(dpi);
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

pub fn new_gui_state(
    preset: Preset,
    startup: bool,
    register_uninstall: bool,
    has_existing: bool,
    is_modify_mode: bool,
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
        is_modify_mode,
        current_step: step,
        current_px: px,
        hover: None,
        pressed: None,
        focus: None,
        tracking_mouse: false,
        dirty: true,
        anim_active: false,
        trimmed_initial: false,
        hits: Vec::new(),
        anim_pct: 0.0,
        target_pct: 0.0,
        worker_stage: 0,
        worker_status: String::new(),
        worker_error: None,
        is_restore: false,
        install_root,
        worker_target: Arc::new(WorkerTarget {
            alive: AtomicBool::new(false),
            hwnd: std::sync::atomic::AtomicUsize::new(0),
        }),
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
        glyph_cache: GlyphSpriteCache::default(),
        backbuffer: None,
        app_icon: create_split_cursor_hicon(32, Preset::Regular, false),
    }))
}

pub fn bring_existing_setup_to_front() -> bool {
    let cls = wide_z(WND_CLASS);
    // SAFETY: NUL-terminated class name passed to `FindWindowW` and `SetForegroundWindow`.
    unsafe {
        let existing = FindWindowW(cls.as_ptr(), null());
        if !existing.is_null() {
            SetForegroundWindow(existing);
            return true;
        }
    }
    false
}

pub fn run_smoke_ui(out_dir: &Path, dpi_override: Option<u32>) -> Result<(), String> {
    let _ = fs::create_dir_all(out_dir);
    let state_ptr = new_gui_state(
        Preset::Compact,
        true,
        true,
        true,
        false,
        PathBuf::from(r"C:\Users\Demo\AppData\Local\ModernInvertedCursors"),
    );
    let hwnd = create_setup_window(state_ptr, false)?;
    unsafe {
        let state = &mut *state_ptr;
        if let Some(dpi) = dpi_override {
            let clamped = dpi.clamp(96, 240);
            if clamped != state.dpi {
                state.dpi = clamped;
                state.fonts.destroy();
                state.fonts = FontSet::create(clamped);
                state.glyph_cache.clear();
            }
        }

        state.screen = Screen::Setup;
        state.focus = None;
        render_ui(state);
        if let Some(bb) = state.backbuffer.as_ref() {
            save_backbuffer_bmp(bb, &out_dir.join("01_setup_dark.bmp"))?;
        }

        state.focus = Some(ControlId::PresetCard(2));
        render_ui(state);
        if let Some(bb) = state.backbuffer.as_ref() {
            save_backbuffer_bmp(bb, &out_dir.join("04_setup_focus_dark.bmp"))?;
        }
        state.focus = None;

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
