#![windows_subsystem = "windows"]

use modern_inverted_cursor::{
    app_paths, app_paths_for_root, balanced_cursor_size, create_split_cursor_hicon,
    current_accessibility_step, is_startup_enabled, load_saved_preset, parse_preset_arg,
    perform_validated_restore, reg_delete_value, reg_get_string, reg_open_notify, reg_set_dword,
    reg_set_string, save_preset, set_startup_enabled, stop_targeted_guardians, trim_working_set,
    validate_cli_args, wide_z, AssetVerifyCache, BoundedLogger, NamedReadyEvent, Preset,
    RegCloseKey, RegNotifyChangeKeyValue, ACCESSIBILITY_STEPS, ACCESSIBILITY_SUBKEY, APP_VERSION,
    CONFIG_SUBKEY, CURSORS_SUBKEY, GUARDIAN_EXE_NAME, GUARDIAN_RUN_VALUE, PACK_VERSION, RUN_SUBKEY,
    SETUP_EXE_NAME, TRAY_WND_CLASS,
};

use std::{
    env,
    ffi::c_void,
    path::{Path, PathBuf},
    process::Command,
    ptr::{null, null_mut},
};

const MUTEX_NAME: &str = "Local\\ModernInvertedCursorGuardian_V6_Mutex";
const SANDBOX_MUTEX_NAME: &str = "Local\\ModernInvertedCursorGuardian_Sandbox_Mutex";
const SANDBOX_WND_CLASS: &str = "ModernInvertedCursorGuardianSandboxWnd";
const TRAY_ICON_ID: u32 = 1;

const WM_DESTROY: u32 = 0x0002;
const WM_CLOSE: u32 = 0x0010;
const WM_QUIT: u32 = 0x0012;
const WM_ENDSESSION: u32 = 0x0016;
const WM_SETTINGCHANGE: u32 = 0x001A;
const WM_DISPLAYCHANGE: u32 = 0x007E;
const WM_TIMER: u32 = 0x0113;
const WM_POWERBROADCAST: u32 = 0x0218;
const WM_WTSSESSION_CHANGE: u32 = 0x02B1;
const WM_THEMECHANGED: u32 = 0x031A;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_LBUTTONDBLCLK: u32 = 0x0203;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_APP: u32 = 0x8000;
const WM_TRAYICON: u32 = WM_APP + 1;
const WM_APP_REAPPLY: u32 = WM_APP + 2;
const WM_APP_SHOW_MENU: u32 = WM_APP + 3;

const TIMER_REAPPLY: usize = 1;
const TIMER_TRAY_RETRY: usize = 2;
const DEBOUNCE_MS: u32 = 400;
const TRAY_RETRY_MS: u32 = 2000;
const MAX_TRAY_RETRIES: u8 = 15;

const PBT_APMRESUMESUSPEND: usize = 0x0007;
const PBT_APMRESUMEAUTOMATIC: usize = 0x0012;
const WTS_SESSION_UNLOCK: usize = 0x8;
const NOTIFY_FOR_THIS_SESSION: u32 = 0;

const WS_POPUP: u32 = 0x8000_0000;
const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;
const WS_EX_NOACTIVATE: u32 = 0x0800_0000;
const GWLP_USERDATA: i32 = -21;

const NIM_ADD: u32 = 0x0000_0000;
const NIM_MODIFY: u32 = 0x0000_0001;
const NIM_DELETE: u32 = 0x0000_0002;
const NIF_MESSAGE: u32 = 0x0000_0001;
const NIF_ICON: u32 = 0x0000_0002;
const NIF_TIP: u32 = 0x0000_0004;

const MF_STRING: u32 = 0x0000_0000;
const MF_GRAYED: u32 = 0x0000_0001;
const MF_CHECKED: u32 = 0x0000_0008;
const MF_POPUP: u32 = 0x0000_0010;
const MF_SEPARATOR: u32 = 0x0000_0800;

const TPM_RIGHTBUTTON: u32 = 0x0002;
const TPM_BOTTOMALIGN: u32 = 0x0020;
const TPM_NONOTIFY: u32 = 0x0080;
const TPM_RETURNCMD: u32 = 0x0100;

const PM_REMOVE: u32 = 0x0001;
const MWMO_INPUTAVAILABLE: u32 = 0x0004;
const QS_ALLINPUT_COMPAT: u32 = 0x04FF;
const INFINITE: u32 = 0xFFFF_FFFF;
const WAIT_OBJECT_0: u32 = 0;
const ERROR_ALREADY_EXISTS: u32 = 183;

