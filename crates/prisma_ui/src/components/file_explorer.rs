use gpui::{
    div, img, px, Action, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, Styled, Window, AppContext
};
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::{Button, ButtonVariants as _}, h_flex, v_flex, input::{InputEvent, InputState, TextInput},
    ActiveTheme, Icon, IconName, StyledExt, Selectable, Disableable
};
use serde::Deserialize;
use std::path::PathBuf;

/// File system item types
#[derive(Clone, Debug, PartialEq)]
pub enum FileItemType {
    File,
    Directory,
    SymLink,
    Image,
    Video,
    Audio,
    Document,
    Archive,
    Application,
}

/// File system item
#[derive(Clone, Debug)]
pub struct FileItem {
    pub name: String,
    pub path: PathBuf,
    pub item_type: FileItemType,
    pub size: Option<u64>,
    pub modified: Option<std::time::SystemTime>,
    pub permissions: Option<String>,
    pub is_hidden: bool,
}

impl FileItem {
    pub fn icon(&self) -> IconName {
        match self.item_type {
            FileItemType::Directory => IconName::Folder,
            FileItemType::File => IconName::BookOpen,
            FileItemType::Image => IconName::Frame,
            FileItemType::Video => IconName::Frame,
            FileItemType::Audio => IconName::Heart,
            FileItemType::Document => IconName::BookOpen,
            FileItemType::Archive => IconName::Folder,
            FileItemType::Application => IconName::Bot,
            FileItemType::SymLink => IconName::ExternalLink,
        }
    }

    pub fn icon_image(&self) -> Option<&str> {
        match self.item_type {
            FileItemType::Directory => Some("icons/folder.png"),
            FileItemType::File => Some("icons/file.png"),
            FileItemType::Image => Some("icons/image.png"),
            FileItemType::Video => Some("icons/video.png"),
            FileItemType::Audio => Some("icons/audio.png"),
            FileItemType::Document => Some("icons/document.png"),
            FileItemType::Archive => Some("icons/archive.png"),
            FileItemType::Application => Some("icons/application.png"),
            FileItemType::SymLink => Some("icons/link.png"),
        }
    }
}

/// Sidebar locations
#[derive(Clone, Debug, PartialEq)]
pub enum SidebarLocation {
    Favorites,
    Recent,
    Applications,
    Desktop,
    Documents,
    Downloads,
    Pictures,
    Music,
    Videos,
    Home,
    Root,
    Network,
    Trash,
    Custom(String),
}

impl SidebarLocation {
    pub fn display_name(&self) -> &str {
        match self {
            Self::Favorites => "Favorites",
            Self::Recent => "Recent",
            Self::Applications => "Applications",
            Self::Desktop => "Desktop",
            Self::Documents => "Documents",
            Self::Downloads => "Downloads",
            Self::Pictures => "Pictures",
            Self::Music => "Music",
            Self::Videos => "Videos",
            Self::Home => "Home",
            Self::Root => "Computer",
            Self::Network => "Network",
            Self::Trash => "Trash",
            Self::Custom(name) => name,
        }
    }

    pub fn icon(&self) -> IconName {
        match self {
            Self::Favorites => IconName::Star,
            Self::Recent => IconName::Calendar,
            Self::Applications => IconName::Bot,
            Self::Desktop => IconName::LayoutDashboard,
            Self::Documents => IconName::BookOpen,
            Self::Downloads => IconName::ArrowDown,
            Self::Pictures => IconName::Frame,
            Self::Music => IconName::Heart,
            Self::Videos => IconName::Frame,
            Self::Home => IconName::User,
            Self::Root => IconName::Folder,
            Self::Network => IconName::Globe,
            Self::Trash => IconName::Delete,
            Self::Custom(_) => IconName::Folder,
        }
    }

