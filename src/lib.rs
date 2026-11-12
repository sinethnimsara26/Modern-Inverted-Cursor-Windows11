#![allow(dead_code)]

use std::{
    env,
    ffi::c_void,
    fs::{self, OpenOptions},
    io::Write,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Duration,
};

pub const PACK_VERSION: &str = "3.6.2";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PUBLISHER: &str = "SN-Tech / sinethnimsara26";
pub const REPO_RAW: &str =
    "https://raw.githubusercontent.com/emvaized/modern_inverted_mouse_cursors/3.6.2/src";

pub const GUARDIAN_EXE_NAME: &str = "ModernInvertedCursorGuardian.exe";
pub const GUARDIAN_RUN_VALUE: &str = "ModernInvertedCursorGuardian";
pub const TRAY_WND_CLASS: &str = "ModernInvertedCursorGuardianTrayWnd";

pub const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const CONFIG_SUBKEY: &str = r"Software\ModernInvertedCursors";
pub const UNINSTALL_PARENT_SUBKEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
pub const UNINSTALL_SUBKEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall\ModernInvertedCursors";
pub const UNINSTALL_KEY_NAME: &str = "ModernInvertedCursors";
pub const CURSORS_SUBKEY: &str = r"Control Panel\Cursors";
pub const SCHEMES_SUBKEY: &str = r"Control Panel\Cursors\Schemes";
pub const ACCESSIBILITY_SUBKEY: &str = r"Software\Microsoft\Accessibility";

// Balanced 15-position Accessibility slider -> 32..88px in 4px increments.
pub const BALANCED_MIN_SIZE: u32 = 32;
pub const BALANCED_STEP_PX: u32 = 4;
pub const ACCESSIBILITY_STEPS: u32 = 15;

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const MAX_LOG_BYTES: u64 = 32 * 1024;

#[derive(Clone, Copy)]
pub struct Asset {
    pub name: &'static str,
    pub size: u64,
    pub git_sha1: &'static str,
}

pub const ASSETS: &[Asset] = &[
    Asset { name: "arrow.cur", size: 2246, git_sha1: "325739ebbc8a589aa757b7a7f19194bd0e6afd3a" },
    Asset { name: "arrow_no_tail.cur", size: 2384, git_sha1: "8a503406723a840324b966c46074ca7257b8b145" },
    Asset { name: "arrow_no_tail_smaller.cur", size: 2246, git_sha1: "daf30b0336f061a1a7961e1dd0dea9d1ce7ccd40" },
    Asset { name: "arrow_no_tail_smaller_v2.cur", size: 2246, git_sha1: "5e7990e298d85a4d8eb205e0b64a2b7d06b3f2b1" },
    Asset { name: "arrow_tail_detached.cur", size: 2246, git_sha1: "aa8082e301909e7bbf0bee1fdeb47db961f6d34a" },
    Asset { name: "beam.cur", size: 2246, git_sha1: "87c55d6e1a8d819ac80e4edd1d8afe4e35846f5a" },
    Asset { name: "beam_v2.cur", size: 2246, git_sha1: "b092bcf3b77e8cd9413c52e71d9a0a64afabaa69" },
    Asset { name: "busy.ani", size: 5474, git_sha1: "fa36f833b346a3d6310458d0a6bc30ec1c8657ad" },
    Asset { name: "cross.cur", size: 2246, git_sha1: "247947ea4293cb80fe60fe7143ae5c8d9633b7ad" },
    Asset { name: "diagonal_1.cur", size: 2246, git_sha1: "fe4205fd226707e165e1c9bc6ebf040419df33d2" },
    Asset { name: "diagonal_2.cur", size: 2246, git_sha1: "2244e646b88022bd3119a16ae7d17ee88aca88a9" },
    Asset { name: "help.cur", size: 2246, git_sha1: "d807f02924caa159d3bebd138af570bd6dd6ae3e" },
    Asset { name: "help_no_tail.cur", size: 2246, git_sha1: "fc3985098751b3d6e38f32ae43a9e736cafbdad1" },
    Asset { name: "help_tail_detached.cur", size: 326, git_sha1: "b3a1e4dcba4e4b2f24ade81dda0eece44e224d4d" },
    Asset { name: "horizontal.cur", size: 2246, git_sha1: "a38b02b149045421513ca7954d07581a6ae61745" },
    Asset { name: "horizontal_v2.cur", size: 2246, git_sha1: "8c7888da14a7a13441c56db830877df91024fd7d" },
    Asset { name: "link.cur", size: 2246, git_sha1: "86dae854fc5479674cca5ea25986b87c0ac2f3e0" },
    Asset { name: "link_v2.cur", size: 2246, git_sha1: "01eab41a23bbdaaf9f84e449f846a4f233777b68" },
    Asset { name: "move.cur", size: 2246, git_sha1: "6fdb0c4e3812c1cb896e5c45d4f691de53d80e84" },
    Asset { name: "move_v2.cur", size: 326, git_sha1: "6df1bcd2bea99c47a025d9f4dd03db9a3a6b3a98" },
    Asset { name: "pen.cur", size: 2246, git_sha1: "e94d2d6a7182abad2cda2b75434ed37ea7d9ebbc" },
    Asset { name: "special.cur", size: 2246, git_sha1: "dbb6b1fac453fb52ae1cfc87d09ee907373b19d3" },
    Asset { name: "unavailable.cur", size: 8774, git_sha1: "70496b18014075e2598f61c87ebd2c790174eb1c" },
    Asset { name: "unavailable_v2.cur", size: 2246, git_sha1: "4baceb98c400b4b6836a35d4e77b3fa7e76ad68d" },
    Asset { name: "unavailable_v3.cur", size: 2246, git_sha1: "d3e0901c59ec8494d8fc0a12caca418bac8b65ad" },
    Asset { name: "vertical.cur", size: 2246, git_sha1: "f6b8c7d085550f01b23a911d617b817bbac87d7d" },
    Asset { name: "vertical_v2.cur", size: 2246, git_sha1: "57b4a71f8fb6ffc70a169c65d3a561887d193aac" },
    Asset { name: "working.ani", size: 5474, git_sha1: "fcced8db8402d254a3f2e1e635bdb50dd334016c" },
    Asset { name: "working_no_tail.ani", size: 5474, git_sha1: "0c368f9a121a14c7c581ccfb60c427bf5093f77f" },
    Asset { name: "working_tail_detached.ani", size: 5474, git_sha1: "dec0cc444841031357b0b1a1b7189bd61340018a" },
    Asset { name: "working_tail_detached_v2.ani", size: 5474, git_sha1: "9f3656efe73c669aead22627e1f0bc854bab8b1a" },
    Asset { name: "working_tail_detached_v3.ani", size: 2072, git_sha1: "c47132bee21ac2941c6db45a72590f1333876907" },
];

