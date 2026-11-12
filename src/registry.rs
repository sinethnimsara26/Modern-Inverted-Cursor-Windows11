use std::{
    env,
    ffi::c_void,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub const PACK_VERSION: &str = "3.6.2";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PUBLISHER: &str = "SN-Tech / sinethnimsara26";

pub const SETUP_EXE_NAME: &str = "ModernInvertedCursors-Setup.exe";
pub const GUARDIAN_EXE_NAME: &str = "ModernInvertedCursors-Guardian.exe";
pub const LEGACY_GUARDIAN_EXE_NAME: &str = "ModernInvertedCursorGuardian.exe";
pub const GUARDIAN_RUN_VALUE: &str = "ModernInvertedCursorsGuardian";
pub const LEGACY_GUARDIAN_RUN_VALUE: &str = "ModernInvertedCursorGuardian";
pub const TRAY_WND_CLASS: &str = "ModernInvertedCursorGuardianTrayWnd";

pub const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const CONFIG_SUBKEY: &str = r"Software\ModernInvertedCursors";
pub const UNINSTALL_PARENT_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall";
pub const UNINSTALL_SUBKEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall\ModernInvertedCursors";
pub const UNINSTALL_KEY_NAME: &str = "ModernInvertedCursors";
pub const CURSORS_SUBKEY: &str = r"Control Panel\Cursors";
pub const SCHEMES_SUBKEY: &str = r"Control Panel\Cursors\Schemes";
pub const ACCESSIBILITY_SUBKEY: &str = r"Software\Microsoft\Accessibility";

pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const MAX_LOG_BYTES: u64 = 32 * 1024;

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
            Preset::NoTail => (
                "arrow_no_tail.cur",
                "help_no_tail.cur",
                "working_no_tail.ani",
            ),
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

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> *mut c_void;
    fn SetProcessWorkingSetSize(
        h_process: *mut c_void,
        dw_minimum_working_set_size: usize,
        dw_maximum_working_set_size: usize,
    ) -> i32;
    fn GetLocalTime(lp_system_time: *mut SystemTimeWin);
    fn GetModuleHandleW(lp_module_name: *const u16) -> *mut c_void;
    fn LoadLibraryW(lp_lib_file_name: *const u16) -> *mut c_void;
    fn LoadLibraryExW(
        lp_lib_file_name: *const u16,
        h_file: *mut c_void,
        dw_flags: u32,
    ) -> *mut c_void;
    fn GetSystemDirectoryW(lp_buffer: *mut u16, u_size: u32) -> u32;
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

/// Loads a Windows system DLL safely from `%SystemRoot%\System32` only (preventing
/// CWD/application-directory DLL search-order hijacking), preferring already-loaded handles.
pub fn load_system_dll(name: &str) -> *mut c_void {
    const LOAD_LIBRARY_SEARCH_SYSTEM32: u32 = 0x0000_0800;
    let w_name = wide_z(name);
    // SAFETY: NUL-terminated UTF-16 DLL name passed to Win32 loader APIs.
    unsafe {
        let loaded = GetModuleHandleW(w_name.as_ptr());
        if !loaded.is_null() {
            return loaded;
        }
        let pinned = LoadLibraryExW(
            w_name.as_ptr(),
            std::ptr::null_mut(),
            LOAD_LIBRARY_SEARCH_SYSTEM32,
        );
        if !pinned.is_null() {
            return pinned;
        }
        // Fallback for unpatched Windows 7 without KB2533623: build explicit System32 path.
        let mut sys_dir = vec![0u16; 260];
        let len = GetSystemDirectoryW(sys_dir.as_mut_ptr(), sys_dir.len() as u32) as usize;
        if len > 0 && len < sys_dir.len() {
            let base = String::from_utf16_lossy(&sys_dir[..len]);
            let full = wide_z(&format!("{}\\{}", base.trim_end_matches('\\'), name));
            return LoadLibraryW(full.as_ptr());
        }
        std::ptr::null_mut()
    }
}

pub fn trim_working_set() {
    // SAFETY: `GetCurrentProcess()` returns a pseudo-handle needing no close; `(usize::MAX, usize::MAX)` trims excess pages safely.
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
        // SAFETY: `st` is initialized `#[repr(C)]` `SYSTEMTIME` stack memory.
        unsafe {
            GetLocalTime(&mut st);
        }
        if let Ok(mut f) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
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
    // SAFETY: `w_subkey` is NUL-terminated; caller closes returned `HKEY` with `RegCloseKey`.
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
    // SAFETY: Stack pointers and buffer byte count match `DWORD`.
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
    // SAFETY: Probes required byte length with null data pointer.
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

    let mut buf = vec![0u16; (bytes as usize).div_ceil(2)];
    let mut actual_bytes = (buf.len() * 2) as u32;
    // SAFETY: `buf` is allocated to at least `actual_bytes` bytes.
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
    // SAFETY: `w_subkey` is NUL-terminated; `key` is closed right after `RegSetValueExW`.
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
        return Err(format!(
            "RegCreateKeyExW({subkey}) failed with status={open}"
        ));
    }

    let w_name = value_name.map(wide_z);
    let name_ptr = w_name.as_ref().map_or(std::ptr::null(), |v| v.as_ptr());
    let w_data = wide_z(data);
    let reg_type = if expand_sz { REG_EXPAND_SZ } else { REG_SZ };
    let byte_len = (w_data.len() * std::mem::size_of::<u16>()) as u32;

    // SAFETY: `w_data` is NUL-terminated UTF-16 and `byte_len` matches its byte length.
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
    // SAFETY: Valid key handle opened above.
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

/// Opens `subkey` once with read/write access and updates only the values whose current data
/// or registry type differs from the desired cursor scheme state. Returns `Ok(true)` if any value
/// was written, or `Ok(false)` when all values already matched verbatim.
pub fn reg_sync_cursor_values(
    subkey: &str,
    default_name: &str,
    pairs: &[(&str, &str)],
    scheme_source: u32,
) -> Result<bool, String> {
    const KEY_QUERY_VALUE: u32 = 0x0001;
    const KEY_SET_VALUE: u32 = 0x0002;
    const REG_SZ: u32 = 1;
    const REG_EXPAND_SZ: u32 = 2;
    const REG_DWORD: u32 = 4;
    const RRF_RT_REG_SZ: u32 = 0x0000_0002;
    const RRF_RT_REG_EXPAND_SZ: u32 = 0x0000_0004;
    const RRF_RT_REG_DWORD: u32 = 0x0000_0010;
    const RRF_NOEXPAND: u32 = 0x1000_0000;

    let w_subkey = wide_z(subkey);
    let mut key: *mut c_void = std::ptr::null_mut();
    // SAFETY: `w_subkey` is NUL-terminated; `key` is always closed before return.
    let open = unsafe {
        RegCreateKeyExW(
            hkcu(),
            w_subkey.as_ptr(),
            0,
            std::ptr::null_mut(),
            0,
            KEY_QUERY_VALUE | KEY_SET_VALUE,
            std::ptr::null_mut(),
            &mut key,
            std::ptr::null_mut(),
        )
    };
    if open != 0 || key.is_null() {
        return Err(format!(
            "RegCreateKeyExW({subkey}) failed with status={open}"
        ));
    }

    let mut changed = false;
    let mut buf = vec![0u16; 260];

    let sync_str = |name_opt: Option<&str>,
                    expand_sz: bool,
                    desired: &str,
                    buf: &mut Vec<u16>,
                    changed: &mut bool|
     -> Result<(), String> {
        let w_name = name_opt.map(wide_z);
        let name_ptr = w_name.as_ref().map_or(std::ptr::null(), |v| v.as_ptr());
        let expected_type = if expand_sz { REG_EXPAND_SZ } else { REG_SZ };

        let mut reg_type: u32 = 0;
        let mut bytes: u32 = (buf.len() * std::mem::size_of::<u16>()) as u32;
        // SAFETY: Query single open `key` without re-opening `subkey`.
        let mut status = unsafe {
            RegGetValueW(
                key,
                std::ptr::null(),
                name_ptr,
                RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ | RRF_NOEXPAND,
                &mut reg_type,
                buf.as_mut_ptr() as *mut c_void,
                &mut bytes,
            )
        };
        if status == 234 {
            // ERROR_MORE_DATA
            buf.resize((bytes as usize).div_ceil(2), 0);
            bytes = (buf.len() * std::mem::size_of::<u16>()) as u32;
            status = unsafe {
                RegGetValueW(
                    key,
                    std::ptr::null(),
                    name_ptr,
                    RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ | RRF_NOEXPAND,
                    &mut reg_type,
                    buf.as_mut_ptr() as *mut c_void,
                    &mut bytes,
                )
            };
        }
        let matches = if status == 0 && reg_type == expected_type && bytes >= 2 {
            let max_u16 = (bytes as usize / 2).min(buf.len());
            let end = buf[..max_u16]
                .iter()
                .position(|&c| c == 0)
                .unwrap_or(max_u16);
            String::from_utf16_lossy(&buf[..end]) == desired
        } else {
            false
        };
        if matches {
            return Ok(());
        }

        let w_data = wide_z(desired);
        let byte_len = (w_data.len() * std::mem::size_of::<u16>()) as u32;
        // SAFETY: `w_data` is NUL-terminated UTF-16 and `byte_len` matches its byte length.
        let set = unsafe {
            RegSetValueExW(
                key,
                name_ptr,
                0,
                expected_type,
                w_data.as_ptr() as *const u8,
                byte_len,
            )
        };
        if set != 0 {
            return Err(format!(
                "RegSetValueExW({subkey}\\{}) failed with status={set}",
                name_opt.unwrap_or("(Default)")
            ));
        }
        *changed = true;
        Ok(())
    };

    let res = (|| -> Result<(), String> {
        sync_str(None, false, default_name, &mut buf, &mut changed)?;
        for &(role, value) in pairs {
            sync_str(Some(role), true, value, &mut buf, &mut changed)?;
        }

        let w_src = wide_z("Scheme Source");
        let mut current_src: u32 = u32::MAX;
        let mut src_bytes: u32 = std::mem::size_of::<u32>() as u32;
        // SAFETY: Query `"Scheme Source"` DWORD from the open handle.
        let q_src = unsafe {
            RegGetValueW(
                key,
                std::ptr::null(),
                w_src.as_ptr(),
                RRF_RT_REG_DWORD,
                std::ptr::null_mut(),
                &mut current_src as *mut u32 as *mut c_void,
                &mut src_bytes,
            )
        };
        if q_src != 0 || current_src != scheme_source {
            // SAFETY: `&scheme_source` points to 4 valid bytes of `REG_DWORD`.
            let set = unsafe {
                RegSetValueExW(
                    key,
                    w_src.as_ptr(),
                    0,
                    REG_DWORD,
                    &scheme_source as *const u32 as *const u8,
                    std::mem::size_of::<u32>() as u32,
                )
            };
            if set != 0 {
                return Err(format!(
                    "RegSetValueExW({subkey}\\Scheme Source) failed with status={set}"
                ));
            }
            changed = true;
        }
        Ok(())
    })();

    // SAFETY: Close `key` handle unconditionally before propagating `res`.
    unsafe {
        RegCloseKey(key);
    }
    res?;
    Ok(changed)
}

pub fn reg_set_dword(subkey: &str, value_name: &str, val: u32) -> Result<(), String> {
    const KEY_SET_VALUE: u32 = 0x0002;
    const REG_DWORD: u32 = 4;

    let w_subkey = wide_z(subkey);
    let mut key: *mut c_void = std::ptr::null_mut();
    // SAFETY: `key` is opened under `HKCU` and closed cleanly below.
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
        return Err(format!(
            "RegCreateKeyExW({subkey}) failed with status={open}"
        ));
    }

    let w_name = wide_z(value_name);
    // SAFETY: Points to 4-byte `u32`.
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
    // SAFETY: Closes `key`.
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
    // SAFETY: Opens key if present and deletes value, then closes key.
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
    // SAFETY: Opens parent key if present and deletes child subkey, then closes parent key.
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
        || reg_get_string(RUN_SUBKEY, Some(LEGACY_GUARDIAN_RUN_VALUE)).is_some()
}

