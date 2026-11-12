use crate::registry::{wide_z, BoundedLogger, CREATE_NO_WINDOW, TRAY_WND_CLASS};
use std::{
    ffi::c_void,
    fs,
    os::windows::process::CommandExt,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Duration,
};

#[repr(C)]
pub struct ProcessEntry32W {
    pub dw_size: u32,
    pub cnt_usage: u32,
    pub th32_process_id: u32,
    pub th32_default_heap_id: usize,
    pub th32_module_id: u32,
    pub cnt_threads: u32,
    pub th32_parent_process_id: u32,
    pub pc_pri_class_base: i32,
    pub dw_flags: u32,
    pub sz_exe_file: [u16; 260],
}

#[link(name = "user32")]
extern "system" {
    fn FindWindowW(lp_class_name: *const u16, lp_window_name: *const u16) -> *mut c_void;
    fn PostMessageW(h_wnd: *mut c_void, msg: u32, w_param: usize, l_param: isize) -> i32;
    fn GetWindowThreadProcessId(h_wnd: *mut c_void, lpdw_process_id: *mut u32) -> u32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetLastError() -> u32;
    fn GetCurrentProcessId() -> u32;
    fn GetModuleHandleW(lp_module_name: *const u16) -> *mut c_void;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const u8) -> *mut c_void;
    fn OpenProcess(dw_desired_access: u32, b_inherit_handle: i32, dw_process_id: u32) -> *mut c_void;
    fn TerminateProcess(h_process: *mut c_void, u_exit_code: u32) -> i32;
    fn WaitForSingleObject(h_handle: *mut c_void, dw_milliseconds: u32) -> u32;
    fn WaitForMultipleObjects(
        n_count: u32,
        lp_handles: *const *mut c_void,
        b_wait_all: i32,
        dw_milliseconds: u32,
    ) -> u32;
    fn CreateEventW(
        lp_event_attributes: *mut c_void,
        b_manual_reset: i32,
        b_initial_state: i32,
        lp_name: *const u16,
    ) -> *mut c_void;
    fn OpenEventW(dw_desired_access: u32, b_inherit_handle: i32, lp_name: *const u16) -> *mut c_void;
    fn SetEvent(h_event: *mut c_void) -> i32;
    pub fn CloseHandle(h_object: *mut c_void) -> i32;
    fn CreateToolhelp32Snapshot(dw_flags: u32, th32_process_id: u32) -> *mut c_void;
    fn Process32FirstW(h_snapshot: *mut c_void, lppe: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(h_snapshot: *mut c_void, lppe: *mut ProcessEntry32W) -> i32;
}

pub fn find_tray_window() -> *mut c_void {
    let w_cls = wide_z(TRAY_WND_CLASS);
    // SAFETY: `w_cls` is NUL-terminated UTF-16 passed to `FindWindowW`.
    unsafe { FindWindowW(w_cls.as_ptr(), std::ptr::null()) }
}

pub fn window_process_id(hwnd: *mut c_void) -> Option<u32> {
    if hwnd.is_null() {
        return None;
    }
    let mut pid: u32 = 0;
    // SAFETY: `pid` points to initialized stack storage for a non-null `HWND`.
    unsafe {
        let _ = GetWindowThreadProcessId(hwnd, &mut pid);
    }
    if pid != 0 {
        Some(pid)
    } else {
        None
    }
}