pub const ROLES: &[&str] = &[
    "Arrow",
    "Help",
    "AppStarting",
    "Wait",
    "Crosshair",
    "IBeam",
    "NWPen",
    "No",
    "SizeNS",
    "SizeWE",
    "SizeNWSE",
    "SizeNESW",
    "SizeAll",
    "UpArrow",
    "Hand",
    "Pin",
    "Person",
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Preset {
    Compact,
    NoTail,
    TailDetached,
    Regular,
}

impl Preset {
    pub const ALL: [Preset; 4] = [
        Preset::Compact,
        Preset::NoTail,
        Preset::TailDetached,
        Preset::Regular,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Preset::Regular => "Modern Inverted v3.6.2 - Regular",
            Preset::NoTail => "Modern Inverted v3.6.2 - No Tail",
            Preset::TailDetached => "Modern Inverted v3.6.2 - Tail Detached",
            Preset::Compact => "Modern Inverted v3.6.2 - Compact",
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Preset::Regular => "Regular",
            Preset::NoTail => "No Tail",
            Preset::TailDetached => "Tail Detached",
            Preset::Compact => "Compact",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Preset::Compact => "Sleek smaller tail-less arrow & detached spinner",
            Preset::NoTail => "Full-size crisp arrow without pointer tail",
            Preset::TailDetached => "Classic pointer with detached modern tail",
            Preset::Regular => "Original full-tailed dynamic inverted arrow",
        }
    }

    pub fn cli_name(self) -> &'static str {
        match self {
            Preset::Regular => "regular",
            Preset::NoTail => "no-tail",
            Preset::TailDetached => "tail-detached",
            Preset::Compact => "compact",
        }
    }

    pub fn from_cli(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "regular" | "1" => Some(Preset::Regular),
            "notail" | "no-tail" | "no_tail" | "no tail" | "2" => Some(Preset::NoTail),
            "detached" | "tail-detached" | "tail_detached" | "tail detached" | "3" => {
                Some(Preset::TailDetached)
            }
            "compact" | "4" => Some(Preset::Compact),
            _ => None,
        }
    }

    pub fn custom_files(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Preset::Regular => ("arrow.cur", "help.cur", "working.ani"),
            Preset::NoTail => ("arrow_no_tail.cur", "help_no_tail.cur", "working_no_tail.ani"),
            Preset::TailDetached => (
                "arrow_tail_detached.cur",
                "help_tail_detached.cur",
                "working_tail_detached_v3.ani",
            ),
            Preset::Compact => (
                "arrow_no_tail_smaller_v2.cur",
                "help_no_tail.cur",
                "working_tail_detached_v3.ani",
            ),
        }
    }
}

#[repr(C)]
struct SystemTimeWin {
    w_year: u16,
    w_month: u16,
    w_day_of_week: u16,
    w_day: u16,
    w_hour: u16,
    w_minute: u16,
    w_second: u16,
    w_milliseconds: u16,
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

#[repr(C)]
struct IconInfo {
    f_icon: i32,
    x_hotspot: u32,
    y_hotspot: u32,
    hbm_mask: *mut c_void,
    hbm_color: *mut c_void,
}

#[link(name = "user32")]
extern "system" {
    fn SystemParametersInfoW(
        ui_action: u32,
        ui_param: u32,
        pv_param: *mut c_void,
        f_win_ini: u32,
    ) -> i32;
    fn LoadImageW(
        h_inst: *mut c_void,
        name: *const u16,
        image_type: u32,
        cx: i32,
        cy: i32,
        load_flags: u32,
    ) -> *mut c_void;
    fn SetSystemCursor(cursor: *mut c_void, id: u32) -> i32;
    fn DestroyCursor(cursor: *mut c_void) -> i32;
    fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> *mut c_void;
    fn PostMessageW(h_wnd: *mut c_void, msg: u32, w_param: usize, l_param: isize) -> i32;
    fn CreateIconIndirect(piconinfo: *const IconInfo) -> *mut c_void;
    fn GetSystemMetrics(n_index: i32) -> i32;
}

#[link(name = "gdi32")]
extern "system" {
    fn CreateDIBSection(
        hdc: *mut c_void,
        pbmi: *const BitmapInfoHeader,
        usage: u32,
        ppv_bits: *mut *mut c_void,
        h_section: *mut c_void,
        offset: u32,
    ) -> *mut c_void;
    fn CreateBitmap(
        n_width: i32,
        n_height: i32,
        n_planes: u32,
        n_bit_count: u32,
        lp_bits: *const c_void,
    ) -> *mut c_void;
    fn DeleteObject(ho: *mut c_void) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetLastError() -> u32;
    fn GetCurrentProcess() -> *mut c_void;
    fn SetProcessWorkingSetSize(
        h_process: *mut c_void,
        dw_minimum_working_set_size: usize,
        dw_maximum_working_set_size: usize,
    ) -> i32;
    fn GetLocalTime(lp_system_time: *mut SystemTimeWin);
    fn LoadLibraryW(lp_lib_file_name: *const u16) -> *mut c_void;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const u8) -> *mut c_void;
}

#[link(name = "advapi32")]
extern "system" {
    fn RegOpenKeyExW(
        h_key: *mut c_void,
        sub_key: *const u16,
        options: u32,
        sam_desired: u32,
        result: *mut *mut c_void,
    ) -> i32;
    fn RegCreateKeyExW(
        h_key: *mut c_void,
        sub_key: *const u16,
        reserved: u32,
        class: *mut u16,
        options: u32,
        sam_desired: u32,
        security_attributes: *mut c_void,
        result: *mut *mut c_void,
        disposition: *mut u32,
    ) -> i32;
    fn RegGetValueW(
        h_key: *mut c_void,
        sub_key: *const u16,
        value: *const u16,
        flags: u32,
        pdw_type: *mut u32,
        pv_data: *mut c_void,
        pcb_data: *mut u32,
    ) -> i32;
    fn RegSetValueExW(
        h_key: *mut c_void,
        value_name: *const u16,
        reserved: u32,
        dw_type: u32,
        lp_data: *const u8,
        cb_data: u32,
    ) -> i32;
    fn RegDeleteValueW(h_key: *mut c_void, value_name: *const u16) -> i32;
    fn RegDeleteKeyW(h_key: *mut c_void, lp_sub_key: *const u16) -> i32;
    pub fn RegNotifyChangeKeyValue(
        h_key: *mut c_void,
        watch_subtree: i32,
        notify_filter: u32,
        event: *mut c_void,
        asynchronous: i32,
    ) -> i32;
    pub fn RegCloseKey(h_key: *mut c_void) -> i32;
}

pub fn wide_z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn hkcu() -> *mut c_void {
    (-2147483647isize) as usize as *mut c_void
}

pub fn trim_working_set() {
    unsafe {
        let _ = SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}

pub fn app_paths_for_root(root: PathBuf) -> (PathBuf, PathBuf, PathBuf) {
    let install_dir = root.join(format!("v{PACK_VERSION}"));
    let backup_path = root.join("backup-original.reg");
    (root, install_dir, backup_path)
}

pub fn app_paths() -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let local_app_data = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| "LOCALAPPDATA is not defined.".to_string())?;
    Ok(app_paths_for_root(
        local_app_data.join("ModernInvertedCursors"),
    ))
}

pub struct BoundedLogger {
    path: PathBuf,
    echo_stdout: bool,
}

impl BoundedLogger {
    pub fn new(path: PathBuf, echo_stdout: bool) -> Self {
        Self { path, echo_stdout }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn line(&self, text: impl AsRef<str>) {
        let text = text.as_ref();
        if self.echo_stdout {
            println!("{text}");
        }
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(meta) = fs::metadata(&self.path) {
            if meta.len() >= MAX_LOG_BYTES {
                let backup = self.path.with_extension("log.old");
                let _ = fs::remove_file(&backup);
                let _ = fs::rename(&self.path, &backup);
            }
        }
        let mut st = SystemTimeWin {
            w_year: 0,
            w_month: 0,
            w_day_of_week: 0,
            w_day: 0,
            w_hour: 0,
            w_minute: 0,
            w_second: 0,
            w_milliseconds: 0,
        };
        unsafe {
            GetLocalTime(&mut st);
        }
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&self.path) {
            let _ = writeln!(
                f,
                "[{:04}-{:02}-{:02} {:02}:{:02}:{:02}] {}",
                st.w_year, st.w_month, st.w_day, st.w_hour, st.w_minute, st.w_second, text
            );
        }
    }
}

