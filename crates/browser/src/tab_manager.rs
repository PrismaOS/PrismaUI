use gpui::{Entity, App};
use gpui_webview::WebView;
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

    pub fn get_active_tab_mut(&mut self) -> Option<&mut BrowserTab> {
        self.active_tab_id.and_then(move |id| self.tabs.get_mut(&id))
    }

    pub fn get_tab(&self, tab_id: usize) -> Option<&BrowserTab> {
        self.tabs.get(&tab_id)
    }

    pub fn get_tab_mut(&mut self, tab_id: usize) -> Option<&mut BrowserTab> {
        self.tabs.get_mut(&tab_id)
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

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }
}