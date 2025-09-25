# PrismaUI - Complete OS System UI

A modern, modular, and efficient operating system UI built with GPUI, designed as a hybrid of Windows 11 and macOS experiences with GPU-accelerated rendering.

## 🚀 Features

### Core Window Management
- **GPU-Backed Compositing**: Hardware-accelerated rendering at 60-120 FPS
- **Multiple Windows**: Resizable, draggable windows with proper Z-ordering
- **Window Operations**: Minimize, maximize, close, snap, and tile
- **Smart Snapping**: Drag windows to screen edges for automatic tiling
- **Focus Management**: Proper window focus and event routing

### System UI Components
- **App Menu**: Start menu-style application launcher with categories and search
- **Command Palette**: Spotlight-style quick launcher with fuzzy search (Ctrl+Space)
- **Taskbar**: Window switcher and system tray with live clock
- **Desktop**: Wallpaper support with multiple display modes and desktop icons
- **System Shell**: Desktop interactions, context menus, and drag-drop

### Modern Design
- **Hybrid Aesthetic**: Combines modern OS design principles
- **Smooth Animations**: GPU-accelerated window transitions and effects
- **Responsive Layouts**: Adaptive UI that scales with screen resolution
- **Theme Support**: Consistent styling with customizable themes
- **High DPI**: Optimized for high-resolution displays

## 🏗️ Architecture

### Component Hierarchy

```
Desktop (Root Component)
├── Wallpaper (Background layer)
├── WindowManager (Application windows)
│   └── ManagedWindow[] (Individual app windows)
├── SystemShell (Desktop interactions)
├── AppMenu (Application launcher)
├── CommandPalette (Quick launcher)
└── Taskbar (System navigation)
```

### Core Systems

#### Window Manager (`window_manager.rs`)
- **Compositing**: GPU-backed window composition with efficient dirty-region updates
- **Event Routing**: Mouse and keyboard event handling through window hierarchy
- **Window Operations**: Create, focus, move, resize, minimize, maximize, close
- **Snapping**: Automatic window tiling with visual snap zones
- **State Management**: Window persistence and restoration

#### Desktop Environment (`desktop.rs`)
- **Main Orchestrator**: Coordinates all UI components and system interactions
- **Event Handling**: Global keyboard shortcuts and system-wide events
- **Resolution Management**: Handles screen resolution changes and multi-monitor setup
- **Component Lifecycle**: Manages creation and destruction of UI components

#### System Components (`components/`)
- **AppMenu**: Application launcher with categorized apps and fuzzy search
- **CommandPalette**: Quick command execution with keyboard navigation
- **Taskbar**: Window switching, system tray, and live system information
- **Wallpaper**: Image display with multiple scaling modes and GPU acceleration

## 🎯 Performance Optimizations

### GPU Acceleration
- **GPUI Framework**: All rendering happens on GPU with minimal CPU overhead
- **Batched Updates**: UI changes are batched and processed efficiently
- **Dirty Regions**: Only changed areas are re-rendered
- **Texture Caching**: Images and UI elements are cached as GPU textures

### Memory Management
- **Smart Entity System**: GPUI's entity system ensures efficient memory usage
- **Event Subscriptions**: Automatic cleanup of event listeners
- **Asset Management**: Efficient loading and caching of images and resources

### 60-120 FPS Target
- **Optimized Rendering**: GPU-based composition maintains high frame rates
- **Minimal Allocations**: Reduced garbage collection pressure
- **Efficient Layouts**: CPU handles layout, GPU handles all drawing

## 🔧 Usage

### Basic Setup

```rust
use gpui::Application;
use prisma_ui::{Assets, Desktop, init};

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        init(cx); // Initialize PrismaUI
        cx.activate(true);

        // Create main OS window
        let window = cx.open_window(options, |window, cx| {
            let desktop = cx.new(|cx| Desktop::new(window, cx));
            cx.new(|cx| Root::new(desktop.into(), window, cx))
        });
    });
}
```

### Creating Application Windows

```rust
// In your desktop component
let window_id = self.create_app_window(
    "My Application".to_string(),
    content_entity,
    Some(bounds),
    window,
    cx,
);
```