pub fn reg_open_notify(subkey: &str) -> Result<*mut c_void, i32> {
    const KEY_NOTIFY: u32 = 0x0010;
    const KEY_QUERY_VALUE: u32 = 0x0001;
    let w_subkey = wide_z(subkey);
    let mut key: *mut c_void = std::ptr::null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            hkcu(),
            w_subkey.as_ptr(),
            0,
            std::ptr::null_mut(),
            0,
            KEY_NOTIFY | KEY_QUERY_VALUE,
            std::ptr::null_mut(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if status == 0 && !key.is_null() {
        Ok(key)
    } else {
        Err(status)
    }
}

pub fn reg_get_dword(subkey: &str, value_name: &str) -> Option<u32> {
    const RRF_RT_REG_DWORD: u32 = 0x0000_0010;
    let w_subkey = wide_z(subkey);
    let w_name = wide_z(value_name);
    let mut val: u32 = 0;
    let mut size: u32 = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            hkcu(),
            w_subkey.as_ptr(),
            w_name.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            &mut val as *mut u32 as *mut c_void,
            &mut size,
        )
    };
    if status == 0 {
        Some(val)
    } else {
        None
    }
}

pub fn reg_get_string(subkey: &str, value_name: Option<&str>) -> Option<String> {
    const RRF_RT_REG_SZ: u32 = 0x0000_0002;
    let w_subkey = wide_z(subkey);
    let w_name = value_name.map(wide_z);
    let name_ptr = w_name.as_ref().map_or(std::ptr::null(), |v| v.as_ptr());

    let mut bytes: u32 = 0;
    let probe = unsafe {
        RegGetValueW(
            hkcu(),
            w_subkey.as_ptr(),
            name_ptr,
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut bytes,
        )
    };
    if probe != 0 || bytes < 2 {
        return None;
    }

    let mut buf = vec![0u16; (bytes as usize + 1) / 2];
    let mut actual_bytes = (buf.len() * 2) as u32;
    let status = unsafe {
        RegGetValueW(
            hkcu(),
            w_subkey.as_ptr(),
            name_ptr,
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr() as *mut c_void,
            &mut actual_bytes,
        )
    };
    if status != 0 {
        return None;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..end]))
}

