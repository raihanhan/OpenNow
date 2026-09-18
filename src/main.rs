mod models;
mod utils;
mod hotkeys;
mod tray;
mod app;

use crate::app::ShortcutManagerApp;
use crate::models::get_data_dir;
use crate::utils::ensure_icon_cache_dir;
use anyhow::Result;
use eframe::egui;
use std::sync::Arc;

#[cfg(target_os = "linux")]
fn init_gtk() {
    if std::env::var("GTK_DEBUG").is_err() {
        let _ = gtk::init();
    }
}

fn main() -> Result<()> {
    #[cfg(target_os = "linux")]
    init_gtk();

    // Ensure data directories exist
    std::fs::create_dir_all(get_data_dir())?;
    ensure_icon_cache_dir();

    // Configure native options
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 480.0])
            .with_min_inner_size([400.0, 300.0])
            .with_title("Shortcut Manager")
            .with_icon(Arc::new(load_window_icon())),
        centered: true,
        persist_window: true,
        ..Default::default()
    };

    let result = eframe::run_native(
        "Shortcut Manager",
        native_options,
        Box::new(|cc| {
            Ok(Box::new(ShortcutManagerApp::new(cc)?) as Box<dyn eframe::App>)
        }),
    );

    if let Err(e) = result {
        eprintln!("Application error: {}", e);
        return Err(anyhow::anyhow!("eframe error: {}", e));
    }

    Ok(())
}

fn load_window_icon() -> egui::IconData {
    let size = 32;
    let mut pixels = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            pixels[idx] = 0x58;
            pixels[idx + 1] = 0x65;
            pixels[idx + 2] = 0xF2;
            pixels[idx + 3] = 0xFF;

            let in_s = (x >= 8 && x <= 24) && (y >= 6 && y <= 26) && (
                (y <= 12 && (x <= 10 || x >= 22)) ||
                (y >= 14 && y <= 18 && x <= 22) ||
                (y >= 20 && (x <= 10 || x >= 22))
            );

            if in_s {
                pixels[idx] = 0xFF;
                pixels[idx + 1] = 0xFF;
                pixels[idx + 2] = 0xFF;
            }
        }
    }

    egui::IconData {
        rgba: pixels,
        width: size as u32,
        height: size as u32,
    }
}