use gpui::prelude::*;
use gpui::{
    div, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable, Render,
    StyleRefinement, Styled, Window, ParentElement, EntityInputHandler
};
use gpui_component::{
    tab::{TabBar, Tab},
    input::{TextInput, InputState},
    button::Button,
    Icon, IconName, Size, Sizable,
    h_flex, v_flex, ActiveTheme
};
use gpui_webview::{WebView, events::*};

use crate::tab_manager::TabManager;

pub struct BrowserWindow {
    tab_manager: TabManager,
    url_input: Entity<InputState>,
    focus_handle: FocusHandle,
    style: StyleRefinement,
}

impl BrowserWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let url_input = cx.new(|cx| {
            InputState::new(window, cx).placeholder("Enter URL or search term...".to_string())
        });
        let focus_handle = cx.focus_handle();

        let mut browser = Self {
            tab_manager: TabManager::new(),
            url_input,
            focus_handle,
            style: StyleRefinement::default(),
        };

        // Set up URL input handlers
        cx.subscribe(&url_input, Self::on_url_input_event).detach();

        // Create initial tab
        browser.create_new_tab("https://www.google.com", window, cx);

        browser
    }

    fn create_new_tab(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        let webview = WebView::new(url, window, cx);

        // Subscribe to webview events
        cx.subscribe(&webview, Self::on_webview_event).detach();

        let tab_id = self.tab_manager.create_tab(url.to_string(), webview);

        // Set the URL in the input field if this is the active tab
        if Some(tab_id) == self.tab_manager.get_active_tab_id() {
            self.update_url_input(url, window, cx);
        }

        cx.notify();
    }

    fn on_webview_event(
        &mut self,
        webview: Entity<WebView>,
        event: &dyn std::any::Any,
        cx: &mut Context<Self>,
    ) {
        // Find the tab that owns this webview
        let tab_id = self.tab_manager.get_all_tabs()
            .iter()
            .find(|tab| tab.webview == webview)
            .map(|tab| tab.id);

        if let Some(tab_id) = tab_id {
            if let Some(event) = event.downcast_ref::<AddressChangedEvent>() {
                self.tab_manager.update_tab_url(tab_id, event.url.clone());
                // URL will be updated in render cycle
            } else if let Some(event) = event.downcast_ref::<TitleChangedEvent>() {
                self.tab_manager.update_tab_title(tab_id, event.title.clone());
            } else if let Some(_) = event.downcast_ref::<LoadStartEvent>() {
                self.tab_manager.set_tab_loading(tab_id, true);
            } else if let Some(_) = event.downcast_ref::<LoadEndEvent>() {
                self.tab_manager.set_tab_loading(tab_id, false);
            }
            cx.notify();
        }
    }

    fn update_url_input(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.url_input.update(cx, |input, cx| {
            input.replace_text_in_range(None, url, window, cx);
        });
    }

    fn on_url_input_event(
        &mut self,
        _input: Entity<InputState>,
        event: &gpui_component::input::InputEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            gpui_component::input::InputEvent::PressEnter { .. } => {
                self.on_url_submit_action(cx);
            }
            _ => {}
        }
    }

    fn on_url_submit_action(&mut self, cx: &mut Context<Self>) {
        let url = self.url_input.read(cx).value().to_string();
        if !url.is_empty() {
            let formatted_url = if url.starts_with("http://") || url.starts_with("https://") {
                url
            } else if url.contains('.') && !url.contains(' ') {
                format!("https://{}", url)
            } else {
                format!("https://www.google.com/search?q={}", urlencoding::encode(&url))
            };

            if let Some(active_tab) = self.tab_manager.get_active_tab() {
                active_tab.webview.update(cx, |webview, _cx| {
                    webview.browser().load_url(&formatted_url);
                });
            }
        }
    }

    fn on_tab_click(&mut self, tab_index: &usize, window: &mut Window, cx: &mut Context<Self>) {
        let tabs: Vec<_> = self.tab_manager.get_all_tabs().iter().map(|tab| tab.id).collect();
        if let Some(&tab_id) = tabs.get(*tab_index) {
            self.tab_manager.set_active_tab(tab_id);
            if let Some(tab) = self.tab_manager.get_active_tab() {
                self.update_url_input(&tab.url, window, cx);
            }
            cx.notify();
        }
    }


    fn create_new_tab_action(&mut self, _event: &gpui::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.create_new_tab("https://www.google.com", window, cx);
    }

    fn close_current_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_tab_id) = self.tab_manager.get_active_tab_id() {
            self.tab_manager.close_tab(active_tab_id);

            // Update URL input for new active tab
            if let Some(tab) = self.tab_manager.get_active_tab() {
                self.update_url_input(&tab.url, window, cx);
            }

            cx.notify();
        }
    }

    fn close_tab_by_id(&mut self, tab_id: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.tab_manager.close_tab(tab_id);

        // Update URL input for new active tab
        if let Some(tab) = self.tab_manager.get_active_tab() {
            self.update_url_input(&tab.url, window, cx);
        }

        cx.notify();
    }
}

impl Styled for BrowserWindow {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Focusable for BrowserWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for BrowserWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tabs: Vec<Tab> = self.tab_manager
            .get_all_tabs()
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let tab_id = tab.id;
                Tab::new(&tab.title)
                    .id(("tab", tab_id))
                    .when(tab.is_loading, |this| this.prefix(Icon::new(IconName::Loader)))
                    .suffix(
                        gpui_component::button::Button::new(("close-tab", tab_id))
                            .icon(IconName::Close)
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(move |this: &mut BrowserWindow, _, _, window, cx| {
                                this.close_tab_by_id(tab_id, window, cx);
                            }))
                    )
            })
            .collect();

        let active_tab_index = self.tab_manager.get_all_tabs()
            .iter()
            .position(|tab| Some(tab.id) == self.tab_manager.get_active_tab_id())
            .unwrap_or(0);

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Tab bar with URL bar
                v_flex()
                    .w_full()
                    .child(
                        TabBar::new("browser-tabs")
                            .underline()
                            .with_size(Size::Medium)
                            .children(tabs)
                            .selected_index(active_tab_index)
                            .on_click(cx.listener(Self::on_tab_click))
                            .suffix(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .cursor_pointer()
                                            .p_1()
                                            .rounded_md()
                                            .hover(|style| style.bg(cx.theme().ghost_element_hover))
                                            .child("+ New Tab")
                                            .on_mouse_down(gpui::MouseButton::Left, cx.listener(Self::create_new_tab_action))
                                    )
                            )
                    )
                    .child(
                        // URL bar
                        h_flex()
                            .w_full()
                            .p_2()
                            .gap_2()
                            .child(
                                TextInput::new(&self.url_input)
                                    .flex_1()
                            )
                    )
            )
            .child(
                // Web content area
                div()
                    .flex_1()
                    .w_full()
                    .when_some(self.tab_manager.get_active_tab(), |this, tab| {
                        this.child(div().size_full().child(tab.webview.clone()))
                    })
            )
    }
}

impl EventEmitter<()> for BrowserWindow {}