### Keyboard Shortcuts
- **Ctrl+Space**: Open command palette
- **Cmd+D**: Create demo window (for testing)
- **Alt+Tab**: Window switcher (planned)
- **Alt+F4**: Close focused window (planned)

## 🎨 Component Usage Examples

### App Menu Integration

```rust
// Add custom application to app menu
app_menu.update(cx, |menu, cx| {
    menu.add_app(AppEntry {
        id: "my_app".to_string(),
        name: "My Application".to_string(),
        description: "Custom application".to_string(),
        icon: IconName::Settings,
        category: "Productivity".to_string(),
        pinned: true,
        recently_used: false,
    });
});
```

### Command Palette Commands

```rust
// Add custom command to palette
command_palette.update(cx, |palette, cx| {
    palette.add_command(Command {
        id: "custom_action".to_string(),
        title: "Custom Action".to_string(),
        subtitle: Some("Perform custom operation".to_string()),
        icon: IconName::Star,
        command_type: CommandType::SystemAction,
        keywords: vec!["custom", "action"],
        executable: Box::new(|| {
            // Custom action implementation
        }),
    });
});
```

### Wallpaper Configuration

```rust
// Set custom wallpaper
desktop.set_wallpaper(
    Some("/path/to/wallpaper.jpg".to_string()),
    cx
);
```

## 🔌 Extensibility

### Custom Components
All components follow GPUI patterns and can be easily extended:

```rust
impl Render for MyCustomComponent {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Use gpui-component widgets for consistent styling
        v_flex()
            .size_full()
            .child(Button::new("my-button").primary().label("Click Me"))
    }
}
```

### Plugin System (Planned)
Future versions will support:
- **App Plugins**: Register custom applications
- **Widget Plugins**: Add new UI components
- **Theme Plugins**: Custom color schemes and styling
- **Effect Plugins**: GPU-accelerated visual effects

## 🏃‍♂️ Running the Demo

```bash
# Build and run PrismaUI
cd crates/prisma_ui
cargo run

# Or run with specific features
cargo run --features "webview,tree-sitter-languages"
```

## 📁 Project Structure

```
crates/prisma_ui/
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Public API exports
│   ├── desktop.rs           # Main desktop environment
│   ├── window_manager.rs    # Window management system
│   ├── shell.rs            # System shell interactions
│   ├── assets.rs           # Asset loading and management
│   └── components/         # UI components
│       ├── mod.rs
│       ├── app_menu.rs     # Application launcher
│       ├── command_palette.rs # Quick command system
│       ├── taskbar.rs      # System navigation bar
│       └── wallpaper.rs    # Desktop background
├── assets/                 # Images, icons, and resources
├── Cargo.toml             # Dependencies and metadata
└── README.md              # This file
```

## 🎯 Roadmap

### Phase 1: Core Functionality ✅
- [x] Window management with GPU acceleration
- [x] Basic system UI components
- [x] App menu and command palette
- [x] Taskbar and system tray
- [x] Wallpaper support

### Phase 2: Enhanced Features 🚧
- [ ] Window animations and effects
- [ ] Multi-monitor support
- [ ] Advanced window snapping
- [ ] System settings panel
- [ ] Notification system

### Phase 3: Advanced Features 📋
- [ ] Plugin system
- [ ] Custom themes
- [ ] Accessibility features
- [ ] Performance profiling tools
- [ ] WebView integration

## 🤝 Contributing

PrismaUI is designed to be modular and extensible. Key areas for contribution:

1. **Performance**: GPU optimization and rendering improvements
2. **Features**: New system UI components and interactions
3. **Platforms**: Cross-platform compatibility improvements
4. **Documentation**: Examples and usage guides
5. **Testing**: Automated testing and benchmarks

## 📄 License

Licensed under Apache License 2.0. See the project root for license details.

## 🙏 Acknowledgments

- **GPUI Framework**: Built on Zed's GPUI for high-performance UI rendering
- **Component Library**: Leverages the comprehensive `crates/ui` component system
- **Design Inspiration**: Windows 11 and macOS for modern OS UI patterns

---

*PrismaUI represents a complete, modern operating system interface designed for performance, modularity, and extensibility. Built with Rust and GPUI for the next generation of system software.*