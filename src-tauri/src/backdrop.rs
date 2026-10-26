//! Read-only desktop sampling for the Liquid Glass material.
//!
//! The window is transparent, and WebView2's `backdrop-filter` can only sample
//! the page — not the desktop. We therefore grab a small, heavily downscaled
//! snapshot of the screen area behind the window while it is still hidden. The
//! renderer blurs that snapshot and paints it underneath the glass tint, which
//! gives real desktop colour and soft refraction instead of a flat colour.
//!
//! Nothing here mutates a window: it is `GetDC` + `StretchBlt` into an
//! off-screen DIB, then a tiny BMP data URL. All handles are released on every
//! path.

#[cfg(windows)]
mod imp {
    use std::ffi::c_void;
    use windows_sys::Win32::Graphics::Gdi::{
        CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GdiFlush, GetDC, ReleaseDC,
        SelectObject, SetStretchBltMode, StretchBlt, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        DIB_RGB_COLORS, HALFTONE, HGDIOBJ, SRCCOPY,
    };

    const TARGET_WIDTH: u32 = 120;
    const MIN_HEIGHT: u32 = 40;
    const MAX_HEIGHT: u32 = 180;

    pub fn capture_data_url(x: i32, y: i32, width: u32, height: u32) -> Option<String> {
        if width == 0 || height == 0 {
            return None;
        }

        let target_height = ((TARGET_WIDTH as f64 * height as f64 / width as f64).round() as u32)
            .clamp(MIN_HEIGHT, MAX_HEIGHT);

        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            if screen.is_null() {
                return None;
            }

            let memory = CreateCompatibleDC(screen);
            if memory.is_null() {
                ReleaseDC(std::ptr::null_mut(), screen);
                return None;
            }

            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = TARGET_WIDTH as i32;
            // Negative height keeps the DIB top-down, matching the read loop below.
            info.bmiHeader.biHeight = -(target_height as i32);
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            info.bmiHeader.biCompression = BI_RGB;

            let mut bits: *mut c_void = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                memory,
                &info,
                DIB_RGB_COLORS,
                &mut bits,
                std::ptr::null_mut(),
                0,
            );
            if bitmap.is_null() || bits.is_null() {
                DeleteDC(memory);
                ReleaseDC(std::ptr::null_mut(), screen);
                return None;
            }

            let previous = SelectObject(memory, bitmap as HGDIOBJ);
            SetStretchBltMode(memory, HALFTONE);
            let copied = StretchBlt(
                memory,
                0,
                0,
                TARGET_WIDTH as i32,
                target_height as i32,
                screen,
                x,
                y,
                width as i32,
                height as i32,
                SRCCOPY,
            );
            GdiFlush();

            let pixels = if copied != 0 {
                let len = (TARGET_WIDTH * target_height * 4) as usize;
                Some(std::slice::from_raw_parts(bits as *const u8, len).to_vec())
            } else {
                None
            };

            SelectObject(memory, previous);
            DeleteObject(bitmap as HGDIOBJ);
            DeleteDC(memory);
            ReleaseDC(std::ptr::null_mut(), screen);

            pixels.map(|pixels| bmp_data_url(TARGET_WIDTH, target_height, &pixels))
        }
    }

    fn bmp_data_url(width: u32, height: u32, bgra: &[u8]) -> String {
        let row = (width * 4) as usize;
        let image_size = row * height as usize;
        let file_size = 54 + image_size;
        let mut out = Vec::with_capacity(file_size);

        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(file_size as u32).to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&(width as i32).to_le_bytes());
        out.extend_from_slice(&(height as i32).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(image_size as u32).to_le_bytes());
        out.extend_from_slice(&[0u8; 16]);

        // BMP rows are bottom-up; the DIB we read is top-down.
        for y in (0..height as usize).rev() {
            let start = y * row;
            out.extend_from_slice(&bgra[start..start + row]);
        }

        format!("data:image/bmp;base64,{}", base64(&out))
    }

    fn base64(input: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
        for chunk in input.chunks(3) {
            let b0 = chunk[0] as u32;
            let b1 = *chunk.get(1).unwrap_or(&0) as u32;
            let b2 = *chunk.get(2).unwrap_or(&0) as u32;
            let triple = (b0 << 16) | (b1 << 8) | b2;
            out.push(TABLE[((triple >> 18) & 63) as usize] as char);
            out.push(TABLE[((triple >> 12) & 63) as usize] as char);
            out.push(if chunk.len() > 1 {
                TABLE[((triple >> 6) & 63) as usize] as char
            } else {
                '='
            });
            out.push(if chunk.len() > 2 {
                TABLE[(triple & 63) as usize] as char
            } else {
                '='
            });
        }
        out
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn capture_data_url(_x: i32, _y: i32, _width: u32, _height: u32) -> Option<String> {
        None
    }
}

pub use imp::capture_data_url;
