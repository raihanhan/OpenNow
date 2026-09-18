use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub enum TrayAction {
    Show,
    Quit,
}

pub fn create_tray_icon(
    action_tx: mpsc::Sender<TrayAction>,
) -> anyhow::Result<(TrayIcon, mpsc::Receiver<TrayAction>)> {
    let (_tray_tx, tray_rx) = mpsc::channel();

    let show_item = MenuItem::new("Show", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    let separator = PredefinedMenuItem::separator();

    let menu = Menu::new();
    menu.append(&show_item)?;
    menu.append(&separator)?;
    menu.append(&quit_item)?;

    let show_id = show_item.id().clone();
    let quit_id = quit_item.id().clone();

    let action_tx_clone = action_tx.clone();
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = MenuEvent::receiver().recv() {
                if event.id == show_id {
                    let _ = action_tx_clone.send(TrayAction::Show);
                } else if event.id == quit_id {
                    let _ = action_tx_clone.send(TrayAction::Quit);
                }
            }
        }
    });

    let icon = load_tray_icon()?;

    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("Shortcut Manager")
        .build()?;

    let action_tx_clone = action_tx.clone();
    std::thread::spawn(move || {
        loop {
            if let Ok(event) = TrayIconEvent::receiver().recv() {
                match event {
                    TrayIconEvent::Click { .. } => {
                        let _ = action_tx_clone.send(TrayAction::Show);
                    }
                    _ => {}
                }
            }
        }
    });

    Ok((tray, tray_rx))
}

fn load_tray_icon() -> anyhow::Result<tray_icon::Icon> {
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

    Ok(tray_icon::Icon::from_rgba(pixels, size as u32, size as u32)?)
}