pub fn set_startup_enabled(guardian_exe: &Path, enabled: bool) -> Result<(), String> {
    if enabled {
        let cmd = format!("\"{}\" --tray", guardian_exe.display());
        reg_set_string(RUN_SUBKEY, Some(GUARDIAN_RUN_VALUE), false, &cmd)?;
        reg_delete_value(RUN_SUBKEY, LEGACY_GUARDIAN_RUN_VALUE);
        Ok(())
    } else {
        reg_delete_value(RUN_SUBKEY, GUARDIAN_RUN_VALUE);
        reg_delete_value(RUN_SUBKEY, LEGACY_GUARDIAN_RUN_VALUE);
        Ok(())
    }
}

pub fn register_uninstall_entry(
    root: &Path,
    setup_exe: &Path,
    estimated_kb: u32,
) -> Result<(), String> {
    let uninstall_cmd = format!("\"{}\" --uninstall", setup_exe.display());
    let quiet_cmd = format!("\"{}\" --uninstall --silent", setup_exe.display());
    let modify_cmd = format!("\"{}\" --modify", setup_exe.display());
    let icon_cmd = format!("{},0", setup_exe.display());

    reg_set_string(
        UNINSTALL_SUBKEY,
        Some("DisplayName"),
        false,
        "Modern Inverted Mouse Cursors",
    )?;
    reg_set_string(UNINSTALL_SUBKEY, Some("DisplayVersion"), false, APP_VERSION)?;
    reg_set_string(UNINSTALL_SUBKEY, Some("Publisher"), false, PUBLISHER)?;
    reg_set_string(
        UNINSTALL_SUBKEY,
        Some("InstallLocation"),
        false,
        &root.to_string_lossy(),
    )?;
    reg_set_string(
        UNINSTALL_SUBKEY,
        Some("UninstallString"),
        false,
        &uninstall_cmd,
    )?;
    reg_set_string(
        UNINSTALL_SUBKEY,
        Some("QuietUninstallString"),
        false,
        &quiet_cmd,
    )?;
    reg_set_string(UNINSTALL_SUBKEY, Some("ModifyPath"), false, &modify_cmd)?;
    reg_set_string(UNINSTALL_SUBKEY, Some("DisplayIcon"), false, &icon_cmd)?;
    reg_set_string(
        UNINSTALL_SUBKEY,
        Some("URLInfoAbout"),
        false,
        "https://github.com/sinethnimsara26/Modern-Inverted-Cursor-Windows11",
    )?;
    reg_set_dword(UNINSTALL_SUBKEY, "NoModify", 0)?;
    reg_set_dword(UNINSTALL_SUBKEY, "NoRepair", 0)?;
    reg_set_dword(UNINSTALL_SUBKEY, "EstimatedSize", estimated_kb.max(100))?;
    Ok(())
}