pub fn reg_set_string(
    subkey: &str,
    value_name: Option<&str>,
    expand_sz: bool,
    data: &str,
) -> Result<(), String> {
    const KEY_SET_VALUE: u32 = 0x0002;
    const REG_SZ: u32 = 1;
    const REG_EXPAND_SZ: u32 = 2;

    let w_subkey = wide_z(subkey);
    let mut key: *mut c_void = std::ptr::null_mut();
    let open = unsafe {
        RegCreateKeyExW(
            hkcu(),
            w_subkey.as_ptr(),
            0,
            std::ptr::null_mut(),
            0,
            KEY_SET_VALUE,
            std::ptr::null_mut(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if open != 0 || key.is_null() {
        return Err(format!("RegCreateKeyExW({subkey}) failed with status={open}"));
    }

    let w_name = value_name.map(wide_z);
    let name_ptr = w_name.as_ref().map_or(std::ptr::null(), |v| v.as_ptr());
    let w_data = wide_z(data);
    let reg_type = if expand_sz { REG_EXPAND_SZ } else { REG_SZ };
    let byte_len = (w_data.len() * std::mem::size_of::<u16>()) as u32;

    let set = unsafe {
        RegSetValueExW(
            key,
            name_ptr,
            0,
            reg_type,
            w_data.as_ptr() as *const u8,
            byte_len,
        )
    };
    unsafe {
        RegCloseKey(key);
    }

    if set == 0 {
        Ok(())
    } else {
        Err(format!(
            "RegSetValueExW({subkey}\\{}) failed with status={set}",
            value_name.unwrap_or("(Default)")
        ))
    }
}

pub fn reg_set_dword(subkey: &str, value_name: &str, val: u32) -> Result<(), String> {
    const KEY_SET_VALUE: u32 = 0x0002;
    const REG_DWORD: u32 = 4;

    let w_subkey = wide_z(subkey);
    let mut key: *mut c_void = std::ptr::null_mut();
    let open = unsafe {
        RegCreateKeyExW(
            hkcu(),
            w_subkey.as_ptr(),
            0,
            std::ptr::null_mut(),
            0,
            KEY_SET_VALUE,
            std::ptr::null_mut(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if open != 0 || key.is_null() {
        return Err(format!("RegCreateKeyExW({subkey}) failed with status={open}"));
    }

    let w_name = wide_z(value_name);
    let set = unsafe {
        RegSetValueExW(
            key,
            w_name.as_ptr(),
            0,
            REG_DWORD,
            &val as *const u32 as *const u8,
            std::mem::size_of::<u32>() as u32,
        )
    };
    unsafe {
        RegCloseKey(key);
    }

    if set == 0 {
        Ok(())
    } else {
        Err(format!(
            "RegSetValueExW({subkey}\\{value_name}) failed with status={set}"
        ))
    }
}

pub fn reg_delete_value(subkey: &str, value_name: &str) {
    const KEY_SET_VALUE: u32 = 0x0002;
    let w_subkey = wide_z(subkey);
    let mut key: *mut c_void = std::ptr::null_mut();
    let open = unsafe { RegOpenKeyExW(hkcu(), w_subkey.as_ptr(), 0, KEY_SET_VALUE, &mut key) };
    if open == 0 && !key.is_null() {
        let w_name = wide_z(value_name);
        unsafe {
            RegDeleteValueW(key, w_name.as_ptr());
            RegCloseKey(key);
        }
    }
}

pub fn reg_delete_subkey(parent_subkey: &str, child_name: &str) {
    const DELETE_ACCESS: u32 = 0x0001_0000;
    let w_parent = wide_z(parent_subkey);
    let mut key: *mut c_void = std::ptr::null_mut();
    let open = unsafe { RegOpenKeyExW(hkcu(), w_parent.as_ptr(), 0, DELETE_ACCESS, &mut key) };
    if open == 0 && !key.is_null() {
        let w_child = wide_z(child_name);
        unsafe {
            RegDeleteKeyW(key, w_child.as_ptr());
            RegCloseKey(key);
        }
    }
}

pub fn load_saved_preset() -> Preset {
    if let Some(raw) = reg_get_string(CONFIG_SUBKEY, Some("Preset")) {
        if let Some(p) = Preset::from_cli(&raw) {
            return p;
        }
    }
    if let Some(raw_scheme) = reg_get_string(CURSORS_SUBKEY, None) {
        let low = raw_scheme.to_ascii_lowercase();
        if low.contains("no tail") {
            return Preset::NoTail;
        }
        if low.contains("tail detached") {
            return Preset::TailDetached;
        }
        if low.contains("regular") {
            return Preset::Regular;
        }
    }
    Preset::Compact
}

pub fn save_preset(preset: Preset) -> Result<(), String> {
    reg_set_string(CONFIG_SUBKEY, Some("Preset"), false, preset.cli_name())
}

pub fn is_startup_enabled() -> bool {
    reg_get_string(RUN_SUBKEY, Some(GUARDIAN_RUN_VALUE)).is_some()
}

pub fn set_startup_enabled(guardian_exe: &Path, enabled: bool) -> Result<(), String> {
    if enabled {
        let cmd = format!("\"{}\" --tray", guardian_exe.display());
        reg_set_string(RUN_SUBKEY, Some(GUARDIAN_RUN_VALUE), false, &cmd)
    } else {
        reg_delete_value(RUN_SUBKEY, GUARDIAN_RUN_VALUE);
        Ok(())
    }
}

pub fn register_uninstall_entry(root: &Path, guardian_exe: &Path) -> Result<(), String> {
    let uninstall_cmd = format!("\"{}\" --uninstall", guardian_exe.display());
    let quiet_cmd = format!("\"{}\" --restore", guardian_exe.display());
    let icon_cmd = format!("{},0", guardian_exe.display());

    reg_set_string(UNINSTALL_SUBKEY, Some("DisplayName"), false, "Modern Inverted Mouse Cursors")?;
    reg_set_string(UNINSTALL_SUBKEY, Some("DisplayVersion"), false, APP_VERSION)?;
    reg_set_string(UNINSTALL_SUBKEY, Some("Publisher"), false, PUBLISHER)?;
    reg_set_string(
        UNINSTALL_SUBKEY,
        Some("InstallLocation"),
        false,
        &root.to_string_lossy(),
    )?;
    reg_set_string(UNINSTALL_SUBKEY, Some("UninstallString"), false, &uninstall_cmd)?;
    reg_set_string(
        UNINSTALL_SUBKEY,
        Some("QuietUninstallString"),
        false,
        &quiet_cmd,
    )?;
    reg_set_string(UNINSTALL_SUBKEY, Some("DisplayIcon"), false, &icon_cmd)?;
    reg_set_string(
        UNINSTALL_SUBKEY,
        Some("URLInfoAbout"),
        false,
        "https://github.com/sinethnimsara26/Modern-Inverted-Cursor-Windows11",
    )?;
    reg_set_dword(UNINSTALL_SUBKEY, "NoModify", 1)?;
    reg_set_dword(UNINSTALL_SUBKEY, "NoRepair", 1)?;
    reg_set_dword(UNINSTALL_SUBKEY, "EstimatedSize", 400)?;
    Ok(())
}

pub fn unregister_uninstall_entry() {
    reg_delete_subkey(UNINSTALL_PARENT_SUBKEY, UNINSTALL_KEY_NAME);
}

pub fn parse_preset_arg(args: &[String]) -> Result<Option<Preset>, String> {
    for i in 0..args.len() {
        if args[i] == "--preset" {
            let value = args.get(i + 1).ok_or_else(|| {
                "--preset requires regular|no-tail|tail-detached|compact".to_string()
            })?;
            return Preset::from_cli(value)
                .map(Some)
                .ok_or_else(|| format!("Unknown preset: {value}"));
        }
        if let Some(value) = args[i].strip_prefix("--preset=") {
            return Preset::from_cli(value)
                .map(Some)
                .ok_or_else(|| format!("Unknown preset: {value}"));
        }
    }
    Ok(None)
}

pub fn current_accessibility_step() -> u32 {
    if let Some(step) = reg_get_dword(ACCESSIBILITY_SUBKEY, "CursorSize") {
        return step.clamp(1, ACCESSIBILITY_STEPS);
    }
    if let Some(base) = reg_get_dword(CURSORS_SUBKEY, "CursorBaseSize") {
        let base = base.clamp(32, 256);
        let rounded_step = ((base.saturating_sub(32) + 8) / 16) + 1;
        return rounded_step.clamp(1, ACCESSIBILITY_STEPS);
    }
    // Windows 7 / 8 / 8.1 do not populate Windows 10/11 Accessibility\CursorSize.
    let metric = unsafe { GetSystemMetrics(13) }; // SM_CXCURSOR
    if metric > 32 {
        let px = (metric as u32).clamp(32, 256);
        let rounded_step = ((px.saturating_sub(32) + 8) / 16) + 1;
        return rounded_step.clamp(1, ACCESSIBILITY_STEPS);
    }
    1
}

pub fn balanced_cursor_size(step: u32) -> u32 {
    let step = step.clamp(1, ACCESSIBILITY_STEPS);
    BALANCED_MIN_SIZE + (step - 1) * BALANCED_STEP_PX
}

pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut msg = data.to_vec();
    let bit_len = (msg.len() as u64) * 8;
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            let j = i * 4;
            w[i] = u32::from_be_bytes([chunk[j], chunk[j + 1], chunk[j + 2], chunk[j + 3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let (mut a, mut b, mut c, mut d, mut e) = (h0, h1, h2, h3, h4);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                _ => (b ^ c ^ d, 0xCA62C1D6),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }
        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    for (i, word) in [h0, h1, h2, h3, h4].iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

pub fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

pub fn git_blob_sha1(bytes: &[u8]) -> String {
    let header = format!("blob {}\0", bytes.len());
    let mut data = Vec::with_capacity(header.len() + bytes.len());
    data.extend_from_slice(header.as_bytes());
    data.extend_from_slice(bytes);
    hex_lower(&sha1(&data))
}

pub fn verify_asset_file(asset: Asset, path: &Path) -> Result<(), String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("Could not read {} for verification: {e}", path.display()))?;
    if bytes.len() as u64 != asset.size {
        return Err(format!(
            "{} has wrong size: expected {}, got {} bytes.",
            asset.name,
            asset.size,
            bytes.len()
        ));
    }
    let got = git_blob_sha1(&bytes);
    if got != asset.git_sha1 {
        return Err(format!(
            "{} failed Git blob SHA-1 check (expected {}, got {got}).",
            asset.name, asset.git_sha1
        ));
    }
    Ok(())
}

pub fn assets_quick_ok(dir: &Path) -> bool {
    ASSETS.iter().all(|a| {
        let p = dir.join(a.name);
        fs::metadata(&p)
            .map(|m| m.is_file() && m.len() == a.size)
            .unwrap_or(false)
    })
}

pub fn parse_https_url(url: &str) -> Option<(&str, &str)> {
    let rest = url.strip_prefix("https://")?;
    let slash = rest.find('/')?;
    let host = &rest[..slash];
    let path = &rest[slash..];
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some((host, path))
}

fn winhttp_download_bytes(url: &str) -> Result<Vec<u8>, String> {
    let (host, path) = parse_https_url(url)
        .ok_or_else(|| format!("Unsupported URL for WinHTTP: {url}"))?;

    type WinHttpOpenFn =
        unsafe extern "system" fn(*const u16, u32, *const u16, *const u16, u32) -> *mut c_void;
    type WinHttpSetOptionFn =
        unsafe extern "system" fn(*mut c_void, u32, *const c_void, u32) -> i32;
    type WinHttpConnectFn =
        unsafe extern "system" fn(*mut c_void, *const u16, u16, u32) -> *mut c_void;
    type WinHttpOpenRequestFn = unsafe extern "system" fn(
        *mut c_void,
        *const u16,
        *const u16,
        *const u16,
        *const u16,
        *const *const u16,
        u32,
    ) -> *mut c_void;
    type WinHttpSendRequestFn = unsafe extern "system" fn(
        *mut c_void,
        *const u16,
        u32,
        *mut c_void,
        u32,
        u32,
        usize,
    ) -> i32;
    type WinHttpReceiveResponseFn = unsafe extern "system" fn(*mut c_void, *mut c_void) -> i32;
    type WinHttpQueryHeadersFn = unsafe extern "system" fn(
        *mut c_void,
        u32,
        *const u16,
        *mut c_void,
        *mut u32,
        *mut u32,
    ) -> i32;
    type WinHttpReadDataFn =
        unsafe extern "system" fn(*mut c_void, *mut u8, u32, *mut u32) -> i32;
    type WinHttpCloseFn = unsafe extern "system" fn(*mut c_void) -> i32;

    struct SafeWinHttpHandle {
        handle: *mut c_void,
        close_fn: WinHttpCloseFn,
    }
    impl Drop for SafeWinHttpHandle {
        fn drop(&mut self) {
            if !self.handle.is_null() {
                unsafe {
                    (self.close_fn)(self.handle);
                }
            }
        }
    }

    unsafe {
        let dll = LoadLibraryW(wide_z("winhttp.dll").as_ptr());
        if dll.is_null() {
            return Err("Could not load winhttp.dll".into());
        }
        macro_rules! load_proc {
            ($name:expr, $ty:ty) => {{
                let p = GetProcAddress(dll, $name.as_ptr());
                if p.is_null() {
                    return Err(format!(
                        "Missing WinHTTP export {}",
                        String::from_utf8_lossy($name).trim_end_matches('\0')
                    ));
                }
                std::mem::transmute::<*mut c_void, $ty>(p)
            }};
        }

        let open_fn = load_proc!(b"WinHttpOpen\0", WinHttpOpenFn);
        let set_opt_fn = load_proc!(b"WinHttpSetOption\0", WinHttpSetOptionFn);
        let connect_fn = load_proc!(b"WinHttpConnect\0", WinHttpConnectFn);
        let open_req_fn = load_proc!(b"WinHttpOpenRequest\0", WinHttpOpenRequestFn);
        let send_req_fn = load_proc!(b"WinHttpSendRequest\0", WinHttpSendRequestFn);
        let recv_resp_fn = load_proc!(b"WinHttpReceiveResponse\0", WinHttpReceiveResponseFn);
        let query_hdrs_fn = load_proc!(b"WinHttpQueryHeaders\0", WinHttpQueryHeadersFn);
        let read_data_fn = load_proc!(b"WinHttpReadData\0", WinHttpReadDataFn);
        let close_fn = load_proc!(b"WinHttpCloseHandle\0", WinHttpCloseFn);

        let agent = wide_z("ModernInvertedCursors/1.0");
        // Try Win8.1+ automatic proxy first, then Win7/8 default proxy.
        let mut h_session = open_fn(agent.as_ptr(), 4, std::ptr::null(), std::ptr::null(), 0);
        if h_session.is_null() {
            h_session = open_fn(agent.as_ptr(), 0, std::ptr::null(), std::ptr::null(), 0);
        }
        if h_session.is_null() {
            return Err(format!("WinHttpOpen failed (err={})", GetLastError()));
        }
        let session = SafeWinHttpHandle { handle: h_session, close_fn };

        // Enable TLS 1.2 + 1.3 (fallback to TLS 1.2 alone on Windows 7/8).
        let tls_12_13: u32 = 0x0000_0800 | 0x0000_2000;
        if set_opt_fn(
            session.handle,
            84, // WINHTTP_OPTION_SECURE_PROTOCOLS
            &tls_12_13 as *const u32 as *const c_void,
            std::mem::size_of::<u32>() as u32,
        ) == 0
        {
            let tls_12: u32 = 0x0000_0800;
            let _ = set_opt_fn(
                session.handle,
                84,
                &tls_12 as *const u32 as *const c_void,
                std::mem::size_of::<u32>() as u32,
            );
        }

        let host_w = wide_z(host);
        let h_connect = connect_fn(session.handle, host_w.as_ptr(), 443, 0);
        if h_connect.is_null() {
            return Err(format!("WinHttpConnect failed (err={})", GetLastError()));
        }
        let connect = SafeWinHttpHandle { handle: h_connect, close_fn };

        let verb_w = wide_z("GET");
        let path_w = wide_z(path);
        let h_req = open_req_fn(
            connect.handle,
            verb_w.as_ptr(),
            path_w.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            0x0080_0000, // WINHTTP_FLAG_SECURE
        );
        if h_req.is_null() {
            return Err(format!("WinHttpOpenRequest failed (err={})", GetLastError()));
        }
        let req = SafeWinHttpHandle { handle: h_req, close_fn };

        if send_req_fn(req.handle, std::ptr::null(), 0, std::ptr::null_mut(), 0, 0, 0) == 0 {
            return Err(format!("WinHttpSendRequest failed (err={})", GetLastError()));
        }
        if recv_resp_fn(req.handle, std::ptr::null_mut()) == 0 {
            return Err(format!("WinHttpReceiveResponse failed (err={})", GetLastError()));
        }

        let mut status: u32 = 0;
        let mut status_len: u32 = std::mem::size_of::<u32>() as u32;
        if query_hdrs_fn(
            req.handle,
            19 | 0x2000_0000, // WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER
            std::ptr::null(),
            &mut status as *mut u32 as *mut c_void,
            &mut status_len,
            std::ptr::null_mut(),
        ) != 0
            && status >= 400
        {
            return Err(format!("HTTP status {status}"));
        }

        let mut out = Vec::with_capacity(8192);
        let mut buf = [0u8; 8192];
        loop {
            let mut read: u32 = 0;
            if read_data_fn(req.handle, buf.as_mut_ptr(), buf.len() as u32, &mut read) == 0 {
                return Err(format!("WinHttpReadData failed (err={})", GetLastError()));
            }
            if read == 0 {
                break;
            }
            out.extend_from_slice(&buf[..read as usize]);
        }
        Ok(out)
    }
}

pub fn download_asset(asset: Asset, dir: &Path, log: Option<&BoundedLogger>) -> Result<(), String> {
    let final_path = dir.join(asset.name);
    if final_path.exists() {
        if verify_asset_file(asset, &final_path).is_ok() {
            return Ok(());
        }
        if let Some(l) = log {
            l.line(format!("[repair] Re-downloading {}", asset.name));
        }
        let _ = fs::remove_file(&final_path);
    }

    let temp_path = dir.join(format!("{}.part", asset.name));
    let _ = fs::remove_file(&temp_path);
    let url = format!("{REPO_RAW}/{}", asset.name);

    // Native WinHTTP works on Windows 7 SP1 through Windows 11 without external curl.exe.
    let mut winhttp_ok = false;
    let mut winhttp_err = String::new();
    for attempt in 0..3 {
        match winhttp_download_bytes(&url) {
            Ok(bytes) => {
                fs::write(&temp_path, &bytes)
                    .map_err(|e| format!("Could not write {}: {e}", temp_path.display()))?;
                winhttp_ok = true;
                break;
            }
            Err(e) => {
                winhttp_err = e;
                if attempt < 2 {
                    thread::sleep(Duration::from_millis(350));
                }
            }
        }
    }

    if !winhttp_ok {
        let status = Command::new("curl.exe")
            .creation_flags(CREATE_NO_WINDOW)
            .args([
                "--fail",
                "--location",
                "--silent",
                "--show-error",
                "--retry",
                "4",
                "--retry-delay",
                "1",
                "--connect-timeout",
                "15",
                "--max-time",
                "90",
                "--output",
            ])
            .arg(&temp_path)
            .arg(&url)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| {
                format!(
                    "Download failed for {} via WinHTTP ({winhttp_err}) and curl.exe fallback ({e})",
                    asset.name
                )
            })?;

        if !status.success() {
            let _ = fs::remove_file(&temp_path);
            return Err(format!(
                "Download failed for {} ({url}): WinHTTP: {winhttp_err}",
                asset.name
            ));
        }
    }

    if let Err(e) = verify_asset_file(asset, &temp_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(format!("Integrity check failed for {}: {e}", asset.name));
    }
    fs::rename(&temp_path, &final_path)
        .map_err(|e| format!("Could not save {}: {e}", final_path.display()))?;
    Ok(())
}

pub fn ensure_all_assets(dir: &Path, verify_all: bool, log: &BoundedLogger) -> Result<(), String> {
    fs::create_dir_all(dir)
        .map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    if !verify_all && assets_quick_ok(dir) {
        return Ok(());
    }
    for asset in ASSETS {
        download_asset(*asset, dir, Some(log))?;
    }
    Ok(())
}

pub fn prepare_assets_parallel<F>(
    dir: &Path,
    log: Option<&BoundedLogger>,
    mut on_progress: F,
) -> Result<(), String>
where
    F: FnMut(usize, usize, &'static str),
{
    fs::create_dir_all(dir)
        .map_err(|e| format!("Could not create {}: {e}", dir.display()))?;

    let mut missing = Vec::new();
    let total = ASSETS.len();
    for (idx, asset) in ASSETS.iter().enumerate() {
        let p = dir.join(asset.name);
        if p.exists() && verify_asset_file(*asset, &p).is_ok() {
            continue;
        }
        missing.push(*asset);
        let _ = idx;
    }

    let cached_done = total - missing.len();
    if missing.is_empty() {
        on_progress(total, total, "All 32 assets verified");
        return Ok(());
    }
    on_progress(cached_done, total, missing[0].name);

    let next_idx = AtomicUsize::new(0);
    let done_count = AtomicUsize::new(cached_done);
    let workers = missing.len().min(4);
    let mut first_err: Option<String> = None;

    let (tx, rx) = std::sync::mpsc::channel::<(usize, &'static str, Result<(), String>)>();

    thread::scope(|s| {
        for _ in 0..workers {
            let tx = tx.clone();
            let missing_ref = &missing;
            let next_ref = &next_idx;
            let done_ref = &done_count;
            s.spawn(move || loop {
                let i = next_ref.fetch_add(1, Ordering::Relaxed);
                if i >= missing_ref.len() {
                    break;
                }
                let asset = missing_ref[i];
                let res = download_asset(asset, dir, log);
                let finished = done_ref.fetch_add(1, Ordering::Relaxed) + 1;
                let _ = tx.send((finished, asset.name, res));
            });
        }
        drop(tx);
        while let Ok((finished, name, res)) = rx.recv() {
            if let Err(err) = res {
                if first_err.is_none() {
                    first_err = Some(err);
                }
            } else {
                on_progress(finished, total, name);
            }
        }
    });

    if let Some(err) = first_err {
        Err(err)
    } else {
        Ok(())
    }
}

pub fn preset_role_paths(preset: Preset, dir: &Path) -> Vec<String> {
    let (arrow, help, working) = preset.custom_files();
    let p = |name: &str| dir.join(name).to_string_lossy().into_owned();
    vec![
        p(arrow),                                    // Arrow
        p(help),                                     // Help
        p(working),                                  // AppStarting
        p("busy.ani"),                               // Wait
        p("cross.cur"),                              // Crosshair
        p("beam_v2.cur"),                            // IBeam
        p("pen.cur"),                                // NWPen
        p("unavailable_v3.cur"),                     // No
        p("vertical_v2.cur"),                        // SizeNS
        p("horizontal_v2.cur"),                      // SizeWE
        p("diagonal_1.cur"),                         // SizeNWSE
        p("diagonal_2.cur"),                         // SizeNESW
        p("move_v2.cur"),                            // SizeAll
        p("special.cur"),                            // UpArrow
        p("link_v2.cur"),                            // Hand
        r"%SystemRoot%\cursors\pin_i.cur".into(),    // Pin
        r"%SystemRoot%\cursors\person_i.cur".into(), // Person
    ]
}

pub fn reload_cursors() -> Result<(), String> {
    const SPI_SETCURSORS: u32 = 0x0057;
    const SPIF_UPDATEINIFILE: u32 = 0x0001;
    const SPIF_SENDCHANGE: u32 = 0x0002;
    let ok = unsafe {
        SystemParametersInfoW(
            SPI_SETCURSORS,
            0,
            std::ptr::null_mut(),
            SPIF_UPDATEINIFILE | SPIF_SENDCHANGE,
        )
    };
    if ok == 0 {
        let err = unsafe { GetLastError() };
        Err(format!("SPI_SETCURSORS failed (GetLastError={err})"))
    } else {
        Ok(())
    }
}

fn load_and_set_system_cursor(
    path: &Path,
    id: u32,
    label: &str,
    cursor_size: u32,
) -> Result<(), String> {
    const IMAGE_CURSOR: u32 = 2;
    const LR_LOADFROMFILE: u32 = 0x0000_0010;
    let wide = wide_z(&path.to_string_lossy());

    let handle = unsafe {
        LoadImageW(
            std::ptr::null_mut(),
            wide.as_ptr(),
            IMAGE_CURSOR,
            cursor_size as i32,
            cursor_size as i32,
            LR_LOADFROMFILE,
        )
    };
    if handle.is_null() {
        let err = unsafe { GetLastError() };
        return Err(format!(
            "LoadImageW failed for {label}: {} at {cursor_size}px (err={err})",
            path.display()
        ));
    }

    let ok = unsafe { SetSystemCursor(handle, id) };
    if ok == 0 {
        let err = unsafe { GetLastError() };
        unsafe {
            DestroyCursor(handle);
        }
        return Err(format!("SetSystemCursor failed for {label} / {id} (err={err})"));
    }
    Ok(())
}

pub fn register_and_apply(
    preset: Preset,
    dir: &Path,
    log: Option<&BoundedLogger>,
) -> Result<(u32, u32), String> {
    let values = preset_role_paths(preset, dir);
    if values.len() != ROLES.len() {
        return Err("Internal cursor role count mismatch.".into());
    }

    let scheme_string = values.join(",");
    reg_set_string(SCHEMES_SUBKEY, Some(preset.name()), true, &scheme_string)?;
    reg_set_string(CURSORS_SUBKEY, None, false, preset.name())?;
    for (role, value) in ROLES.iter().zip(values.iter()) {
        reg_set_string(CURSORS_SUBKEY, Some(role), true, value)?;
    }
    reg_set_dword(CURSORS_SUBKEY, "Scheme Source", 0)?;

    let _ = reload_cursors();

    let (arrow, help, working) = preset.custom_files();
    let system_root = env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"));
    let system_cursors = system_root.join("Cursors");

    let live: [(&str, PathBuf, u32); 17] = [
        ("Normal Select", dir.join(arrow), 32512),
        ("Text Select", dir.join("beam_v2.cur"), 32513),
        ("Busy", dir.join("busy.ani"), 32514),
        ("Precision Select", dir.join("cross.cur"), 32515),
        ("Alternate Select", dir.join("special.cur"), 32516),
        ("Pen", dir.join("pen.cur"), 32631),
        ("Diagonal Resize 1", dir.join("diagonal_1.cur"), 32642),
        ("Diagonal Resize 2", dir.join("diagonal_2.cur"), 32643),
        ("Horizontal Resize", dir.join("horizontal_v2.cur"), 32644),
        ("Vertical Resize", dir.join("vertical_v2.cur"), 32645),
        ("Move", dir.join("move_v2.cur"), 32646),
        ("Unavailable", dir.join("unavailable_v3.cur"), 32648),
        ("Link Select", dir.join("link_v2.cur"), 32649),
        ("Working In Background", dir.join(working), 32650),
        ("Help Select", dir.join(help), 32651),
        ("Location Select", system_cursors.join("pin_i.cur"), 32671),
        ("Person Select", system_cursors.join("person_i.cur"), 32672),
    ];

    let step = current_accessibility_step();
    let px = balanced_cursor_size(step);
    for (label, path, id) in live {
        // Optional extended cursor slots (Pen 32631, Location 32671, Person 32672)
        // may be absent on Windows 7 / 8 / 8.1 or early Windows 10 builds.
        let optional = id == 32631 || id == 32671 || id == 32672;
        if !path.exists() && optional {
            continue;
        }
        if let Err(err) = load_and_set_system_cursor(&path, id, label, px) {
            if !optional {
                return Err(err);
            }
        }
    }

    if let Some(l) = log {
        l.line(format!(
            "Applied {} at {}px (Accessibility {}/{})",
            preset.short_name(),
            px,
            step,
            ACCESSIBILITY_STEPS
        ));
    }
    trim_working_set();
    Ok((step, px))
}

pub fn export_backup(backup: &Path, log: Option<&BoundedLogger>) -> Result<(), String> {
    if backup.exists() {
        if let Some(l) = log {
            l.line(format!("[backup] Keeping existing {}", backup.display()));
        }
        return Ok(());
    }
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    let status = Command::new("reg.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["export", r"HKCU\Control Panel\Cursors"])
        .arg(backup)
        .arg("/y")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Could not start reg.exe for backup: {e}"))?;
    if !status.success() {
        return Err("Windows could not export the current cursor registry key.".into());
    }
    if let Some(l) = log {
        l.line(format!("[backup] Saved original cursors to {}", backup.display()));
    }
    Ok(())
}

pub fn restore_backup(backup: &Path, log: Option<&BoundedLogger>) -> Result<(), String> {
    if !backup.exists() {
        return Err(format!("No original backup exists at:\n{}", backup.display()));
    }
    let status = Command::new("reg.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["import"])
        .arg(backup)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Could not start reg.exe for restore: {e}"))?;
    if !status.success() {
        return Err("reg.exe could not import the original cursor backup.".into());
    }
    let _ = reload_cursors();
    if let Some(l) = log {
        l.line("[restore] Original Windows cursor configuration restored.");
    }
    Ok(())
}

pub fn find_tray_window() -> *mut c_void {
    let w_cls = wide_z(TRAY_WND_CLASS);
    unsafe { FindWindowW(w_cls.as_ptr(), std::ptr::null()) }
}

pub fn close_existing_v6_window() {
    const WM_CLOSE: u32 = 0x0010;
    let existing = find_tray_window();
    if !existing.is_null() {
        unsafe {
            PostMessageW(existing, WM_CLOSE, 0, 0);
        }
        for _ in 0..40 {
            thread::sleep(Duration::from_millis(50));
            if find_tray_window().is_null() {
                break;
            }
        }
    }
}

pub fn stop_any_running_guardian() {
    close_existing_v6_window();
    let _ = Command::new("taskkill.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["/IM", GUARDIAN_EXE_NAME, "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    thread::sleep(Duration::from_millis(180));
}

pub fn write_guardian_binary(guardian_exe: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut last_err = None;
    for _ in 0..25 {
        match fs::write(guardian_exe, bytes) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
    Err(format!(
        "Could not write {}: {}",
        guardian_exe.display(),
        last_err.map(|e| e.to_string()).unwrap_or_default()
    ))
}

pub fn schedule_self_delete(path: &Path) {
    let cmd = format!(
        "ping -n 2 127.0.0.1 >nul & del /f /q \"{}\"",
        path.display()
    );
    let _ = Command::new("cmd.exe")
        .creation_flags(CREATE_NO_WINDOW)
        .args(["/C", &cmd])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
}

// --- Shared 4x4 supersampled vector cursor renderer ---

fn point_in_poly(x: f32, y: f32, poly: &[(f32, f32)]) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        let intersect =
            ((yi > y) != (yj > y)) && (x < (xj - xi) * (y - yi) / ((yj - yi) + 1e-6) + xi);
        if intersect {
            inside = !inside;
        }
        j = i;
    }
    inside
}

fn dist_to_segment(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    let t = if len_sq > 0.0 {
        (((px - ax) * dx + (py - ay) * dy) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let cx = ax + t * dx;
    let cy = ay + t * dy;
    ((px - cx) * (px - cx) + (py - cy) * (py - cy)).sqrt()
}

fn dist_to_poly(px: f32, py: f32, poly: &[(f32, f32)]) -> f32 {
    let mut best = f32::MAX;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (ax, ay) = poly[j];
        let (bx, by) = poly[i];
        best = best.min(dist_to_segment(px, py, ax, ay, bx, by));
        j = i;
    }
    best
}

fn preset_polys(preset: Preset) -> (&'static [(f32, f32)], Option<&'static [(f32, f32)]>, f32) {
    const REGULAR: &[(f32, f32)] = &[
        (0.22, 0.09),
        (0.22, 0.77),
        (0.36, 0.62),
        (0.48, 0.87),
        (0.58, 0.82),
        (0.46, 0.57),
        (0.69, 0.57),
    ];
    const NO_TAIL: &[(f32, f32)] = &[
        (0.22, 0.12),
        (0.22, 0.78),
        (0.43, 0.61),
        (0.71, 0.61),
    ];
    const DETACHED_HEAD: &[(f32, f32)] = &[
        (0.22, 0.09),
        (0.22, 0.71),
        (0.42, 0.56),
        (0.70, 0.56),
    ];
    const DETACHED_TAIL: &[(f32, f32)] = &[
        (0.43, 0.68),
        (0.52, 0.88),
        (0.62, 0.83),
        (0.53, 0.64),
    ];
    const COMPACT: &[(f32, f32)] = &[
        (0.28, 0.18),
        (0.28, 0.71),
        (0.45, 0.58),
        (0.67, 0.58),
    ];

    match preset {
        Preset::Regular => (REGULAR, None, 0.22),
        Preset::NoTail => (NO_TAIL, None, 0.22),
        Preset::TailDetached => (DETACHED_HEAD, Some(DETACHED_TAIL), 0.22),
        Preset::Compact => (COMPACT, None, 0.28),
    }
}

/// Composites a 4x4 supersampled split-contrast cursor arrow directly into a
/// top-down `0xAARRGGBB` pixel slice (`premultiplied == true` for Win32 `HICON`s,
/// `false` when compositing cleanly over an opaque dark UI surface).
pub fn draw_cursor_glyph(
    pixels: &mut [u32],
    stride: usize,
    x0: i32,
    y0: i32,
    size: i32,
    preset: Preset,
    paused: bool,
    premultiplied: bool,
) {
    if size <= 0 {
        return;
    }
    let (main_poly, extra_poly, origin_x) = preset_polys(preset);
    let outline_px = (size as f32 * 0.056).max(1.25);

    for dy in 0..size {
        let py_i = y0 + dy;
        if py_i < 0 {
            continue;
        }
        let row_start = py_i as usize * stride;
        if row_start >= pixels.len() {
            break;
        }
        for dx in 0..size {
            let px_i = x0 + dx;
            if px_i < 0 || px_i as usize >= stride {
                continue;
            }
            let idx = row_start + px_i as usize;
            if idx >= pixels.len() {
                continue;
            }

            let mut sum_r = 0.0f32;
            let mut sum_g = 0.0f32;
            let mut sum_b = 0.0f32;
            let mut sum_a = 0.0f32;

            for sy in 0..4 {
                for sx in 0..4 {
                    let fx = dx as f32 + (sx as f32 + 0.5) * 0.25;
                    let fy = dy as f32 + (sy as f32 + 0.5) * 0.25;
                    let nx = fx / size as f32;
                    let ny = fy / size as f32;

                    let mut inside = point_in_poly(nx, ny, main_poly);
                    let mut dist = dist_to_poly(nx, ny, main_poly) * size as f32;
                    if let Some(ep) = extra_poly {
                        if point_in_poly(nx, ny, ep) {
                            inside = true;
                        }
                        dist = dist.min(dist_to_poly(nx, ny, ep) * size as f32);
                    }

                    let split_x = origin_x + (ny - 0.10) * 0.42;
                    let right_half = nx >= split_x;

                    let (mut r, mut g, mut b, mut a) = if inside {
                        if right_half {
                            (18.0f32, 20.0f32, 26.0f32, 255.0f32)
                        } else {
                            (255.0f32, 255.0f32, 255.0f32, 255.0f32)
                        }
                    } else if dist <= outline_px {
                        if right_half {
                            (250.0, 250.0, 252.0, 255.0)
                        } else {
                            (16.0, 18.0, 22.0, 255.0)
                        }
                    } else {
                        (0.0, 0.0, 0.0, 0.0)
                    };

                    if paused && a > 0.0 {
                        a *= 0.55;
                    }
                    if paused {
                        let cx = size as f32 * 0.765;
                        let cy = size as f32 * 0.765;
                        let rad = size as f32 * 0.195;
                        let bd = ((fx - cx) * (fx - cx) + (fy - cy) * (fy - cy)).sqrt();
                        if bd <= rad {
                            let bar_h = rad * 0.48;
                            let bar_w = rad * 0.24;
                            let gap = rad * 0.18;
                            let in_bars = (fy - cy).abs() <= bar_h
                                && (((fx - (cx - gap)).abs() <= bar_w)
                                    || ((fx - (cx + gap)).abs() <= bar_w));
                            if in_bars {
                                (r, g, b, a) = (255.0, 255.0, 255.0, 255.0);
                            } else {
                                (r, g, b, a) = (240.0, 158.0, 24.0, 255.0);
                            }
                        }
                    }

                    let af = a / 255.0;
                    sum_r += r * af;
                    sum_g += g * af;
                    sum_b += b * af;
                    sum_a += a;
                }
            }

            let pr = sum_r / 16.0;
            let pg = sum_g / 16.0;
            let pb = sum_b / 16.0;
            let pa = sum_a / 16.0;

            if premultiplied {
                let r8 = pr.round().clamp(0.0, 255.0) as u32;
                let g8 = pg.round().clamp(0.0, 255.0) as u32;
                let b8 = pb.round().clamp(0.0, 255.0) as u32;
                let a8 = pa.round().clamp(0.0, 255.0) as u32;
                pixels[idx] = (a8 << 24) | (r8 << 16) | (g8 << 8) | b8;
            } else if pa > 0.5 {
                let dst = pixels[idx];
                let dr = ((dst >> 16) & 0xFF) as f32;
                let dg = ((dst >> 8) & 0xFF) as f32;
                let db = (dst & 0xFF) as f32;
                let inv = 1.0 - (pa / 255.0);
                let out_r = (pr + dr * inv).round().clamp(0.0, 255.0) as u32;
                let out_g = (pg + dg * inv).round().clamp(0.0, 255.0) as u32;
                let out_b = (pb + db * inv).round().clamp(0.0, 255.0) as u32;
                pixels[idx] = 0xFF00_0000 | (out_r << 16) | (out_g << 8) | out_b;
            }
        }
    }
}

pub fn create_split_cursor_hicon(size: usize, preset: Preset, paused: bool) -> *mut c_void {
    let bmi = BitmapInfoHeader {
        bi_size: std::mem::size_of::<BitmapInfoHeader>() as u32,
        bi_width: size as i32,
        bi_height: -(size as i32),
        bi_planes: 1,
        bi_bit_count: 32,
        bi_compression: 0,
        bi_size_image: 0,
        bi_x_pels_per_meter: 0,
        bi_y_pels_per_meter: 0,
        bi_clr_used: 0,
        bi_clr_important: 0,
    };

    let mut bits_ptr: *mut c_void = std::ptr::null_mut();
    let hbm_color = unsafe {
        CreateDIBSection(
            std::ptr::null_mut(),
            &bmi,
            0,
            &mut bits_ptr,
            std::ptr::null_mut(),
            0,
        )
    };
    if hbm_color.is_null() || bits_ptr.is_null() {
        return std::ptr::null_mut();
    }

    let pixels = unsafe { std::slice::from_raw_parts_mut(bits_ptr as *mut u32, size * size) };
    pixels.fill(0);
    draw_cursor_glyph(pixels, size, 0, 0, size as i32, preset, paused, true);

    let mask_bytes = vec![0u8; size * ((size + 7) / 8)];
    let hbm_mask = unsafe {
        CreateBitmap(
            size as i32,
            size as i32,
            1,
            1,
            mask_bytes.as_ptr() as *const c_void,
        )
    };
    let info = IconInfo {
        f_icon: 1,
        x_hotspot: 0,
        y_hotspot: 0,
        hbm_mask,
        hbm_color,
    };
    let hicon = unsafe { CreateIconIndirect(&info) };
    unsafe {
        if !hbm_mask.is_null() {
            DeleteObject(hbm_mask);
        }
        DeleteObject(hbm_color);
    }
    hicon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_blob_sha1_matches_git_spec() {
        assert_eq!(
            git_blob_sha1(b"hello\n"),
            "ce013625030ba8dba906f756967f9e9ca394464a"
        );
    }

    #[test]
    fn test_balanced_cursor_size_progression() {
        assert_eq!(balanced_cursor_size(0), 32);
        assert_eq!(balanced_cursor_size(1), 32);
        assert_eq!(balanced_cursor_size(2), 36);
        assert_eq!(balanced_cursor_size(15), 88);
        assert_eq!(balanced_cursor_size(99), 88);
    }

    #[test]
    fn test_preset_cli_and_roles_match() {
        for preset in Preset::ALL {
            assert_eq!(Preset::from_cli(preset.cli_name()), Some(preset));
            assert_eq!(
                preset_role_paths(preset, Path::new(r"C:\test")).len(),
                ROLES.len()
            );
        }
        assert_eq!(Preset::from_cli("unknown"), None);
    }

    #[test]
    fn test_split_cursor_glyph_renders_distinct_presets() {
        let mut buf_a = vec![0xFF00_0000u32; 32 * 32];
        let mut buf_b = vec![0xFF00_0000u32; 32 * 32];
        draw_cursor_glyph(&mut buf_a, 32, 0, 0, 32, Preset::Compact, false, false);
        draw_cursor_glyph(&mut buf_b, 32, 0, 0, 32, Preset::Regular, false, false);
        assert_ne!(buf_a, buf_b);
        assert!(buf_a.iter().any(|&p| p == 0xFFFF_FFFF));
    }

    #[test]
    fn test_create_split_cursor_hicon_succeeds() {
        let icon = create_split_cursor_hicon(32, Preset::Regular, false);
        assert!(!icon.is_null());
        unsafe {
            DestroyCursor(icon);
        }
    }

    #[test]
    fn test_parse_https_url_extracts_host_and_path() {
        assert_eq!(
            parse_https_url("https://raw.githubusercontent.com/emvaized/modern_inverted_mouse_cursors/3.6.2/src/arrow.cur"),
            Some((
                "raw.githubusercontent.com",
                "/emvaized/modern_inverted_mouse_cursors/3.6.2/src/arrow.cur"
            ))
        );
        assert_eq!(parse_https_url("http://example.com/a"), None);
        assert_eq!(parse_https_url("https://example.com"), None);
    }
}