    pub fn path(&self) -> PathBuf {
        match self {
            Self::Desktop => PathBuf::from("C:\\Users\\Public\\Desktop"),
            Self::Documents => PathBuf::from("C:\\Users\\Documents"),
            Self::Downloads => PathBuf::from("C:\\Users\\Downloads"),
            Self::Pictures => PathBuf::from("C:\\Users\\Pictures"),
            Self::Music => PathBuf::from("C:\\Users\\Music"),
            Self::Videos => PathBuf::from("C:\\Users\\Videos"),
            Self::Home => PathBuf::from("C:\\Users"),
            Self::Root => PathBuf::from("C:\\"),
            Self::Applications => PathBuf::from("C:\\Program Files"),
            _ => PathBuf::from("C:\\"),
        }
    }
}

/// View modes for the file explorer
#[derive(Clone, Debug, PartialEq)]
pub enum ViewMode {
    Icons,
    List,
    Columns,
}

/// File Explorer actions
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = file_explorer, no_json)]
pub enum FileExplorerAction {
    NavigateTo(String),
    NavigateBack,
    NavigateForward,
    NavigateUp,
    SelectItem(String),
    OpenItem(String),
    NewFolder,
    DeleteSelected,
    RenameSelected,
    CopySelected,
    CutSelected,
    Paste,
    ShowInfo,
    ToggleViewMode,
    Search(String),
}

/// File Explorer component - modern macOS Finder-like interface
pub struct FileExplorer {
    /// Current directory path
    current_path: PathBuf,
    /// Navigation history for back/forward
    history: Vec<PathBuf>,
    /// Current position in history
    history_index: usize,
    /// Current directory contents
    items: Vec<FileItem>,
    /// Selected items
    selected_items: Vec<String>,
    /// Current view mode
    view_mode: ViewMode,
    /// Search input state
    search_input: Entity<InputState>,
    /// Show hidden files
    show_hidden: bool,
    /// Sidebar locations
    sidebar_locations: Vec<SidebarLocation>,
    /// Active sidebar location
    active_location: Option<SidebarLocation>,
    /// Clipboard for copy/cut operations
    clipboard: Vec<FileItem>,
    /// Whether clipboard items are cut (vs copied)
    clipboard_cut: bool,
    /// Focus handle
    focus_handle: FocusHandle,
    /// Loading state
    loading: bool,
}