const MB_YESNO: u32 = 0x0000_0004;
const MB_ICONERROR: u32 = 0x0000_0010;
const MB_ICONQUESTION: u32 = 0x0000_0020;
const MB_SETFOREGROUND: u32 = 0x0001_0000;
const IDYES: i32 = 6;

const CMD_REAPPLY: usize = 1001;
const CMD_PAUSE: usize = 1002;
const CMD_PRESET_REGULAR: usize = 1010;
const CMD_PRESET_NO_TAIL: usize = 1011;
const CMD_PRESET_DETACHED: usize = 1012;
const CMD_PRESET_COMPACT: usize = 1013;
const CMD_STARTUP: usize = 1020;
const CMD_OPEN_SETUP: usize = 1021;
const CMD_OPEN_FOLDER: usize = 1022;
const CMD_VIEW_LOG: usize = 1023;
const CMD_RESTORE: usize = 1024;
const CMD_EXIT: usize = 1099;

const REG_NOTIFY_CHANGE_LAST_SET: u32 = 0x0000_0004;

type WndProc = unsafe extern "system" fn(*mut c_void, u32, usize, isize) -> isize;

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
struct NotifyIconDataW {
    cb_size: u32,
    h_wnd: *mut c_void,
    u_id: u32,
    u_flags: u32,
    u_callback_message: u32,
    h_icon: *mut c_void,
    sz_tip: [u16; 128],
    dw_state: u32,
    dw_state_mask: u32,
    sz_info: [u16; 256],
    u_timeout_or_version: u32,
    sz_info_title: [u16; 64],
    dw_info_flags: u32,
    guid_item: [u8; 16],
    h_balloon_icon: *mut c_void,
}