pub fn normalize_win_path_str(p: &Path) -> String {
    let s = p.to_string_lossy().replace('/', "\\");
    s.strip_prefix(r"\\?\")
        .unwrap_or(&s)
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

pub fn is_guardian_image_path(exe_path: &Path, allowed_root: &Path) -> bool {
    let exe_norm = normalize_win_path_str(exe_path);
    let root_norm = normalize_win_path_str(allowed_root);
    if root_norm.is_empty() || !exe_norm.starts_with(&(root_norm + "\\")) {
        return false;
    }
    let file_name = exe_path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    file_name == "moderninvertedcursorguardian.exe"
        || file_name.starts_with("moderninvertedcursors-guardian")
        || file_name.starts_with("moderninvertedinstallerv5")
}

pub fn process_image_path(pid: u32) -> Option<PathBuf> {
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    type QueryFullImageFn =
        unsafe extern "system" fn(*mut c_void, u32, *mut u16, *mut u32) -> i32;

    // SAFETY: `kernel32.dll` stays loaded in every Win32 process; opened process handle is closed before returning.
    unsafe {
        let k32 = GetModuleHandleW(wide_z("kernel32.dll").as_ptr());
        if k32.is_null() {
            return None;
        }
        let proc = GetProcAddress(k32, b"QueryFullProcessImageNameW\0".as_ptr());
        if proc.is_null() {
            return None;
        }
        let query_fn: QueryFullImageFn = std::mem::transmute(proc);
        let h_proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h_proc.is_null() {
            return None;
        }
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = query_fn(h_proc, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(h_proc);
        if ok != 0 && len > 0 {
            Some(PathBuf::from(String::from_utf16_lossy(&buf[..len as usize])))
        } else {
            None
        }
    }
}

pub fn find_verified_guardian_pids(allowed_root: &Path) -> Vec<u32> {
    const TH32CS_SNAPPROCESS: u32 = 0x0000_0002;
    let self_pid = unsafe { GetCurrentProcessId() };
    let mut pids = Vec::new();
    // SAFETY: Snapshot handle is closed before returning; `dw_size` initialized cleanly.
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap.is_null() || snap as isize == -1 {
            return pids;
        }
        let mut pe: ProcessEntry32W = std::mem::zeroed();
        pe.dw_size = std::mem::size_of::<ProcessEntry32W>() as u32;
        if Process32FirstW(snap, &mut pe) != 0 {
            loop {
                if pe.th32_process_id != 0 && pe.th32_process_id != self_pid {
                    let end = pe
                        .sz_exe_file
                        .iter()
                        .position(|&c| c == 0)
                        .unwrap_or(pe.sz_exe_file.len());
                    let exe_name =
                        String::from_utf16_lossy(&pe.sz_exe_file[..end]).to_ascii_lowercase();
                    if exe_name.contains("moderninverted") {
                        if let Some(img_path) = process_image_path(pe.th32_process_id) {
                            if is_guardian_image_path(&img_path, allowed_root) {
                                pids.push(pe.th32_process_id);
                            }
                        }
                    }
                }
                if Process32NextW(snap, &mut pe) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    pids
}

fn terminate_verified_pid(
    pid: u32,
    graceful_hwnd: *mut c_void,
    log: Option<&BoundedLogger>,
) -> Result<bool, String> {
    const SYNCHRONIZE: u32 = 0x0010_0000;
    const PROCESS_TERMINATE: u32 = 0x0001;
    const WM_CLOSE: u32 = 0x0010;
    const WAIT_OBJECT_0: u32 = 0;

    // SAFETY: Operates strictly on a PID verified via `is_guardian_image_path` and closes `h_proc`.
    unsafe {
        let h_proc = OpenProcess(SYNCHRONIZE | PROCESS_TERMINATE, 0, pid);
        if h_proc.is_null() {
            return Ok(false);
        }
        if !graceful_hwnd.is_null() {
            PostMessageW(graceful_hwnd, WM_CLOSE, 0, 0);
            if WaitForSingleObject(h_proc, 3500) == WAIT_OBJECT_0 {
                CloseHandle(h_proc);
                if let Some(l) = log {
                    l.line(format!("[stop] Guardian PID {pid} exited cleanly."));
                }
                return Ok(true);
            }
        }
        let term_ok = TerminateProcess(h_proc, 1) != 0;
        let wait = WaitForSingleObject(h_proc, 2500);
        CloseHandle(h_proc);
        if wait == WAIT_OBJECT_0 {
            if let Some(l) = log {
                l.line(format!(
                    "[stop] Terminated verified guardian PID {pid} cleanly (graceful fallback)."
                ));
            }
            return Ok(true);
        }
        Err(format!(
            "Verified guardian process PID {pid} did not terminate in time (terminate_ok={term_ok}, wait={wait})."
        ))
    }
}

pub fn stop_targeted_guardians(
    allowed_root: &Path,
    log: Option<&BoundedLogger>,
) -> Result<usize, String> {
    let mut stopped = 0usize;
    let self_pid = unsafe { GetCurrentProcessId() };

    let hwnd = find_tray_window();
    if let Some(pid) = window_process_id(hwnd) {
        if pid != self_pid {
            let allowed = process_image_path(pid)
                .map(|p| is_guardian_image_path(&p, allowed_root))
                .unwrap_or(false);
            if allowed && terminate_verified_pid(pid, hwnd, log)? {
                stopped += 1;
            }
        }
    }

    for pid in find_verified_guardian_pids(allowed_root) {
        if terminate_verified_pid(pid, std::ptr::null_mut(), log)? {
            stopped += 1;
        }
    }
    Ok(stopped)
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReadyWaitOutcome {
    Ready,
    ExitedEarly,
    TimedOut,
    WaitError(u32),
}

pub struct NamedReadyEvent {
    handle: *mut c_void,
    pub name: String,
}

impl NamedReadyEvent {
    pub fn create_unique(tag: &str) -> Result<Self, String> {
        static SEQ: AtomicUsize = AtomicUsize::new(1);
        let pid = unsafe { GetCurrentProcessId() };
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        let name = format!("Local\\ModernInvertedReady_{tag}_{pid}_{seq}");
        let w_name = wide_z(&name);
        // SAFETY: Valid NUL-terminated UTF-16 event name; closed once in `Drop`.
        let handle = unsafe { CreateEventW(std::ptr::null_mut(), 1, 0, w_name.as_ptr()) };
        if handle.is_null() {
            return Err(format!("CreateEventW failed (err={})", unsafe {
                GetLastError()
            }));
        }
        Ok(Self { handle, name })
    }

    pub fn signal_by_name(name: &str) -> Result<(), String> {
        const EVENT_MODIFY_STATE: u32 = 0x0002;
        let w_name = wide_z(name);
        // SAFETY: Opens existing event by name and closes handle before returning.
        unsafe {
            let h = OpenEventW(EVENT_MODIFY_STATE, 0, w_name.as_ptr());
            if h.is_null() {
                return Err(format!(
                    "OpenEventW({name}) failed (err={})",
                    GetLastError()
                ));
            }
            let ok = SetEvent(h);
            let err = GetLastError();
            CloseHandle(h);
            if ok == 0 {
                return Err(format!("SetEvent({name}) failed (err={err})"));
            }
        }
        Ok(())
    }

    pub fn wait_process_or_ready(
        &self,
        proc_handle: *mut c_void,
        timeout_ms: u32,
    ) -> ReadyWaitOutcome {
        const WAIT_OBJECT_0: u32 = 0;
        const WAIT_OBJECT_1: u32 = 1;
        const WAIT_TIMEOUT: u32 = 258;
        let handles = [self.handle, proc_handle];
        // SAFETY: Both synchronization handles remain valid during the wait.
        let res = unsafe { WaitForMultipleObjects(2, handles.as_ptr(), 0, timeout_ms) };
        match res {
            WAIT_OBJECT_0 => ReadyWaitOutcome::Ready,
            WAIT_OBJECT_1 => ReadyWaitOutcome::ExitedEarly,
            WAIT_TIMEOUT => ReadyWaitOutcome::TimedOut,
            other => ReadyWaitOutcome::WaitError(other),
        }
    }
}

impl Drop for NamedReadyEvent {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            // SAFETY: Valid `CreateEventW` handle owned exclusively by `NamedReadyEvent`.
            unsafe {
                CloseHandle(self.handle);
            }
        }
    }
}

pub fn read_log_tail(path: &Path, max_lines: usize) -> String {
    let Ok(text) = fs::read_to_string(path) else {
        return String::new();
    };
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join(" | ")
}

pub fn close_existing_v6_window() {
    const WM_CLOSE: u32 = 0x0010;
    let existing = find_tray_window();
    if !existing.is_null() {
        // SAFETY: `existing` returned by `FindWindowW` for our tray window class.
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

pub fn write_binary_retry(dest: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    }
    let mut last_err = None;
    for _ in 0..25 {
        match fs::write(dest, bytes) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                thread::sleep(Duration::from_millis(100));
            }
        }
    }
    Err(format!(
        "Could not write {}: {}",
        dest.display(),
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
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}
