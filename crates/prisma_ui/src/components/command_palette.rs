/// Command palette - Spotlight-style quick launcher with fuzzy search
use gpui::{
    div, px, Action, Context, Entity, FocusHandle, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, Styled, Window, App, AppContext
};
use gpui_component::{
    input::{InputState, TextInput},
    modal::Modal,
    v_flex, h_flex, ActiveTheme, Icon, IconName, StyledExt
};
use serde::Deserialize;
use std::collections::HashMap;

/// Command types available in the palette
#[derive(Clone, Debug, PartialEq)]
pub enum CommandType {
    /// Launch an application
    LaunchApp,
    /// System action (shutdown, restart, etc.)
    SystemAction,
    /// Window action (close, minimize, etc.)
    WindowAction,
    /// File operation
    FileOperation,
    /// Calculator expression
    Calculator,
    /// Web search
    WebSearch,
}

/// A command that can be executed from the palette
#[derive(Clone, Debug)]
pub struct Command {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: IconName,
    pub command_type: CommandType,
    pub keywords: Vec<String>,
    pub executable: Box<dyn Fn() + Send + Sync>,
}

/// Actions for command palette
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = command_palette, no_json)]
pub enum CommandPaletteAction {
    Toggle,
    ExecuteCommand(String),
    NavigateUp,
    NavigateDown,
    ExecuteSelected,
}

/// Command palette component with fuzzy search and quick actions
pub struct CommandPalette {
    /// Whether the palette is currently open
    open: bool,
    /// Search input state
    search_input: Entity<InputState>,
    /// All available commands
    commands: HashMap<String, Command>,
    /// Filtered and ranked commands based on search
    filtered_commands: Vec<String>,
    /// Currently selected command index
    selected_index: usize,
    /// Focus handle
    focus_handle: FocusHandle,
}