#[link(name = "kernel32")]
extern "system" {
    fn GetModuleHandleW(lp_module_name: *const u16) -> *mut c_void;
    fn GetLastError() -> u32;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const u8) -> *mut c_void;
    fn CreateMutexW(
        lp_mutex_attributes: *mut c_void,
        b_initial_owner: i32,
        lp_name: *const u16,
    ) -> *mut c_void;
    fn CreateEventW(
        lp_event_attributes: *mut c_void,
        b_manual_reset: i32,
        b_initial_state: i32,
        lp_name: *const u16,
    ) -> *mut c_void;
    fn CloseHandle(h_object: *mut c_void) -> i32;
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
    fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> *mut c_void;
    fn DefWindowProcW(h_wnd: *mut c_void, msg: u32, w_param: usize, l_param: isize) -> isize;
    fn DestroyWindow(h_wnd: *mut c_void) -> i32;
    fn PostQuitMessage(n_exit_code: i32);
    fn PostMessageW(h_wnd: *mut c_void, msg: u32, w_param: usize, l_param: isize) -> i32;
    fn PeekMessageW(
        lp_msg: *mut Msg,
        h_wnd: *mut c_void,
        w_msg_filter_min: u32,
        w_msg_filter_max: u32,
        w_remove_msg: u32,
    ) -> i32;
    fn TranslateMessage(lp_msg: *const Msg) -> i32;
    fn DispatchMessageW(lp_msg: *const Msg) -> isize;
    fn MsgWaitForMultipleObjectsEx(
        n_count: u32,
        p_handles: *const *mut c_void,
        dw_milliseconds: u32,
        dw_wake_mask: u32,
        dw_flags: u32,
    ) -> u32;
    fn RegisterWindowMessageW(lp_string: *const u16) -> u32;
    fn SetTimer(
        h_wnd: *mut c_void,
        n_id_event: usize,
        u_elapse: u32,
        lp_timer_func: *mut c_void,
    ) -> usize;
    fn KillTimer(h_wnd: *mut c_void, u_id_event: usize) -> i32;
    #[cfg(target_pointer_width = "64")]
    fn SetWindowLongPtrW(h_wnd: *mut c_void, n_index: i32, dw_new_long: isize) -> isize;
    #[cfg(target_pointer_width = "64")]
    fn GetWindowLongPtrW(h_wnd: *mut c_void, n_index: i32) -> isize;
    #[cfg(target_pointer_width = "32")]
    fn SetWindowLongW(h_wnd: *mut c_void, n_index: i32, dw_new_long: isize) -> isize;
    #[cfg(target_pointer_width = "32")]
    fn GetWindowLongW(h_wnd: *mut c_void, n_index: i32) -> isize;
    fn SetForegroundWindow(h_wnd: *mut c_void) -> i32;
    fn GetCursorPos(lp_point: *mut Point) -> i32;
    fn CreatePopupMenu() -> *mut c_void;
    fn DestroyMenu(h_menu: *mut c_void) -> i32;
    fn AppendMenuW(
        h_menu: *mut c_void,
        u_flags: u32,
        u_id_new_item: usize,
        lp_new_item: *const u16,
    ) -> i32;
    fn TrackPopupMenuEx(
        h_menu: *mut c_void,
        u_flags: u32,
        x: i32,
        y: i32,
        hwnd: *mut c_void,
        lptpm: *const c_void,
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

#[link(name = "shell32")]
extern "system" {
    fn Shell_NotifyIconW(dw_message: u32, lp_data: *const NotifyIconDataW) -> i32;
    fn ShellExecuteW(
        hwnd: *mut c_void,
        lp_operation: *const u16,
        lp_file: *const u16,
        lp_parameters: *const u16,
        lp_directory: *const u16,
        n_show_cmd: i32,
    ) -> isize;
}

unsafe fn wts_session_notify(hwnd: *mut c_void, register: bool) -> bool {
    let lib_name = wide_z("wtsapi32.dll");
    if register {
        let module = modern_inverted_cursor::load_system_dll("wtsapi32.dll");
        if module.is_null() {
            return false;
        }
        type RegisterFn = unsafe extern "system" fn(*mut c_void, u32) -> i32;
        let proc = GetProcAddress(module, b"WTSRegisterSessionNotification\0".as_ptr());
        if !proc.is_null() {
            let f: RegisterFn = std::mem::transmute(proc);
            return f(hwnd, NOTIFY_FOR_THIS_SESSION) != 0;
        }
        false
    } else {
        let module = GetModuleHandleW(lib_name.as_ptr());
        if module.is_null() {
            return false;
        }
        type UnregisterFn = unsafe extern "system" fn(*mut c_void) -> i32;
        let proc = GetProcAddress(module, b"WTSUnRegisterSessionNotification\0".as_ptr());
        if !proc.is_null() {
            let f: UnregisterFn = std::mem::transmute(proc);
            return f(hwnd) != 0;
        }
        false
    }
}

struct AppState {
    hwnd: *mut c_void,
    preset: Preset,
    paused: bool,
    last_step: u32,
    last_px: u32,
    root: PathBuf,
    install_dir: PathBuf,
    backup_path: PathBuf,
    guardian_exe: PathBuf,
    log: BoundedLogger,
    asset_cache: AssetVerifyCache,
    sandbox_mode: bool,
    icon_active: *mut c_void,
    icon_paused: *mut c_void,
    wm_taskbar_created: u32,
    menu_open: bool,
    tray_added: bool,
    tray_retries: u8,
    last_tray_state: Option<(bool, Preset, u32, u32)>,
}

impl AppState {
    fn current_icon(&self) -> *mut c_void {
        if self.paused {
            self.icon_paused
        } else {
            self.icon_active
        }
    }

    fn tooltip_text(&self) -> String {
        if self.paused {
            format!(
                "Modern Inverted v{PACK_VERSION} (Paused)\nPreset: {}",
                self.preset.short_name()
            )
        } else {
            format!(
                "Modern Inverted v{PACK_VERSION}\n{} • {}px ({}/{ACCESSIBILITY_STEPS})",
                self.preset.short_name(),
                self.last_px,
                self.last_step
            )
        }
    }

    fn sync_tray(&mut self, prefer_modify: bool) {
        if self.hwnd.is_null() {
            return;
        }
        let tray_key = (self.paused, self.preset, self.last_px, self.last_step);
        if prefer_modify && self.tray_added && self.last_tray_state == Some(tray_key) {
            return;
        }
        let message = if prefer_modify && self.tray_added {
            NIM_MODIFY
        } else {
            NIM_ADD
        };
        let mut nid: NotifyIconDataW = unsafe { std::mem::zeroed() };
        nid.cb_size = std::mem::size_of::<NotifyIconDataW>() as u32;
        nid.h_wnd = self.hwnd;
        nid.u_id = TRAY_ICON_ID;
        nid.u_flags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.u_callback_message = WM_TRAYICON;
        nid.h_icon = self.current_icon();

        let tip = self.tooltip_text();
        for (dst, ch) in nid.sz_tip[..127].iter_mut().zip(tip.encode_utf16()) {
            *dst = ch;
        }
        let ok = unsafe { Shell_NotifyIconW(message, &nid) != 0 };
        if ok {
            self.tray_added = true;
            self.tray_retries = 0;
            self.last_tray_state = Some(tray_key);
            unsafe {
                KillTimer(self.hwnd, TIMER_TRAY_RETRY);
            }
        } else if !self.tray_added && self.tray_retries < MAX_TRAY_RETRIES {
            self.tray_retries += 1;
            unsafe {
                SetTimer(self.hwnd, TIMER_TRAY_RETRY, TRAY_RETRY_MS, null_mut());
            }
        }
    }

    fn apply_now(&mut self, force_full_hash: bool) -> Result<(), String> {
        if let Err(err) = self
            .asset_cache
            .ensure_all_assets_cached(&self.install_dir, force_full_hash, &self.log)
        {
            self.log.line(format!("[error] Asset check failed: {err}"));
            return Err(err);
        }
        if self.sandbox_mode {
            let step = current_accessibility_step();
            let px = balanced_cursor_size(step);
            let (arrow, help, working) = self.preset.custom_files();
            for req in [arrow, help, working] {
                if !self.install_dir.join(req).is_file() {
                    return Err(format!("Sandbox asset missing: {req}"));
                }
            }
            let _ = std::fs::write(
                self.root.join("sandbox-runtime.txt"),
                format!(
                    "pid={}\nversion={APP_VERSION}\npreset={}\nstep={step}\npx={px}\n",
                    std::process::id(),
                    self.preset.short_name()
                ),
            );
            self.last_step = step;
            self.last_px = px;
            self.sync_tray(true);
            trim_working_set();
            return Ok(());
        }
        match modern_inverted_cursor::register_and_apply(self.preset, &self.install_dir, Some(&self.log)) {
            Ok((step, px)) => {
                self.last_step = step;
                self.last_px = px;
                self.sync_tray(true);
                trim_working_set();
                Ok(())
            }
            Err(err) => {
                self.log.line(format!("[error] Apply failed: {err}"));
                Err(err)
            }
        }
    }
}

fn append_menu_str(hmenu: *mut c_void, flags: u32, id: usize, text: &str) {
    let w = wide_z(text);
    unsafe {
        AppendMenuW(hmenu, flags, id, w.as_ptr());
    }
}

fn run_popup_menu(
    hwnd: *mut c_void,
    preset: Preset,
    paused: bool,
    last_px: u32,
    last_step: u32,
    startup: bool,
) -> usize {
    let status_label = if paused {
        format!(
            "Modern Inverted v{PACK_VERSION} (Paused) — {}",
            preset.short_name()
        )
    } else {
        format!(
            "Modern Inverted v{PACK_VERSION} — {} ({}px, {}/{ACCESSIBILITY_STEPS})",
            preset.short_name(),
            last_px,
            last_step
        )
    };

    let hmenu = unsafe { CreatePopupMenu() };
    let hpreset = unsafe { CreatePopupMenu() };
    if hmenu.is_null() || hpreset.is_null() {
        unsafe {
            if !hmenu.is_null() {
                DestroyMenu(hmenu);
            }
            if !hpreset.is_null() {
                DestroyMenu(hpreset);
            }
        }
        return 0;
    }

    let presets = [
        (Preset::Compact, CMD_PRESET_COMPACT, "Compact (Recommended)"),
        (Preset::NoTail, CMD_PRESET_NO_TAIL, "No Tail"),
        (Preset::TailDetached, CMD_PRESET_DETACHED, "Tail Detached"),
        (Preset::Regular, CMD_PRESET_REGULAR, "Regular"),
    ];
    for (p, id, label) in presets {
        let flags = MF_STRING | if preset == p { MF_CHECKED } else { 0 };
        append_menu_str(hpreset, flags, id, label);
    }

    append_menu_str(hmenu, MF_STRING | MF_GRAYED, 0, &status_label);
    append_menu_str(hmenu, MF_SEPARATOR, 0, "");
    append_menu_str(hmenu, MF_STRING, CMD_REAPPLY, "Reapply Cursors Now");
    append_menu_str(
        hmenu,
        MF_STRING | if paused { MF_CHECKED } else { 0 },
        CMD_PAUSE,
        "Pause Guardian",
    );
    append_menu_str(hmenu, MF_POPUP, hpreset as usize, "Preset");
    append_menu_str(
        hmenu,
        MF_STRING | if startup { MF_CHECKED } else { 0 },
        CMD_STARTUP,
        "Start at Windows Sign-in",
    );
    append_menu_str(hmenu, MF_SEPARATOR, 0, "");
    append_menu_str(hmenu, MF_STRING, CMD_OPEN_SETUP, "Open Setup & Maintenance...");
    append_menu_str(hmenu, MF_STRING, CMD_OPEN_FOLDER, "Open Install Folder");
    append_menu_str(hmenu, MF_STRING, CMD_VIEW_LOG, "View Guardian Log");
    append_menu_str(hmenu, MF_STRING, CMD_RESTORE, "Restore Original Cursors...");
    append_menu_str(hmenu, MF_SEPARATOR, 0, "");
    append_menu_str(hmenu, MF_STRING, CMD_EXIT, "Exit");

    let mut pt = Point { x: 0, y: 0 };
    unsafe {
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd);
    }
    let cmd = unsafe {
        TrackPopupMenuEx(
            hmenu,
            TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RETURNCMD | TPM_NONOTIFY,
            pt.x,
            pt.y,
            hwnd,
            null(),
        )
    } as usize;
    unsafe {
        PostMessageW(hwnd, 0, 0, 0);
        DestroyMenu(hmenu);
    }
    cmd
}

