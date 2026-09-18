use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HotkeyAction {
    TogglePopup,
    LaunchShortcut(String),
}

pub struct HotkeyManager {
    manager: Arc<GlobalHotKeyManager>,
    bindings: Arc<Mutex<HashMap<u32, (HotKey, HotkeyAction)>>>,
    action_tx: mpsc::Sender<HotkeyAction>,
}

impl Clone for HotkeyManager {
    fn clone(&self) -> Self {
        Self {
            manager: Arc::clone(&self.manager),
            bindings: Arc::clone(&self.bindings),
            action_tx: self.action_tx.clone(),
        }
    }
}

impl HotkeyManager {
    pub fn new(action_tx: mpsc::Sender<HotkeyAction>) -> anyhow::Result<Self> {
        let manager = Arc::new(GlobalHotKeyManager::new()?);
        Ok(Self {
            manager,
            bindings: Arc::new(Mutex::new(HashMap::new())),
            action_tx,
        })
    }

    pub fn register_toggle(&self, modifiers: Modifiers, key: Code) -> anyhow::Result<()> {
        let hotkey = HotKey::new(Some(modifiers), key);
        let id = hotkey.id();
        self.manager.register(hotkey)?;
        self.bindings.lock().insert(id, (hotkey, HotkeyAction::TogglePopup));
        Ok(())
    }

    pub fn register_shortcut(&self, id: &str, hotkey_str: &str) -> anyhow::Result<()> {
        let (modifiers, key) = parse_hotkey(hotkey_str)?;
        let hotkey = HotKey::new(Some(modifiers), key);
        let hk_id = hotkey.id();
        self.manager.register(hotkey)?;
        self.bindings
            .lock()
            .insert(hk_id, (hotkey, HotkeyAction::LaunchShortcut(id.to_string())));
        Ok(())
    }

    pub fn unregister_all(&self) -> anyhow::Result<()> {
        let bindings = self.bindings.lock();
        for (hk, _) in bindings.values() {
            let _ = self.manager.unregister(*hk);
        }
        Ok(())
    }

    pub fn rebuild(&self, shortcuts: &[crate::models::Shortcut]) -> anyhow::Result<()> {
        self.unregister_all()?;
        self.bindings.lock().clear();

        self.register_toggle(Modifiers::CONTROL | Modifiers::ALT, Code::Space)?;

        for s in shortcuts {
            if let Some(hk) = &s.hotkey {
                if let Err(e) = self.register_shortcut(&s.id, hk) {
                    eprintln!("Failed to register hotkey '{}' for '{}': {}", hk, s.name, e);
                }
            }
        }
        Ok(())
    }

    pub fn handle_event(&self, event: GlobalHotKeyEvent) {
        if let Some((_, action)) = self.bindings.lock().get(&event.id) {
            let _ = self.action_tx.send(action.clone());
        }
    }
}

fn parse_hotkey(s: &str) -> anyhow::Result<(Modifiers, Code)> {
    let parts: Vec<String> = s.split('+').map(|p| p.trim().to_lowercase()).collect();
    let mut modifiers = Modifiers::empty();
    let mut key = None;

    for part in parts {
        match part.as_str() {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "super" | "win" | "cmd" | "meta" => modifiers |= Modifiers::SUPER,
            k => {
                key = Some(parse_key_code(k)?);
            }
        }
    }

    let key = key.ok_or_else(|| anyhow::anyhow!("No key in hotkey: {}", s))?;
    Ok((modifiers, key))
}

fn parse_key_code(s: &str) -> anyhow::Result<Code> {
    use Code::*;
    Ok(match s {
        "space" => Space,
        "enter" => Enter,
        "tab" => Tab,
        "escape" | "esc" => Escape,
        "backspace" => Backspace,
        "delete" => Delete,
        "home" => Home,
        "end" => End,
        "pageup" => PageUp,
        "pagedown" => PageDown,
        "up" => ArrowUp,
        "down" => ArrowDown,
        "left" => ArrowLeft,
        "right" => ArrowRight,
        "f1" => F1,
        "f2" => F2,
        "f3" => F3,
        "f4" => F4,
        "f5" => F5,
        "f6" => F6,
        "f7" => F7,
        "f8" => F8,
        "f9" => F9,
        "f10" => F10,
        "f11" => F11,
        "f12" => F12,
        "0" => Digit0,
        "1" => Digit1,
        "2" => Digit2,
        "3" => Digit3,
        "4" => Digit4,
        "5" => Digit5,
        "6" => Digit6,
        "7" => Digit7,
        "8" => Digit8,
        "9" => Digit9,
        "a" => KeyA,
        "b" => KeyB,
        "c" => KeyC,
        "d" => KeyD,
        "e" => KeyE,
        "f" => KeyF,
        "g" => KeyG,
        "h" => KeyH,
        "i" => KeyI,
        "j" => KeyJ,
        "k" => KeyK,
        "l" => KeyL,
        "m" => KeyM,
        "n" => KeyN,
        "o" => KeyO,
        "p" => KeyP,
        "q" => KeyQ,
        "r" => KeyR,
        "s" => KeyS,
        "t" => KeyT,
        "u" => KeyU,
        "v" => KeyV,
        "w" => KeyW,
        "x" => KeyX,
        "y" => KeyY,
        "z" => KeyZ,
        _ => anyhow::bail!("Unknown key: {}", s),
    })
}