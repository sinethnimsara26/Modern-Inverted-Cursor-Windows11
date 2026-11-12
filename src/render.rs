use crate::registry::Preset;
use std::ffi::c_void;

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
    fn CreateIconIndirect(piconinfo: *const IconInfo) -> *mut c_void;
    pub fn DestroyCursor(cursor: *mut c_void) -> i32;
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

type PolySpec = (&'static [(f32, f32)], Option<&'static [(f32, f32)]>, f32);

fn preset_polys(preset: Preset) -> PolySpec {
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
#[allow(clippy::too_many_arguments)]
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

/// Pre-rasterized premultiplied ARGB (`0xAARRGGBB`) cursor glyph sprite cached per DPI/size
/// so interactive UI repaints composite via fast integer alpha blending without re-running
/// per-pixel 4x4 polygon distance math.
#[derive(Clone)]
pub struct CursorGlyphSprite {
    pub size: i32,
    pub premul: Vec<u32>,
}

impl CursorGlyphSprite {
    pub fn rasterize(size: i32, preset: Preset, paused: bool) -> Self {
        let side = size.max(1) as usize;
        let mut premul = vec![0u32; side * side];
        draw_cursor_glyph(&mut premul, side, 0, 0, size, preset, paused, true);
        Self { size, premul }
    }

    pub fn blit_onto(&self, pixels: &mut [u32], stride: usize, x0: i32, y0: i32) {
        if self.size <= 0 {
            return;
        }
        let side = self.size as usize;
        for dy in 0..self.size {
            let py = y0 + dy;
            if py < 0 {
                continue;
            }
            let row_dst = py as usize * stride;
            if row_dst >= pixels.len() {
                break;
            }
            let row_src = dy as usize * side;
            for dx in 0..self.size {
                let px = x0 + dx;
                if px < 0 || px as usize >= stride {
                    continue;
                }
                let idx = row_dst + px as usize;
                if idx >= pixels.len() {
                    continue;
                }
                let src = self.premul[row_src + dx as usize];
                let a = (src >> 24) & 0xFF;
                if a == 0 {
                    continue;
                }
                if a == 255 {
                    pixels[idx] = 0xFF00_0000 | (src & 0x00FF_FFFF);
                    continue;
                }
                let inv = 255 - a;
                let dst = pixels[idx];
                let out_r = (((src >> 16) & 0xFF) + (((dst >> 16) & 0xFF) * inv + 127) / 255).min(255);
                let out_g = (((src >> 8) & 0xFF) + (((dst >> 8) & 0xFF) * inv + 127) / 255).min(255);
                let out_b = ((src & 0xFF) + ((dst & 0xFF) * inv + 127) / 255).min(255);
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
    // SAFETY: `CreateDIBSection` allocates `size * size * 4` pixels owned by `hbm_color`, destroyed before return.
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

    // SAFETY: `bits_ptr` is valid for `size * size` `u32` elements while `hbm_color` is live.
    let pixels = unsafe { std::slice::from_raw_parts_mut(bits_ptr as *mut u32, size * size) };
    pixels.fill(0);
    draw_cursor_glyph(pixels, size, 0, 0, size as i32, preset, paused, true);

    let mask_bytes = vec![0u8; size * size.div_ceil(8)];
    // SAFETY: `mask_bytes` meets monochrome bitmap stride/size; `hbm_mask` and `hbm_color` are freed via `DeleteObject`.
    unsafe {
        let hbm_mask = CreateBitmap(
            size as i32,
            size as i32,
            1,
            1,
            mask_bytes.as_ptr() as *const c_void,
        );
        let info = IconInfo {
            f_icon: 1,
            x_hotspot: 0,
            y_hotspot: 0,
            hbm_mask,
            hbm_color,
        };
        let hicon = CreateIconIndirect(&info);
        if !hbm_mask.is_null() {
            DeleteObject(hbm_mask);
        }
        DeleteObject(hbm_color);
        hicon
    }
}