impl CommandPalette {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Type a command or search...")
        });

        let mut this = Self {
            open: false,
            search_input,
            commands: HashMap::new(),
            filtered_commands: Vec::new(),
            selected_index: 0,
            focus_handle: cx.focus_handle(),
        };

        this.setup_default_commands();
        this.update_filtered_commands("");
        this
    }

    /// Create command palette entity
    pub fn create(window: &mut Window, cx: &mut gpui::App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    /// Toggle palette open/closed state
    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = !self.open;
        if self.open {
            // Reset search and selection
            self.search_input.update(cx, |input, cx| {
                input.set_value("", window, cx);
            });
            self.selected_index = 0;
            self.update_filtered_commands("");
            cx.focus_view(&self.search_input, window);
        }
        cx.notify();
    }

    /// Close the palette
    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        cx.notify();
    }

    /// Execute a command by ID
    pub fn execute_command(&mut self, command_id: &str, cx: &mut Context<Self>) {
        if let Some(command) = self.commands.get(command_id) {
            tracing::info!("Executing command: {}", command.title);
            // TODO: Execute the actual command
            self.close(cx);
        }
    }

    /// Execute the currently selected command
    pub fn execute_selected(&mut self, cx: &mut Context<Self>) {
        if let Some(command_id) = self.filtered_commands.get(self.selected_index).cloned() {
            self.execute_command(&command_id, cx);
        }
    }

    /// Navigate selection up
    pub fn navigate_up(&mut self, cx: &mut Context<Self>) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
            cx.notify();
        }
    }

    /// Navigate selection down
    pub fn navigate_down(&mut self, cx: &mut Context<Self>) {
        if self.selected_index < self.filtered_commands.len().saturating_sub(1) {
            self.selected_index += 1;
            cx.notify();
        }
    }

    /// Update search and filter commands
    pub fn search(&mut self, query: &str, cx: &mut Context<Self>) {
        self.update_filtered_commands(query);
        self.selected_index = 0;
        cx.notify();
    }

    /// Setup default system commands
    fn setup_default_commands(&mut self) {
        let commands = vec![
            Command {
                id: "app_terminal".to_string(),
                title: "Terminal".to_string(),
                subtitle: Some("Open terminal application".to_string()),
                icon: IconName::SquareTerminal,
                command_type: CommandType::LaunchApp,
                keywords: vec!["terminal", "cmd", "command", "shell"].into_iter().map(|s| s.to_string()).collect(),
                executable: Box::new(|| {
                    tracing::info!("Launching terminal");
                }),
            },
            Command {
                id: "app_files".to_string(),
                title: "File Manager".to_string(),
                subtitle: Some("Browse files and folders".to_string()),
                icon: IconName::Folder,
                command_type: CommandType::LaunchApp,
                keywords: vec!["files", "explorer", "browser", "folder"].into_iter().map(|s| s.to_string()).collect(),
                executable: Box::new(|| {
                    tracing::info!("Launching file manager");
                }),
            },
            Command {
                id: "app_settings".to_string(),
                title: "System Settings".to_string(),
                subtitle: Some("Configure system preferences".to_string()),
                icon: IconName::Settings,
                command_type: CommandType::LaunchApp,
                keywords: vec!["settings", "preferences", "config", "control"].into_iter().map(|s| s.to_string()).collect(),
                executable: Box::new(|| {
                    tracing::info!("Opening system settings");
                }),
            },
            Command {
                id: "sys_shutdown".to_string(),
                title: "Shutdown".to_string(),
                subtitle: Some("Turn off the computer".to_string()),
                icon: IconName::Settings,
                command_type: CommandType::SystemAction,
                keywords: vec!["shutdown", "power", "off", "halt"].into_iter().map(|s| s.to_string()).collect(),
                executable: Box::new(|| {
                    tracing::info!("Shutting down system");
                }),
            },
            Command {
                id: "sys_restart".to_string(),
                title: "Restart".to_string(),
                subtitle: Some("Restart the computer".to_string()),
                icon: IconName::Settings2,
                command_type: CommandType::SystemAction,
                keywords: vec!["restart", "reboot", "reset"].into_iter().map(|s| s.to_string()).collect(),
                executable: Box::new(|| {
                    tracing::info!("Restarting system");
                }),
            },
            Command {
                id: "sys_lock".to_string(),
                title: "Lock Screen".to_string(),
                subtitle: Some("Lock the current session".to_string()),
                icon: IconName::User,
                command_type: CommandType::SystemAction,
                keywords: vec!["lock", "secure", "session"].into_iter().map(|s| s.to_string()).collect(),
                executable: Box::new(|| {
                    tracing::info!("Locking screen");
                }),
            },
            Command {
                id: "calc_basic".to_string(),
                title: "Calculator".to_string(),
                subtitle: Some("Perform quick calculations".to_string()),
                icon: IconName::LayoutDashboard,
                command_type: CommandType::Calculator,
                keywords: vec!["calculator", "calc", "math", "compute"].into_iter().map(|s| s.to_string()).collect(),
                executable: Box::new(|| {
                    tracing::info!("Opening calculator");
                }),
            },
        ];

        for command in commands {
            self.commands.insert(command.id.clone(), command);
        }
    }

    /// Update filtered commands based on search query with fuzzy matching
    fn update_filtered_commands(&mut self, query: &str) {
        let query = query.to_lowercase();

        if query.is_empty() {
            // Show all commands when no query
            self.filtered_commands = self.commands.keys().cloned().collect();
            return;
        }

        // Simple fuzzy matching algorithm
        let mut scored_commands: Vec<(String, i32)> = self.commands
            .iter()
            .filter_map(|(id, command)| {
                let score = self.calculate_match_score(&query, command);
                if score > 0 {
                    Some((id.clone(), score))
                } else {
                    None
                }
            })
            .collect();

        // Sort by score (highest first)
        scored_commands.sort_by(|a, b| b.1.cmp(&a.1));

        self.filtered_commands = scored_commands
            .into_iter()
            .map(|(id, _)| id)
            .take(10) // Limit to top 10 results
            .collect();
    }

    /// Calculate match score for a command against query
    fn calculate_match_score(&self, query: &str, command: &Command) -> i32 {
        let mut score = 0;

        // Exact title match gets highest score
        if command.title.to_lowercase() == query {
            score += 1000;
        }
        // Title starts with query
        else if command.title.to_lowercase().starts_with(query) {
            score += 500;
        }
        // Title contains query
        else if command.title.to_lowercase().contains(query) {
            score += 250;
        }

        // Check keywords
        for keyword in &command.keywords {
            if keyword == query {
                score += 300;
            } else if keyword.starts_with(query) {
                score += 150;
            } else if keyword.contains(query) {
                score += 75;
            }
        }

        // Check subtitle
        if let Some(subtitle) = &command.subtitle {
            if subtitle.to_lowercase().contains(query) {
                score += 50;
            }
        }

        score
    }

    fn render_command_item(&self, command_id: &str, is_selected: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let command = &self.commands[command_id];

        div()
            .w_full()
            .p_3()
            .rounded(cx.theme().radius)
            .when(is_selected, |this| this.bg(cx.theme().accent))
            .hover(|this| this.bg(cx.theme().muted))
            .cursor_pointer()
            .on_click({
                let command_id = command_id.to_string();
                cx.listener(move |this, _, _, cx| {
                    this.execute_command(&command_id, cx);
                })
            })
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .size_10()
                            .flex()
                            .items_center()
                            .justify_center()
                            .bg(cx.theme().primary.opacity(0.1))
                            .text_color(cx.theme().primary)
                            .rounded(cx.theme().radius)
                            .child(Icon::new(command.icon).size_5())
                    )
                    .child(
                        v_flex()
                            .flex_1()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(if is_selected { cx.theme().accent_foreground } else { cx.theme().foreground })
                                    .child(&command.title)
                            )
                            .when_some(command.subtitle.as_ref(), |this, subtitle| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(if is_selected { cx.theme().accent_foreground.opacity(0.8) } else { cx.theme().muted_foreground })
                                        .child(subtitle)
                                )
                            })
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(if is_selected { cx.theme().accent_foreground.opacity(0.6) } else { cx.theme().muted_foreground })
                            .child(match command.command_type {
                                CommandType::LaunchApp => "App",
                                CommandType::SystemAction => "System",
                                CommandType::WindowAction => "Window",
                                CommandType::FileOperation => "File",
                                CommandType::Calculator => "Calc",
                                CommandType::WebSearch => "Search",
                            })
                    )
            )
    }
}

