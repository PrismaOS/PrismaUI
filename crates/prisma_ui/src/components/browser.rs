/// Web browser component for PrismaUI
use gpui::{
    div, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement, Render, Styled, Window, EntityInputHandler
};
use gpui::prelude::FluentBuilder;
use gpui_component::{
    tab::{TabBar, Tab},
    input::{InputState, TextInput, InputEvent},
    button::{Button, ButtonVariants as _},
    Icon, IconName, Size, Sizable,
    h_flex, v_flex, ActiveTheme
};
use gpui_webview::{WebView, events::*};
use std::collections::HashMap;

pub struct BrowserTab {
    pub id: usize,
    pub url: String,
    pub title: String,
    pub webview: Entity<WebView>,
    pub is_loading: bool,
}

impl BrowserTab {
    pub fn new(id: usize, url: String, webview: Entity<WebView>) -> Self {
        Self {
            id,
            url: url.clone(),
            title: url,
            webview,
            is_loading: true,
        }
    }
}

pub struct TabManager {
    tabs: HashMap<usize, BrowserTab>,
    active_tab_id: Option<usize>,
    next_tab_id: usize,
}

impl TabManager {
    pub fn new() -> Self {
        Self {
            tabs: HashMap::new(),
            active_tab_id: None,
            next_tab_id: 0,
        }
    }

    pub fn create_tab(&mut self, url: String, webview: Entity<WebView>) -> usize {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        let tab = BrowserTab::new(tab_id, url, webview);
        self.tabs.insert(tab_id, tab);

        if self.active_tab_id.is_none() {
            self.active_tab_id = Some(tab_id);
        }

        tab_id
    }

    pub fn close_tab(&mut self, tab_id: usize) -> bool {
        if self.tabs.remove(&tab_id).is_some() {
            if self.active_tab_id == Some(tab_id) {
                self.active_tab_id = self.tabs.keys().next().copied();
            }
            true
        } else {
            false
        }
    }

    pub fn set_active_tab(&mut self, tab_id: usize) {
        if self.tabs.contains_key(&tab_id) {
            self.active_tab_id = Some(tab_id);
        }
    }

    pub fn get_active_tab(&self) -> Option<&BrowserTab> {
        self.active_tab_id.and_then(|id| self.tabs.get(&id))
    }

    pub fn get_all_tabs(&self) -> Vec<&BrowserTab> {
        self.tabs.values().collect()
    }

    pub fn get_active_tab_id(&self) -> Option<usize> {
        self.active_tab_id
    }

    pub fn update_tab_url(&mut self, tab_id: usize, url: String) {
        if let Some(tab) = self.tabs.get_mut(&tab_id) {
            tab.url = url.clone();
            if tab.title.is_empty() || tab.title == tab.url {
                tab.title = url;
            }
        }
    }

    pub fn update_tab_title(&mut self, tab_id: usize, title: String) {
        if let Some(tab) = self.tabs.get_mut(&tab_id) {
            tab.title = title;
        }
    }

    pub fn set_tab_loading(&mut self, tab_id: usize, loading: bool) {
        if let Some(tab) = self.tabs.get_mut(&tab_id) {
            tab.is_loading = loading;
        }
    }
}

/// Web browser window component
pub struct Browser {
    tab_manager: TabManager,
    url_input: Entity<InputState>,
    focus_handle: FocusHandle,
}

