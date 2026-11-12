use crate::registry::{wide_z, BoundedLogger, CREATE_NO_WINDOW};
use std::{
    ffi::c_void,
    fs,
    os::windows::process::CommandExt,
    path::Path,
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
    time::Duration,
};

pub const REPO_RAW: &str =
    "https://raw.githubusercontent.com/emvaized/modern_inverted_mouse_cursors/3.6.2/src";

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

#[link(name = "kernel32")]
extern "system" {
    fn GetLastError() -> u32;
    fn LoadLibraryW(lp_lib_file_name: *const u16) -> *mut c_void;
    fn GetProcAddress(h_module: *mut c_void, lp_proc_name: *const u8) -> *mut c_void;
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
        for (i, dst) in w[..16].iter_mut().enumerate() {
            let j = i * 4;
            *dst = u32::from_be_bytes([chunk[j], chunk[j + 1], chunk[j + 2], chunk[j + 3]]);
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
                // SAFETY: Valid HINTERNET handle closed via `WinHttpCloseHandle`.
                unsafe {
                    (self.close_fn)(self.handle);
                }
            }
        }
    }

    // SAFETY: Dynamically resolves WinHTTP exports cleanly with RAII handle cleanup.
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

        let agent = wide_z("ModernInvertedCursors/1.1");
        let mut h_session = open_fn(agent.as_ptr(), 4, std::ptr::null(), std::ptr::null(), 0);
        if h_session.is_null() {
            h_session = open_fn(agent.as_ptr(), 0, std::ptr::null(), std::ptr::null(), 0);
        }
        if h_session.is_null() {
            return Err(format!("WinHttpOpen failed (err={})", GetLastError()));
        }
        let session = SafeWinHttpHandle { handle: h_session, close_fn };

        let tls_12_13: u32 = 0x0000_0800 | 0x0000_2000;
        if set_opt_fn(
            session.handle,
            84,
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
            0x0080_0000,
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
            19 | 0x2000_0000,
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
    for asset in ASSETS {
        let p = dir.join(asset.name);
        if p.exists() && verify_asset_file(*asset, &p).is_ok() {
            continue;
        }
        missing.push(*asset);
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