fn handle_menu_action(hwnd: *mut c_void, cmd: usize) {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState };
    if ptr.is_null() {
        return;
    }

    if cmd == CMD_RESTORE {
        let setup_exe = {
            let state = unsafe { &*ptr };
            state.root.join(SETUP_EXE_NAME)
        };
        if setup_exe.is_file() {
            if let Err(err) = Command::new(&setup_exe).arg("--uninstall").spawn() {
                show_error_box(hwnd, &format!("Could not launch {}: {err}", setup_exe.display()));
            }
            return;
        }
        let title = wide_z("Modern Inverted Cursors");
        let msg = wide_z(
            "Restore your original Windows cursors, disable startup launch, and exit the tray guardian?",
        );
        let ans = unsafe {
            MessageBoxW(
                hwnd,
                msg.as_ptr(),
                title.as_ptr(),
                MB_YESNO | MB_ICONQUESTION | MB_SETFOREGROUND,
            )
        };
        if ans == IDYES {
            let ptr_after = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState };
            if !ptr_after.is_null() {
                let state = unsafe { &mut *ptr_after };
                let res = perform_validated_restore(
                    &state.backup_path,
                    Some(&state.log),
                    || Ok(0),
                    || {
                        reg_delete_value(RUN_SUBKEY, GUARDIAN_RUN_VALUE);
                        reg_delete_value(CONFIG_SUBKEY, "Preset");
                        Ok(())
                    },
                );
                match res {
                    Ok(()) => unsafe {
                        PostMessageW(hwnd, WM_CLOSE, 0, 0);
                    },
                    Err(err) => show_error_box(hwnd, &err),
                }
            }
        }
        return;
    }

    let state = unsafe { &mut *ptr };
    match cmd {
        CMD_REAPPLY => {
            state.paused = false;
            let _ = state.apply_now(true);
        }
        CMD_PAUSE => {
            state.paused = !state.paused;
            state.log.line(if state.paused {
                "Guardian paused by user."
            } else {
                "Guardian resumed by user."
            });
            if !state.paused {
                let _ = state.apply_now(false);
            } else {
                state.sync_tray(true);
            }
            trim_working_set();
        }
        CMD_PRESET_REGULAR | CMD_PRESET_NO_TAIL | CMD_PRESET_DETACHED | CMD_PRESET_COMPACT => {
            let next = match cmd {
                CMD_PRESET_REGULAR => Preset::Regular,
                CMD_PRESET_NO_TAIL => Preset::NoTail,
                CMD_PRESET_DETACHED => Preset::TailDetached,
                _ => Preset::Compact,
            };
            state.preset = next;
            state.paused = false;
            if !state.sandbox_mode {
                let _ = save_preset(next);
                if is_startup_enabled() {
                    let _ = set_startup_enabled(&state.guardian_exe, true);
                }
            }
            let _ = state.apply_now(false);
        }
        CMD_STARTUP => {
            if state.sandbox_mode {
                return;
            }
            let next = !is_startup_enabled();
            if let Err(err) = set_startup_enabled(&state.guardian_exe, next) {
                show_error_box(hwnd, &err);
            } else {
                state.log.line(if next {
                    "Startup at sign-in enabled."
                } else {
                    "Startup at sign-in disabled."
                });
            }
        }
        CMD_OPEN_SETUP => {
            let setup_exe = state.root.join(SETUP_EXE_NAME);
            if setup_exe.is_file() {
                if let Err(err) = Command::new(&setup_exe).arg("--modify").spawn() {
                    show_error_box(hwnd, &format!("Could not launch {}: {err}", setup_exe.display()));
                }
            } else {
                open_shell_path(hwnd, &state.root);
            }
        }
        CMD_OPEN_FOLDER => open_shell_path(hwnd, &state.root),
        CMD_VIEW_LOG => open_shell_path(hwnd, state.log.path()),
        CMD_EXIT => unsafe {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
        },
        _ => {}
    }
}