impl Browser {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let url_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter URL or search term...")
        });

        let focus_handle = cx.focus_handle();

        let mut browser = Self {
            tab_manager: TabManager::new(),
            url_input: url_input.clone(),
            focus_handle,
        };

        // Set up URL input handlers
        cx.subscribe(&url_input, Self::on_url_input_event).detach();

        // Create initial tab
        browser.create_new_tab("https://www.google.com", window, cx);

        browser
    }

    fn create_new_tab(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        let webview = WebView::new(url, window, cx);

        // Subscribe to webview events individually
        cx.subscribe(&webview, |this: &mut Browser, webview, event: &AddressChangedEvent, cx| {
            this.on_address_changed(webview, event, cx);
        }).detach();

        cx.subscribe(&webview, |this: &mut Browser, webview, event: &TitleChangedEvent, cx| {
            this.on_title_changed(webview, event, cx);
        }).detach();

        cx.subscribe(&webview, |this: &mut Browser, webview, event: &LoadStartEvent, cx| {
            this.on_load_start(webview, event, cx);
        }).detach();

        cx.subscribe(&webview, |this: &mut Browser, webview, event: &LoadEndEvent, cx| {
            this.on_load_end(webview, event, cx);
        }).detach();

        let tab_id = self.tab_manager.create_tab(url.to_string(), webview);

        // Set the URL in the input field if this is the active tab
        if Some(tab_id) == self.tab_manager.get_active_tab_id() {
            self.update_url_input(url, window, cx);
        }

        cx.notify();
    }

    fn on_address_changed(
        &mut self,
        webview: Entity<WebView>,
        event: &AddressChangedEvent,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.find_tab_by_webview(webview);
        if let Some(tab_id) = tab_id {
            self.tab_manager.update_tab_url(tab_id, event.url.clone());
            cx.notify();
        }
    }

    fn on_title_changed(
        &mut self,
        webview: Entity<WebView>,
        event: &TitleChangedEvent,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.find_tab_by_webview(webview);
        if let Some(tab_id) = tab_id {
            self.tab_manager.update_tab_title(tab_id, event.title.clone());
            cx.notify();
        }
    }

    fn on_load_start(
        &mut self,
        webview: Entity<WebView>,
        _event: &LoadStartEvent,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.find_tab_by_webview(webview);
        if let Some(tab_id) = tab_id {
            self.tab_manager.set_tab_loading(tab_id, true);
            cx.notify();
        }
    }

    fn on_load_end(
        &mut self,
        webview: Entity<WebView>,
        _event: &LoadEndEvent,
        cx: &mut Context<Self>,
    ) {
        let tab_id = self.find_tab_by_webview(webview);
        if let Some(tab_id) = tab_id {
            self.tab_manager.set_tab_loading(tab_id, false);
            cx.notify();
        }
    }

    fn find_tab_by_webview(&self, webview: Entity<WebView>) -> Option<usize> {
        self.tab_manager.get_all_tabs()
            .iter()
            .find(|tab| tab.webview == webview)
            .map(|tab| tab.id)
    }

    fn update_url_input(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.url_input.update(cx, |input, cx| {
            input.replace_text_in_range(None, url, window, cx);
        });
    }

    fn on_url_input_event(
        &mut self,
        _input: Entity<InputState>,
        event: &InputEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            InputEvent::PressEnter { .. } => {
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

    fn on_tab_click(&mut self, tab_index: &usize, _window: &mut Window, cx: &mut Context<Self>) {
        let tabs: Vec<_> = self.tab_manager.get_all_tabs().iter().map(|tab| tab.id).collect();
        if let Some(&tab_id) = tabs.get(*tab_index) {
            self.tab_manager.set_active_tab(tab_id);
            // URL will be updated in render cycle
            cx.notify();
        }
    }

    fn create_new_tab_action(&mut self, _event: &gpui::ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.create_new_tab("https://www.google.com", window, cx);
    }

    fn close_tab_by_id(&mut self, tab_id: usize, cx: &mut Context<Self>) {
        self.tab_manager.close_tab(tab_id);

        // URL will be updated in render cycle

        cx.notify();
    }
}

impl Focusable for Browser {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Browser {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Update URL input with active tab's URL
        if let Some(active_tab) = self.tab_manager.get_active_tab() {
            self.url_input.update(cx, |input, cx| {
                let current_value = input.value();
                if current_value != active_tab.url {
                    input.replace_text_in_range(None, &active_tab.url, window, cx);
                }
            });
        }
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
                        Button::new(("close-tab", tab_id))
                            .icon(IconName::Close)
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(move |this: &mut Browser, _, _, cx| {
                                this.close_tab_by_id(tab_id, cx);
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
                                Button::new("new-tab")
                                    .ghost()
                                    .icon(IconName::Plus)
                                    .on_click(cx.listener(Self::create_new_tab_action))
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

impl EventEmitter<()> for Browser {}