impl Focusable for CommandPalette {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for CommandPalette {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.open {
            return div(); // Hidden when closed
        }

        // Center modal overlay
        Modal::new("command-palette-modal")
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_1_2()
                    .transform_translate_x_1_2()
                    .mt(px(100.0))
                    .w(px(600.0))
                    .max_h(px(500.0))
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .shadow_xl()
                    .child(
                        v_flex()
                            .size_full()
                            .child(
                                // Search input
                                div()
                                    .p_4()
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        TextInput::new(&self.search_input)
                                            .full_width()
                                            .text_lg()
                                            .on_input(cx.listener(|this, query, _, cx| {
                                                this.search(&query, cx);
                                            }))
                                            .on_key_down(cx.listener(|this, event, _, cx| {
                                                match event.keystroke.key.as_str() {
                                                    "ArrowUp" => this.navigate_up(cx),
                                                    "ArrowDown" => this.navigate_down(cx),
                                                    "Enter" => this.execute_selected(cx),
                                                    "Escape" => this.close(cx),
                                                    _ => {}
                                                }
                                            }))
                                    )
                            )
                            .child(
                                // Command list
                                div()
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .p_2()
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .children(self.filtered_commands.iter().enumerate().map(|(index, command_id)| {
                                                self.render_command_item(command_id, index == self.selected_index, cx)
                                            }))
                                    )
                                    .when(self.filtered_commands.is_empty(), |this| {
                                        this.child(
                                            div()
                                                .p_6()
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("No commands found")
                                        )
                                    })
                            )
                    )
            )
            .on_click_outside(cx.listener(|this, _, _, cx| {
                this.close(cx);
            }))
    }
}