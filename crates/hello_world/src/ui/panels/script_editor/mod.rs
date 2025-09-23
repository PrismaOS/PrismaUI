mod file_explorer;
mod text_editor;
mod terminal;

pub use file_explorer::FileExplorer;
pub use text_editor::TextEditor;
pub use terminal::Terminal;

use std::path::PathBuf;
use gpui::{*, prelude::FluentBuilder};
use gpui_component::{
    dock::{Panel, PanelEvent},
    resizable::{h_resizable, v_resizable, resizable_panel, ResizableState},
    h_flex,
    ActiveTheme,
};

pub struct ScriptEditor {
    focus_handle: FocusHandle,
    file_explorer: Entity<FileExplorer>,
    text_editor: Entity<TextEditor>,
    terminal: Entity<Terminal>,
    horizontal_resizable_state: Entity<ResizableState>,
    vertical_resizable_state: Entity<ResizableState>,
    terminal_visible: bool,
}

impl ScriptEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let horizontal_resizable_state = ResizableState::new(cx);
        let vertical_resizable_state = ResizableState::new(cx);

        let file_explorer = cx.new(|cx| FileExplorer::new(window, cx));
        let text_editor = cx.new(|cx| TextEditor::new(window, cx));
        let terminal = cx.new(|cx| Terminal::new(window, cx));

        Self {
            focus_handle: cx.focus_handle(),
            file_explorer,
            text_editor,
            terminal,
            horizontal_resizable_state,
            vertical_resizable_state,
            terminal_visible: true,
        }
    }

    pub fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.text_editor.update(cx, |editor, cx| {
            editor.open_file(path, window, cx);
        });
    }

    pub fn get_file_explorer(&self) -> &Entity<FileExplorer> {
        &self.file_explorer
    }

    pub fn get_text_editor(&self) -> &Entity<TextEditor> {
        &self.text_editor
    }

    pub fn toggle_terminal(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.terminal_visible = !self.terminal_visible;
        cx.notify();
    }
}

impl Panel for ScriptEditor {
    fn panel_name(&self) -> &'static str {
        "Script Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div().child("Script Editor").into_any_element()
    }

    fn dump(&self, _cx: &App) -> gpui_component::dock::PanelState {
        gpui_component::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for ScriptEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for ScriptEditor {}

impl Render for ScriptEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Check for file opening requests from the file explorer
        if let Some(path) = self.file_explorer.update(cx, |explorer, _| explorer.get_last_opened_file()) {
            self.open_file(path, window, cx);
        }
        h_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                h_resizable("script-editor-horizontal", self.horizontal_resizable_state.clone())
                    .child(
                        resizable_panel()
                            .size(px(300.))
                            .size_range(px(200.)..px(500.))
                            .child(
                                div()
                                    .size_full()
                                    .bg(cx.theme().sidebar)
                                    .border_r_1()
                                    .border_color(cx.theme().border)
                                    .child(self.file_explorer.clone())
                            )
                    )
                    .child(
                        resizable_panel()
                            .child(
                                div()
                                    .size_full()
                                    .child(
                                        v_resizable("script-editor-vertical", self.vertical_resizable_state.clone())
                                            .child(
                                                resizable_panel()
                                                    .child(
                                                        div()
                                                            .size_full()
                                                            .child(self.text_editor.clone())
                                                    )
                                            )
                                            .when(self.terminal_visible, |resizable| {
                                                resizable.child(
                                                    resizable_panel()
                                                        .size(px(200.))
                                                        .size_range(px(100.)..px(400.))
                                                        .child(
                                                            div()
                                                                .size_full()
                                                                .bg(cx.theme().secondary)
                                                                .border_t_1()
                                                                .border_color(cx.theme().border)
                                                                .child(self.terminal.clone())
                                                        )
                                                )
                                            })
                                    )
                            )
                    )
            )
    }
}