pub fn unregister_uninstall_entry() {
    reg_delete_subkey(UNINSTALL_PARENT_SUBKEY, UNINSTALL_KEY_NAME);
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct InstallDetection {
    pub has_backup: bool,
    pub has_legacy_guardian: bool,
    pub has_new_guardian: bool,
    pub has_installed_setup: bool,
}

impl InstallDetection {
    pub fn has_any(&self) -> bool {
        self.has_backup
            || self.has_legacy_guardian
            || self.has_new_guardian
            || self.has_installed_setup
    }
}

pub fn detect_existing_installation(root: &Path, backup_path: &Path) -> InstallDetection {
    InstallDetection {
        has_backup: backup_path.is_file(),
        has_legacy_guardian: root.join(LEGACY_GUARDIAN_EXE_NAME).is_file(),
        has_new_guardian: root.join(GUARDIAN_EXE_NAME).is_file(),
        has_installed_setup: root.join(SETUP_EXE_NAME).is_file(),
    }
}

/// Removes superseded v1.0.0–v1.0.2 guardian binaries cleanly once the upgraded guardian
/// has verified startup, never touching `backup-original.reg` or `v3.6.2` assets.
pub fn cleanup_legacy_guardian_binary(root: &Path, log: Option<&BoundedLogger>) {
    let legacy_exe = root.join(LEGACY_GUARDIAN_EXE_NAME);
    if legacy_exe.is_file() && fs::remove_file(&legacy_exe).is_ok() {
        if let Some(l) = log {
            l.line(format!(
                "[migrate] Removed superseded legacy guardian {}",
                legacy_exe.display()
            ));
        }
    }
}

pub fn parse_preset_arg(args: &[String]) -> Result<Option<Preset>, String> {
    let mut found: Option<Preset> = None;
    for i in 0..args.len() {
        if args[i] == "--preset" {
            let value = args.get(i + 1).ok_or_else(|| {
                "--preset requires regular|no-tail|tail-detached|compact".to_string()
            })?;
            found =
                Some(Preset::from_cli(value).ok_or_else(|| format!("Unknown preset: {value}"))?);
        } else if let Some(value) = args[i].strip_prefix("--preset=") {
            found =
                Some(Preset::from_cli(value).ok_or_else(|| format!("Unknown preset: {value}"))?);
        } else {
            let short = match args[i].as_str() {
                "--compact" => Some(Preset::Compact),
                "--no-tail" => Some(Preset::NoTail),
                "--tail-detached" | "--detached" => Some(Preset::TailDetached),
                "--regular" => Some(Preset::Regular),
                _ => None,
            };
            if let Some(p) = short {
                found = Some(p);
            }
        }
    }
    Ok(found)
}

/// Validates CLI flags against allowed boolean, required-value, and optional-value schemas so
/// typos or unknown flags fail fast instead of silently falling through to default behavior.
pub fn validate_cli_args(
    args: &[String],
    bool_flags: &[&str],
    value_flags: &[&str],
    optional_value_flags: &[&str],
) -> Result<(), String> {
    let mut i = 0usize;
    while i < args.len() {
        let arg = args[i].as_str();
        if bool_flags.contains(&arg) {
            i += 1;
            continue;
        }
        if value_flags.contains(&arg) {
            if i + 1 >= args.len() || args[i + 1].starts_with("--") {
                return Err(format!("Flag {arg} requires a value."));
            }
            i += 2;
            continue;
        }
        if optional_value_flags.contains(&arg) {
            if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if let Some((flag, val)) = arg.split_once('=') {
            if value_flags.contains(&flag) || optional_value_flags.contains(&flag) {
                if val.is_empty() {
                    return Err(format!("Flag {flag}= requires a non-empty value."));
                }
                i += 1;
                continue;
            }
        }
        return Err(format!("Unrecognized CLI argument: {arg}"));
    }
    Ok(())
}
