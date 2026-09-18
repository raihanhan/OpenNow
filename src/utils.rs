use std::path::{Path, PathBuf};
use std::process::Command;
use crate::models::get_icon_cache_dir;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to launch: {0}")]
    Failed(String),
}

pub type Result<T> = std::result::Result<T, LaunchError>;

pub fn launch_target(target: &str, args: &str) -> Result<()> {
    let is_url = target.starts_with("http://") || target.starts_with("https://");

    if is_url {
        open::that(target)?;
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        launch_windows(target, args)?;
    }

    #[cfg(target_os = "macos")]
    {
        launch_macos(target, args)?;
    }

    #[cfg(target_os = "linux")]
    {
        launch_linux(target, args)?;
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_windows(target: &str, args: &str) -> Result<()> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    let target_path = Path::new(target);
    let is_exe = target_path
        .extension()
        .map(|e| e.eq_ignore_ascii_case("exe"))
        .unwrap_or(false);

    if is_exe && !args.is_empty() {
        let mut cmd = Command::new(target);
        cmd.args(args.split_whitespace());
        cmd.creation_flags(0x00000010); // CREATE_NEW_CONSOLE
        cmd.spawn()?;
    } else {
        // Use ShellExecute for proper file associations
        let target_wide: Vec<u16> = OsStr::new(target).encode_wide().chain(Some(0)).collect();
        let args_wide: Vec<u16> = if args.is_empty() {
            vec![0]
        } else {
            OsStr::new(args).encode_wide().chain(Some(0)).collect()
        };

        unsafe {
            let result = windows::Win32::UI::Shell::ShellExecuteW(
                None,
                windows::core::PCWSTR::null(),
                windows::core::PCWSTR::from_raw(target_wide.as_ptr()),
                if args.is_empty() {
                    windows::core::PCWSTR::null()
                } else {
                    windows::core::PCWSTR::from_raw(args_wide.as_ptr())
                },
                None,
                windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
            );
            if (result.0 as isize) <= 32 {
                return Err(LaunchError::Failed(format!(
                    "ShellExecute failed with code: {:?}",
                    result.0
                )));
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn launch_macos(target: &str, args: &str) -> Result<()> {
    let mut cmd = Command::new("open");
    cmd.arg(target);
    if !args.is_empty() {
        cmd.arg("--args");
        cmd.args(args.split_whitespace());
    }
    cmd.spawn()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn launch_linux(target: &str, args: &str) -> Result<()> {
    let target_path = Path::new(target);
    let is_dir = target_path.is_dir();
    let is_executable = target_path.exists() && !is_dir && {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(target_path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    };

    if is_dir || (!is_executable && target_path.exists()) {
        Command::new("xdg-open").arg(target).spawn()?;
    } else {
        let mut cmd = Command::new(target);
        if !args.is_empty() {
            cmd.args(args.split_whitespace());
        }
        cmd.spawn()?;
    }
    Ok(())
}

fn icon_cache_path(target: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    target.hash(&mut hasher);
    let key = hasher.finish();
    get_icon_cache_dir().join(format!("{:x}.png", key))
}

pub fn extract_icon(target: &str) -> Option<PathBuf> {
    let cache_path = icon_cache_path(target);
    if cache_path.exists() {
        return Some(cache_path);
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(path) = extract_icon_windows(target, &cache_path) {
            return Some(path);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(path) = extract_icon_macos(target, &cache_path) {
            return Some(path);
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(path) = extract_icon_linux(target, &cache_path) {
            return Some(path);
        }
    }

    None
}

#[cfg(target_os = "windows")]
fn extract_icon_windows(target: &str, cache_path: &Path) -> Option<PathBuf> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::*;
    use windows::Win32::Graphics::Gdi::*;
    use windows::Win32::UI::Shell::*;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::*;

    unsafe {
        let target_wide: Vec<u16> = OsStr::new(target)
            .encode_wide()
            .chain(Some(0))
            .collect();

        let mut large_icons = [HICON::default(); 1];
        let mut small_icons = [HICON::default(); 1];

        let count = ExtractIconExW(
            PCWSTR::from_raw(target_wide.as_ptr()),
            0,
            Some(large_icons.as_mut_ptr()),
            Some(small_icons.as_mut_ptr()),
            1,
        );

        if count == 0 || large_icons[0].is_invalid() {
            return None;
        }

        let hicon = large_icons[0];
        let ico_x = GetSystemMetrics(SM_CXICON);
        let ico_y = GetSystemMetrics(SM_CYICON);

        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let hbitmap = CreateCompatibleBitmap(hdc_screen, ico_x, ico_y);
        let _old_bitmap = SelectObject(hdc_mem, HGDIOBJ(hbitmap.0));

        DrawIcon(hdc_mem, 0, 0, hicon);

        let mut bmp_info = BITMAP::default();
        GetObjectW(HGDIOBJ(hbitmap.0), std::mem::size_of::<BITMAP>() as i32, Some(&mut bmp_info as *mut _ as *mut _));

        let width = bmp_info.bmWidth as u32;
        let height = bmp_info.bmHeight as u32;

        let mut bits = vec![0u8; (width * height * 4) as usize];
        let mut bmi = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32), // Top-down DIB
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            biSizeImage: 0,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0,
        };

        GetDIBits(
            hdc_mem,
            hbitmap,
            0,
            height,
            Some(bits.as_mut_ptr() as *mut _),
            &mut bmi as *mut _ as *mut _,
            DIB_RGB_COLORS,
        );

        // Convert BGRA to RGBA
        for chunk in bits.chunks_exact_mut(4) {
            chunk.swap(0, 2);
        }

        let img = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(width, height, bits)?;
        img.save(cache_path).ok()?;

        // Cleanup
        SelectObject(hdc_mem, _old_bitmap);
        DeleteObject(HGDIOBJ(hbitmap.0));
        DeleteDC(hdc_mem);
        ReleaseDC(None, hdc_screen);
        DestroyIcon(hicon);

        Some(cache_path.to_path_buf())
    }
}

#[cfg(target_os = "macos")]
fn extract_icon_macos(_target: &str, _cache_path: &Path) -> Option<PathBuf> {
    // Could use `iconutil` or `sips` but requires spawning process
    // For now, return None - egui will show default icon
    None
}

#[cfg(target_os = "linux")]
fn extract_icon_linux(_target: &str, _cache_path: &Path) -> Option<PathBuf> {
    // Could use `gio` or `gtk` but adds heavy deps
    None
}

pub fn ensure_icon_cache_dir() {
    let _ = std::fs::create_dir_all(get_icon_cache_dir());
}