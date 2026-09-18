# Open Now

Cross-platform shortcut manager with global hotkeys and system tray. Like Spotlight/Alfred but for your custom shortcuts.

![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blue)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)
![License](https://img.shields.io/badge/license-MIT-green)

## Features

- **Global Hotkeys** - Launch shortcuts instantly with `Ctrl+Alt+Space` (configurable)
- **Launcher Popup** - Fast keyboard-driven search (type to filter, Enter to launch)
- **System Tray** - Always accessible from menu bar / taskbar
- **Categories** - Organize shortcuts (General, Development, Media, etc.)
- **Launch Count Tracking** - Most used shortcuts appear first
- **Cross-platform** - Windows, Linux, macOS (Intel + Apple Silicon)
- **Portable** - Single binary, no installer needed

## Screenshots

| Main Window | Launcher Popup | System Tray |
|-------------|----------------|-------------|
| ![Main](docs/main.png) | ![Launcher](docs/launcher.png) | ![Tray](docs/tray.png) |

## Installation

### Download (Recommended)
Grab the latest release for your platform:
- **Windows**: `open-now-windows-x64.exe`
- **Linux**: `open-now-linux-x64`
- **macOS Intel**: `open-now-macos-x64`
- **macOS Apple Silicon**: `open-now-macos-arm64`

[Latest Release](https://github.com/USERNAME/shortcut-manager-rs/releases/latest)

### Package Managers (Planned)
```bash
# Coming soon
scoop install open-now        # Windows
brew install open-now         # macOS
pacman -S open-now            # Arch Linux
```

### From Source
```bash
# Requirements: Rust 1.75+
git clone https://github.com/USERNAME/shortcut-manager-rs
cd shortcut-manager-rs
cargo build --release
# Binary at: target/release/open-now
```

## Usage

### First Run
1. Run the binary - it minimizes to system tray
2. Click tray icon → **Show** to open main window
3. Click **➕ Add** to create your first shortcut

### Adding Shortcuts
| Field | Description | Example |
|-------|-------------|---------|
| **Name** | Display name | "VS Code" |
| **Target** | Path, URL, or command | `C:\Program Files\Microsoft VS Code\Code.exe` |
| **Arguments** | Optional args | `--new-window` |
| **Category** | Group for filtering | `Development` |
| **Hotkey** | Optional direct hotkey | `ctrl+alt+v` |

### Supported Targets
- **Executables** - `.exe`, scripts, binaries
- **Files/Folders** - Opens with default app
- **URLs** - `https://github.com`, `mailto:...`
- **Commands** - `cmd /c`, `powershell`, `bash -c`

### Keyboard Shortcuts
| Key | Action |
|-----|--------|
| `Ctrl+Alt+Space` | Toggle launcher popup |
| `Type` | Filter shortcuts |
| `↑/↓` | Navigate results |
| `Enter` | Launch selected |
| `Esc` | Close launcher |

### Main Window
- **Search** - Filter by name/category
- **Category dropdown** - Filter by category
- **Table** - Sortable by name, category, hotkey, launch count
- **Double-click** - Launch shortcut
- **Buttons** - Add, Edit, Delete, Launch

## Configuration

Data stored in:
- **Windows**: `%APPDATA%\ShortcutManager\shortcuts.json`
- **Linux**: `~/.local/share/ShortcutManager/shortcuts.json`
- **macOS**: `~/Library/Application Support/ShortcutManager/shortcuts.json`

Icon cache: `<data_dir>/icon_cache/`

## Building

### Prerequisites
- Rust 1.75+ (`rustup update`)
- **Linux**: `pkg-config libglib2.0-dev libgtk-3-dev libayatana-appindicator3-dev libxdo-dev`
- **macOS**: `pkg-config gtk+3` (via Homebrew)
- **Windows**: Visual Studio Build Tools or MSVC

### Build Commands
```bash
# Development
cargo run

# Release (optimized)
cargo build --release

# Cross-compile from Linux
cargo build --release --target x86_64-pc-windows-gnu
```

### Release Profile (in Cargo.toml)
- LTO enabled
- Strip symbols
- Optimize for size (`opt-level = "z"`)
- Single codegen unit
- Panic = abort

## Architecture

```
src/
├── main.rs       # Entry point, window config
├── app.rs        # Main UI (main window, launcher, dialogs)
├── models.rs     # Shortcut struct, ShortcutStore (JSON persistence)
├── hotkeys.rs    # Global hotkey registration & handling
├── tray.rs       # System tray icon & menu
└── utils.rs      # Cross-platform launch, icon extraction
```

### Tech Stack
- **GUI**: `eframe` + `egui` (immediate mode)
- **Hotkeys**: `global-hotkey` (TAURI)
- **Tray**: `tray-icon` + `muda`
- **Windowing**: `winit`
- **Serialization**: `serde` + `serde_json`

## Contributing

1. Fork the repo
2. Create feature branch: `git checkout -b feature/amazing`
3. Commit changes: `git commit -m 'Add amazing feature'`
4. Push: `git push origin feature/amazing`
5. Open Pull Request

## License

MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

- [egui](https://github.com/emilk/egui) - Immediate mode GUI
- [global-hotkey](https://github.com/tauri-apps/global-hotkey) - Cross-platform hotkeys
- [tray-icon](https://github.com/tauri-apps/tray-icon) - System tray support
- [rfd](https://github.com/PolyMeilex/rfd) - File dialogs