impl FileExplorer {
    pub fn new_simple() -> Self {
        // Create a very simple version for testing
        let current_path = PathBuf::from("C:\\Users");
        Self {
            current_path: current_path.clone(),
            history: vec![current_path],
            history_index: 0,
            items: vec![
                FileItem {
                    name: "Documents".to_string(),
                    path: PathBuf::from("C:\\Users\\Documents"),
                    item_type: FileItemType::Directory,
                    size: None,
                    modified: Some(std::time::SystemTime::now()),
                    permissions: Some("rwx".to_string()),
                    is_hidden: false,
                },
                FileItem {
                    name: "Downloads".to_string(),
                    path: PathBuf::from("C:\\Users\\Downloads"),
                    item_type: FileItemType::Directory,
                    size: None,
                    modified: Some(std::time::SystemTime::now()),
                    permissions: Some("rwx".to_string()),
                    is_hidden: false,
                },
            ],
            selected_items: Vec::new(),
            view_mode: ViewMode::Icons,
            search_input: unsafe { std::mem::zeroed() }, // Temporary - won't use this field in simple render
            show_hidden: false,
            sidebar_locations: vec![
                SidebarLocation::Home,
                SidebarLocation::Documents,
                SidebarLocation::Downloads,
            ],
            active_location: Some(SidebarLocation::Home),
            clipboard: Vec::new(),
            clipboard_cut: false,
            focus_handle: unsafe { std::mem::zeroed() }, // Temporary - won't use this field in simple render
            loading: false,
        }
    }

    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search files...")
        });

        // Subscribe to search input changes
        cx.subscribe(&search_input, |this, _, event, cx| {
            match event {
                InputEvent::Change => {
                    let query = this.search_input.read(cx).value();
                    this.search(&query, cx);
                }
                _ => {}
            }
        }).detach();

        let sidebar_locations = vec![
            SidebarLocation::Favorites,
            SidebarLocation::Recent,
            SidebarLocation::Desktop,
            SidebarLocation::Documents,
            SidebarLocation::Downloads,
            SidebarLocation::Pictures,
            SidebarLocation::Music,
            SidebarLocation::Videos,
            SidebarLocation::Home,
            SidebarLocation::Applications,
            SidebarLocation::Root,
            SidebarLocation::Network,
            SidebarLocation::Trash,
        ];

        let current_path = PathBuf::from("C:\\Users");
        let mut explorer = Self {
            current_path: current_path.clone(),
            history: vec![current_path.clone()],
            history_index: 0,
            items: Vec::new(),
            selected_items: Vec::new(),
            view_mode: ViewMode::Icons,
            search_input,
            show_hidden: false,
            sidebar_locations,
            active_location: Some(SidebarLocation::Home),
            clipboard: Vec::new(),
            clipboard_cut: false,
            focus_handle: cx.focus_handle(),
            loading: false,
        };

        explorer.load_directory(cx);
        explorer
    }

    /// Create file explorer entity
    pub fn create(window: &mut Window, cx: &mut gpui::App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    /// Navigate to a specific path
    pub fn navigate_to(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.current_path == path {
            return;
        }

        // Add to history if not navigating through history
        if self.history_index == self.history.len() - 1 {
            self.history.push(path.clone());
            self.history_index = self.history.len() - 1;
        } else {
            // Replace future history
            self.history.truncate(self.history_index + 1);
            self.history.push(path.clone());
            self.history_index = self.history.len() - 1;
        }

        self.current_path = path;
        self.selected_items.clear();
        self.load_directory(cx);
    }

    /// Navigate back in history
    pub fn navigate_back(&mut self, cx: &mut Context<Self>) {
        if self.history_index > 0 {
            self.history_index -= 1;
            self.current_path = self.history[self.history_index].clone();
            self.selected_items.clear();
            self.load_directory(cx);
        }
    }

    /// Navigate forward in history
    pub fn navigate_forward(&mut self, cx: &mut Context<Self>) {
        if self.history_index < self.history.len() - 1 {
            self.history_index += 1;
            self.current_path = self.history[self.history_index].clone();
            self.selected_items.clear();
            self.load_directory(cx);
        }
    }

    /// Navigate up one directory
    pub fn navigate_up(&mut self, cx: &mut Context<Self>) {
        if let Some(parent) = self.current_path.parent() {
            self.navigate_to(parent.to_path_buf(), cx);
        }
    }

    /// Load directory contents
    fn load_directory(&mut self, cx: &mut Context<Self>) {
        self.loading = true;

        // Mock directory contents for demonstration
        let mock_items = match self.current_path.to_string_lossy().as_ref() {
            "C:\\Users" => vec![
                FileItem {
                    name: "Documents".to_string(),
                    path: PathBuf::from("C:\\Users\\Documents"),
                    item_type: FileItemType::Directory,
                    size: None,
                    modified: Some(std::time::SystemTime::now()),
                    permissions: Some("rwx".to_string()),
                    is_hidden: false,
                },
                FileItem {
                    name: "Downloads".to_string(),
                    path: PathBuf::from("C:\\Users\\Downloads"),
                    item_type: FileItemType::Directory,
                    size: None,
                    modified: Some(std::time::SystemTime::now()),
                    permissions: Some("rwx".to_string()),
                    is_hidden: false,
                },
                FileItem {
                    name: "Pictures".to_string(),
                    path: PathBuf::from("C:\\Users\\Pictures"),
                    item_type: FileItemType::Directory,
                    size: None,
                    modified: Some(std::time::SystemTime::now()),
                    permissions: Some("rwx".to_string()),
                    is_hidden: false,
                },
                FileItem {
                    name: "Music".to_string(),
                    path: PathBuf::from("C:\\Users\\Music"),
                    item_type: FileItemType::Directory,
                    size: None,
                    modified: Some(std::time::SystemTime::now()),
                    permissions: Some("rwx".to_string()),
                    is_hidden: false,
                },
                FileItem {
                    name: "Videos".to_string(),
                    path: PathBuf::from("C:\\Users\\Videos"),
                    item_type: FileItemType::Directory,
                    size: None,
                    modified: Some(std::time::SystemTime::now()),
                    permissions: Some("rwx".to_string()),
                    is_hidden: false,
                },
            ],
            "C:\\Users\\Documents" => vec![
                FileItem {
                    name: "Report.docx".to_string(),
                    path: PathBuf::from("C:\\Users\\Documents\\Report.docx"),
                    item_type: FileItemType::Document,
                    size: Some(2048),
                    modified: Some(std::time::SystemTime::now()),
                    permissions: Some("rw-".to_string()),
                    is_hidden: false,
                },
                FileItem {
                    name: "Presentation.pptx".to_string(),
                    path: PathBuf::from("C:\\Users\\Documents\\Presentation.pptx"),
                    item_type: FileItemType::Document,
                    size: Some(5120),
                    modified: Some(std::time::SystemTime::now()),
                    permissions: Some("rw-".to_string()),
                    is_hidden: false,
                },
                FileItem {
                    name: "Projects".to_string(),
                    path: PathBuf::from("C:\\Users\\Documents\\Projects"),
                    item_type: FileItemType::Directory,
                    size: None,
                    modified: Some(std::time::SystemTime::now()),
                    permissions: Some("rwx".to_string()),
                    is_hidden: false,
                },
            ],
            _ => vec![
                FileItem {
                    name: "Empty Folder".to_string(),
                    path: self.current_path.join("Empty Folder"),
                    item_type: FileItemType::Directory,
                    size: None,
                    modified: Some(std::time::SystemTime::now()),
                    permissions: Some("rwx".to_string()),
                    is_hidden: false,
                },
            ],
        };

        self.items = mock_items;
        self.loading = false;
        cx.notify();
    }

    /// Toggle view mode
    pub fn toggle_view_mode(&mut self, cx: &mut Context<Self>) {
        self.view_mode = match self.view_mode {
            ViewMode::Icons => ViewMode::List,
            ViewMode::List => ViewMode::Columns,
            ViewMode::Columns => ViewMode::Icons,
        };
        cx.notify();
    }

    /// Search files
    pub fn search(&mut self, query: &str, _cx: &mut Context<Self>) {
        // For demo purposes, just filter current items
        // In a real implementation, this would perform a file system search
        tracing::info!("Searching for: {}", query);
    }

    /// Select an item
    pub fn select_item(&mut self, item_name: &str, cx: &mut Context<Self>) {
        if !self.selected_items.contains(&item_name.to_string()) {
            self.selected_items.clear();
            self.selected_items.push(item_name.to_string());
            cx.notify();
        }
    }

    /// Open an item (double-click)
    pub fn open_item(&mut self, item_name: &str, cx: &mut Context<Self>) {
        if let Some(item) = self.items.iter().find(|i| i.name == item_name) {
            match item.item_type {
                FileItemType::Directory => {
                    self.navigate_to(item.path.clone(), cx);
                }
                _ => {
                    // Emit event to open file with default application
                    cx.emit(FileExplorerAction::OpenItem(item.path.to_string_lossy().to_string()));
                }
            }
        }
    }

    /// Check if we can navigate back
    pub fn can_navigate_back(&self) -> bool {
        self.history_index > 0
    }

    /// Check if we can navigate forward
    pub fn can_navigate_forward(&self) -> bool {
        self.history_index < self.history.len() - 1
    }

    /// Get breadcrumb path segments
    pub fn breadcrumbs(&self) -> Vec<(String, PathBuf)> {
        let mut crumbs = Vec::new();
        let mut path = PathBuf::new();

        for component in self.current_path.components() {
            path.push(component);
            crumbs.push((
                component.as_os_str().to_string_lossy().to_string(),
                path.clone()
            ));
        }

        crumbs
    }

    /// Format file size
    fn format_size(size: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = size as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", size as u64, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .h(px(44.0))
            .w_full()
            .items_center()
            .px_4()
            .gap_2()
            .bg(cx.theme().sidebar)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Button::new("back")
                            .ghost()
                            .size(px(32.0))
                            .child(img("icons/arrow-left.png").w_4().h_4())
                            .disabled(!self.can_navigate_back())
                            .tooltip("Back")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.navigate_back(cx);
                            }))
                    )
                    .child(
                        Button::new("forward")
                            .ghost()
                            .size(px(32.0))
                            .child(img("icons/arrow-right.png").w_4().h_4())
                            .disabled(!self.can_navigate_forward())
                            .tooltip("Forward")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.navigate_forward(cx);
                            }))
                    )
                    .child(
                        Button::new("up")
                            .ghost()
                            .size(px(32.0))
                            .child(img("icons/arrow-up.png").w_4().h_4())
                            .tooltip("Up")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.navigate_up(cx);
                            }))
                    )
            )
            .child(
                // Breadcrumbs
                h_flex()
                    .flex_1()
                    .items_center()
                    .gap_1()
                    .px_3()
                    .py_1()
                    .bg(cx.theme().input)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(6.0))
                    .children(self.breadcrumbs().into_iter().enumerate().map(|(idx, (name, path))| {
                        let path_clone = path.clone();
                        h_flex()
                            .items_center()
                            .child(if idx > 0 {
                                div()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(" › ")
                                    .into_any_element()
                            } else {
                                div().into_any_element()
                            })
                            .child(
                                Button::new(("breadcrumb", idx))
                                    .ghost()
                                    .compact()
                                    .child(name)
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.navigate_to(path_clone.clone(), cx);
                                    }))
                            )
                    }))
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        TextInput::new(&self.search_input)
                            .w(px(200.0))
                            .h(px(32.0))
                    )
                    .child(
                        Button::new("view_mode")
                            .ghost()
                            .size(px(32.0))
                            .child(match self.view_mode {
                                ViewMode::Icons => img("icons/grid.png").w_4().h_4(),
                                ViewMode::List => img("icons/list.png").w_4().h_4(),
                                ViewMode::Columns => img("icons/columns.png").w_4().h_4(),
                            })
                            .tooltip("Toggle View Mode")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_view_mode(cx);
                            }))
                    )
            )
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(200.0))
            .h_full()
            .bg(cx.theme().sidebar.opacity(0.8))
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                // Favorites section
                v_flex()
                    .p_3()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .mb_2()
                            .child("FAVORITES")
                    )
                    .children(self.sidebar_locations.iter().take(4).enumerate().map(|(idx, location)| {
                        let is_active = self.active_location.as_ref() == Some(location);
                        Button::new(("sidebar_fav", idx))
                            .w_full()
                            .ghost()
                            .compact()
                            .justify_start()
                            .px_2()
                            .when(is_active, |btn| btn.selected(true))
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(match location {
                                        SidebarLocation::Favorites => img("icons/star.png").w_4().h_4().into_any_element(),
                                        SidebarLocation::Recent => img("icons/clock.png").w_4().h_4().into_any_element(),
                                        _ => Icon::new(location.icon()).size_4().into_any_element(),
                                    })
                                    .child(
                                        div()
                                            .text_sm()
                                            .child(location.display_name().to_string())
                                    )
                            )
                            .on_click({
                                let location = location.clone();
                                cx.listener(move |this, _, _, cx| {
                                    this.active_location = Some(location.clone());
                                    this.navigate_to(location.path(), cx);
                                })
                            })
                    }))
            )
            .child(
                // Locations section
                v_flex()
                    .p_3()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .mb_2()
                            .child("LOCATIONS")
                    )
                    .children(self.sidebar_locations.iter().skip(4).enumerate().map(|(idx, location)| {
                        let is_active = self.active_location.as_ref() == Some(location);
                        Button::new(("sidebar_loc", idx))
                            .w_full()
                            .ghost()
                            .compact()
                            .justify_start()
                            .px_2()
                            .when(is_active, |btn| btn.selected(true))
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(Icon::new(location.icon()).size_4().text_color(
                                        if is_active { cx.theme().primary } else { cx.theme().muted_foreground }
                                    ))
                                    .child(
                                        div()
                                            .text_sm()
                                            .child(location.display_name().to_string())
                                    )
                            )
                            .on_click({
                                let location = location.clone();
                                cx.listener(move |this, _, _, cx| {
                                    this.active_location = Some(location.clone());
                                    this.navigate_to(location.path(), cx);
                                })
                            })
                    }))
            )
    }

    fn render_main_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.view_mode {
            ViewMode::Icons => self.render_icon_view(cx).into_any_element(),
            ViewMode::List => self.render_list_view(cx).into_any_element(),
            ViewMode::Columns => self.render_column_view(cx).into_any_element(),
        }.into_any_element()
    }

    fn render_icon_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .p_4()
            .scrollable(gpui::Axis::Vertical)
            .child(
                div()
                    .w_full()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap_4()
                    .items_start()
                    .justify_start()
                    .children(self.items.iter().enumerate().map(|(idx, item)| {
                        let is_selected = self.selected_items.contains(&item.name);
                        Button::new(("item", idx))
                            .ghost()
                            .w(px(100.0))
                            .h(px(120.0))
                            .p_2()
                            .rounded(cx.theme().radius)
                            .when(is_selected, |btn| btn.selected(true))
                            .child(
                                v_flex()
                                    .size_full()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .size(px(64.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(if let Some(icon_path) = item.icon_image() {
                                                img(icon_path).w_12().h_12().into_any_element()
                                            } else {
                                                Icon::new(item.icon())
                                                    .size(px(48.0))
                                                    .text_color(match item.item_type {
                                                        FileItemType::Directory => cx.theme().primary,
                                                        _ => cx.theme().foreground,
                                                    })
                                                    .into_any_element()
                                            })
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_center()
                                            .w_full()
                                            .line_clamp(2)
                                            .child(item.name.clone())
                                    )
                            )
                            .on_click({
                                let name = item.name.clone();
                                cx.listener(move |this, _, _, cx| {
                                    this.open_item(&name, cx);
                                })
                            })
                    }))
            )
    }

    fn render_list_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_1()
            .child(
                // Header
                h_flex()
                    .h(px(32.0))
                    .w_full()
                    .items_center()
                    .px_4()
                    .bg(cx.theme().sidebar.opacity(0.5))
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().muted_foreground)
                    .child(div().w(px(300.0)).child("Name"))
                    .child(div().w(px(100.0)).child("Size"))
                    .child(div().w(px(150.0)).child("Modified"))
            )
            .child(
                div()
                    .flex_1()
                    .scrollable(gpui::Axis::Vertical)
                    .children(self.items.iter().enumerate().map(|(idx, item)| {
                        let is_selected = self.selected_items.contains(&item.name);
                        Button::new(("list_item", idx))
                            .w_full()
                            .ghost()
                            .h(px(28.0))
                            .px_4()
                            .justify_start()
                            .when(is_selected, |btn| btn.selected(true))
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .child(
                                        h_flex()
                                            .w(px(300.0))
                                            .items_center()
                                            .gap_2()
                                            .child(if let Some(icon_path) = item.icon_image() {
                                                img(icon_path).w_4().h_4().into_any_element()
                                            } else {
                                                Icon::new(item.icon()).size_4().into_any_element()
                                            })
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .child(item.name.clone())
                                            )
                                    )
                                    .child(
                                        div()
                                            .w(px(100.0))
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(item.size.map(Self::format_size).unwrap_or("—".to_string()))
                                    )
                                    .child(
                                        div()
                                            .w(px(150.0))
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Today") // Simplified for demo
                                    )
                            )
                            .on_click({
                                let name = item.name.clone();
                                cx.listener(move |this, _, _, cx| {
                                    this.open_item(&name, cx);
                                })
                            })
                    }))
            )
    }

    fn render_column_view(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Column view (simplified for demo)
        div()
            .flex_1()
            .p_4()
            .child(
                div()
                    .text_center()
                    .text_color(cx.theme().muted_foreground)
                    .child("Column view coming soon...")
            )
    }
}

impl EventEmitter<FileExplorerAction> for FileExplorer {}

impl Focusable for FileExplorer {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FileExplorer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(cx))
            .child(
                h_flex()
                    .flex_1()
                    .child(self.render_sidebar(cx))
                    .child(self.render_main_view(cx))
            )
    }
}