fn open_shell_path(hwnd: *mut c_void, path: &Path) {
    let op = wide_z("open");
    let target = wide_z(&path.to_string_lossy());
    unsafe {
        ShellExecuteW(hwnd, op.as_ptr(), target.as_ptr(), null(), null(), 5);
    }
}

fn show_error_box(hwnd: *mut c_void, text: &str) {
    let title = wide_z("Modern Inverted Cursor Guardian");
    let body = wide_z(text);
    unsafe {
        MessageBoxW(
            hwnd,
            body.as_ptr(),
            title.as_ptr(),
            MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
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
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
    if !ptr.is_null() {
        if msg == (*ptr).wm_taskbar_created && (*ptr).wm_taskbar_created != 0 {
            let state = &mut *ptr;
            state.tray_added = false;
            state.tray_retries = 0;
            state.sync_tray(false);
            return 0;
        }
        match msg {
            WM_TRAYICON => {
                let ev = (l_param as u32) & 0xFFFF;
                if ev == WM_LBUTTONUP || ev == WM_RBUTTONUP {
                    PostMessageW(hwnd, WM_APP_SHOW_MENU, 0, 0);
                } else if ev == WM_LBUTTONDBLCLK {
                    let state = &mut *ptr;
                    state.paused = false;
                    let _ = state.apply_now(true);
                }
                return 0;
            }
            WM_APP_SHOW_MENU => {
                let snapshot = {
                    let state = &mut *ptr;
                    if state.menu_open {
                        return 0;
                    }
                    state.menu_open = true;
                    (state.preset, state.paused, state.last_px, state.last_step)
                };
                let cmd = run_popup_menu(
                    hwnd,
                    snapshot.0,
                    snapshot.1,
                    snapshot.2,
                    snapshot.3,
                    is_startup_enabled(),
                );
                if cmd != 0 {
                    handle_menu_action(hwnd, cmd);
                }
                let ptr_after = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState;
                if !ptr_after.is_null() {
                    (*ptr_after).menu_open = false;
                }
                trim_working_set();
                return 0;
            }
            WM_APP_REAPPLY => {
                let state = &mut *ptr;
                state.paused = false;
                let _ = state.apply_now(false);
                return 0;
            }
            WM_TIMER if w_param == TIMER_REAPPLY => {
                KillTimer(hwnd, TIMER_REAPPLY);
                let state = &mut *ptr;
                if !state.paused {
                    let _ = state.apply_now(false);
                }
                return 0;
            }
            WM_TIMER if w_param == TIMER_TRAY_RETRY => {
                KillTimer(hwnd, TIMER_TRAY_RETRY);
                let state = &mut *ptr;
                if !state.tray_added {
                    state.sync_tray(false);
                }
                return 0;
            }
            WM_WTSSESSION_CHANGE if w_param == WTS_SESSION_UNLOCK => {
                if !(*ptr).paused {
                    SetTimer(hwnd, TIMER_REAPPLY, 500, null_mut());
                }
                return 0;
            }
            WM_DISPLAYCHANGE => {
                if !(*ptr).paused {
                    SetTimer(hwnd, TIMER_REAPPLY, 500, null_mut());
                }
                return 0;
            }
            WM_SETTINGCHANGE | WM_THEMECHANGED => {
                if !(*ptr).paused {
                    let arrow_now = reg_get_string(CURSORS_SUBKEY, Some("Arrow")).unwrap_or_default();
                    if !arrow_now.contains("ModernInvertedCursors")
                        || current_accessibility_step() != (*ptr).last_step
                    {
                        SetTimer(hwnd, TIMER_REAPPLY, DEBOUNCE_MS, null_mut());
                    }
                }
            }
            WM_POWERBROADCAST
                if w_param == PBT_APMRESUMEAUTOMATIC || w_param == PBT_APMRESUMESUSPEND =>
            {
                if !(*ptr).paused {
                    SetTimer(hwnd, TIMER_REAPPLY, 500, null_mut());
                }
                return 0;
            }
            WM_CLOSE | WM_ENDSESSION => {
                let mut nid: NotifyIconDataW = std::mem::zeroed();
                nid.cb_size = std::mem::size_of::<NotifyIconDataW>() as u32;
                nid.h_wnd = hwnd;
                nid.u_id = TRAY_ICON_ID;
                Shell_NotifyIconW(NIM_DELETE, &nid);
                wts_session_notify(hwnd, false);
                if msg == WM_CLOSE {
                    DestroyWindow(hwnd);
                    return 0;
                }
            }
            _ => {}
        }
    }
    DefWindowProcW(hwnd, msg, w_param, l_param)
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

fn sandbox_arg(args: &[String]) -> Option<PathBuf> {
    if let Some(val) = arg_value(args, "--sandbox") {
        return Some(PathBuf::from(val));
    }
    if args.iter().any(|a| a == "--sandbox") {
        return Some(env::temp_dir().join("mic-guardian-sandbox"));
    }
    None
}

fn validate_guardian_args(args: &[String]) -> Result<(), String> {
    validate_cli_args(
        args,
        &[
            "--tray",
            "--replace",
            "--quit",
            "--compact",
            "--no-tail",
            "--tail-detached",
            "--detached",
            "--regular",
        ],
        &["--ready-event", "--preset"],
        &["--sandbox"],
    )
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    validate_guardian_args(&args)?;
    let sandbox_dir = sandbox_arg(&args);
    let sandbox_mode = sandbox_dir.is_some();
    let (root, install_dir, backup_path) = match sandbox_dir {
        Some(dir) => app_paths_for_root(dir),
        None => app_paths()?,
    };
    let log = BoundedLogger::new(root.join("guardian.log"), false);
    let mutex_name = if sandbox_mode { SANDBOX_MUTEX_NAME } else { MUTEX_NAME };
    let wnd_class = if sandbox_mode { SANDBOX_WND_CLASS } else { TRAY_WND_CLASS };

    if args.iter().any(|a| a == "--quit") {
        if sandbox_mode {
            let w_cls = wide_z(SANDBOX_WND_CLASS);
            let existing = unsafe { FindWindowW(w_cls.as_ptr(), null()) };
            if !existing.is_null() {
                unsafe {
                    PostMessageW(existing, WM_CLOSE, 0, 0);
                }
            }
            return Ok(());
        }
        stop_targeted_guardians(&root, Some(&log))?;
        return Ok(());
    }

    if args.iter().any(|a| a == "--replace") && !sandbox_mode {
        stop_targeted_guardians(&root, Some(&log))?;
    }

    let ready_event = arg_value(&args, "--ready-event");

    let w_mutex = wide_z(mutex_name);
    let mutex = unsafe { CreateMutexW(null_mut(), 0, w_mutex.as_ptr()) };
    if !mutex.is_null() && unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            CloseHandle(mutex);
        }
        if ready_event.is_some() {
            return Err("Another guardian instance remained active and blocked startup.".into());
        }
        let w_cls = wide_z(wnd_class);
        let existing = unsafe { FindWindowW(w_cls.as_ptr(), null()) };
        if !existing.is_null() {
            unsafe {
                PostMessageW(existing, WM_APP_REAPPLY, 0, 0);
            }
        }
        return Ok(());
    }

    let preset = match parse_preset_arg(&args)? {
        Some(p) => {
            if !sandbox_mode {
                let _ = save_preset(p);
            }
            p
        }
        None => {
            if sandbox_mode {
                Preset::Compact
            } else {
                load_saved_preset()
            }
        }
    };

    let current_exe = env::current_exe().unwrap_or_else(|_| root.join(GUARDIAN_EXE_NAME));
    let reg_key = reg_open_notify(ACCESSIBILITY_SUBKEY)
        .map_err(|s| format!("RegCreateKeyExW(Accessibility) failed: status={s}"))?;
    let reg_event = unsafe { CreateEventW(null_mut(), 0, 0, null()) };
    if reg_event.is_null() {
        unsafe {
            RegCloseKey(reg_key);
            if !mutex.is_null() {
                CloseHandle(mutex);
            }
        }
        return Err("CreateEventW failed.".into());
    }

    let h_inst = unsafe { GetModuleHandleW(null()) };
    let w_cls = wide_z(wnd_class);
    let wc = WndClassExW {
        cb_size: std::mem::size_of::<WndClassExW>() as u32,
        style: 0,
        lpfn_wnd_proc: wnd_proc,
        cb_cls_extra: 0,
        cb_wnd_extra: 0,
        h_instance: h_inst,
        h_icon: null_mut(),
        h_cursor: null_mut(),
        hbr_background: null_mut(),
        lpsz_menu_name: null(),
        lpsz_class_name: w_cls.as_ptr(),
        h_icon_sm: null_mut(),
    };
    unsafe {
        RegisterClassExW(&wc);
    }

    let w_title = wide_z("Modern Inverted Cursor Guardian");
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w_cls.as_ptr(),
            w_title.as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            h_inst,
            null_mut(),
        )
    };
    if hwnd.is_null() {
        unsafe {
            CloseHandle(reg_event);
            RegCloseKey(reg_key);
            if !mutex.is_null() {
                CloseHandle(mutex);
            }
        }
        return Err("CreateWindowExW failed.".into());
    }

    let wm_taskbar_created = unsafe { RegisterWindowMessageW(wide_z("TaskbarCreated").as_ptr()) };
    let step = current_accessibility_step();
    let px = balanced_cursor_size(step);

    let state_ptr = Box::into_raw(Box::new(AppState {
        hwnd,
        preset,
        paused: false,
        last_step: step,
        last_px: px,
        root,
        install_dir,
        backup_path,
        guardian_exe: current_exe,
        log,
        asset_cache: AssetVerifyCache::new(),
        sandbox_mode,
        icon_active: create_split_cursor_hicon(32, Preset::Regular, false),
        icon_paused: create_split_cursor_hicon(32, Preset::Regular, true),
        wm_taskbar_created,
        menu_open: false,
        tray_added: false,
        tray_retries: 0,
        last_tray_state: None,
    }));

    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
        let _wts_registered = wts_session_notify(hwnd, true);
        (*state_ptr).log.line(format!(
            "Guardian v{APP_VERSION} started in {} mode (preset={}, size={}px).",
            if sandbox_mode { "sandbox" } else { "tray" },
            (*state_ptr).preset.short_name(),
            (*state_ptr).last_px
        ));
        (*state_ptr).sync_tray(false);
        let init_res = (*state_ptr).apply_now(true);
        if let Err(err) = init_res {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DestroyWindow(hwnd);
            let mut state = Box::from_raw(state_ptr);
            if !state.icon_active.is_null() {
                DestroyIcon(state.icon_active);
            }
            if !state.icon_paused.is_null() {
                DestroyIcon(state.icon_paused);
            }
            let _ = &mut state;
            CloseHandle(reg_event);
            RegCloseKey(reg_key);
            if !mutex.is_null() {
                CloseHandle(mutex);
            }
            return Err(format!("Guardian initial cursor application failed: {err}"));
        }
        if !sandbox_mode {
            let _ = reg_set_string(CONFIG_SUBKEY, Some("RuntimeVersion"), false, APP_VERSION);
            let _ = reg_set_dword(
                CONFIG_SUBKEY,
                "RuntimePid",
                std::process::id(),
            );
        }
        if let Some(ev_name) = ready_event.as_deref() {
            NamedReadyEvent::signal_by_name(ev_name)?;
        }
    }

    let arm_notify = |rk: *mut c_void, ev: *mut c_void| -> i32 {
        unsafe { RegNotifyChangeKeyValue(rk, 0, REG_NOTIFY_CHANGE_LAST_SET, ev, 1) }
    };
    let mut notify_ok = arm_notify(reg_key, reg_event) == 0;

    let mut msg: Msg = unsafe { std::mem::zeroed() };
    'main_loop: loop {
        while unsafe { PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) } != 0 {
            if msg.message == WM_QUIT {
                break 'main_loop;
            }
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }

        let wait = if notify_ok {
            unsafe {
                MsgWaitForMultipleObjectsEx(
                    1,
                    &reg_event,
                    INFINITE,
                    QS_ALLINPUT_COMPAT,
                    MWMO_INPUTAVAILABLE,
                )
            }
        } else {
            unsafe {
                MsgWaitForMultipleObjectsEx(
                    0,
                    null(),
                    INFINITE,
                    QS_ALLINPUT_COMPAT,
                    MWMO_INPUTAVAILABLE,
                )
            }
        };

        if notify_ok && wait == WAIT_OBJECT_0 {
            notify_ok = arm_notify(reg_key, reg_event) == 0;
            if unsafe { !(*state_ptr).paused } {
                unsafe {
                    SetTimer(hwnd, TIMER_REAPPLY, DEBOUNCE_MS, null_mut());
                }
            }
        }
    }

    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        let state = Box::from_raw(state_ptr);
        if !state.icon_active.is_null() {
            DestroyIcon(state.icon_active);
        }
        if !state.icon_paused.is_null() {
            DestroyIcon(state.icon_paused);
        }
        CloseHandle(reg_event);
        RegCloseKey(reg_key);
        if !mutex.is_null() {
            CloseHandle(mutex);
        }
    }
    Ok(())
}

fn main() {
    if let Err(err) = run() {
        show_error_box(null_mut(), &err);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_win32_tray_struct_sizes_match_sdk() {
        #[cfg(target_pointer_width = "64")]
        {
            assert_eq!(std::mem::size_of::<NotifyIconDataW>(), 976);
            assert_eq!(std::mem::size_of::<Msg>(), 48);
        }
        #[cfg(target_pointer_width = "32")]
        {
            assert_eq!(std::mem::size_of::<NotifyIconDataW>(), 956);
            assert_eq!(std::mem::size_of::<Msg>(), 28);
        }
    }
}
