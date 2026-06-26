use crate::state::locale_is_en;
use tauri::menu::IconMenuItem;

// ---------------------------------------------------------------------------
// Tray i18n — struct-based approach
// ---------------------------------------------------------------------------

pub struct TrayLocale {
    pub show: &'static str,
    pub lock_active: &'static str,
    pub lock_inactive: &'static str,
    pub break_active: &'static str,
    pub break_inactive: &'static str,
    pub pause: &'static str,
    pub resume: &'static str,
    pub pause_inactive: &'static str,
    pub settings: &'static str,
    pub quit: &'static str,
    pub planning: &'static str,
    pub focus_prefix: &'static str,
    pub break_prefix: &'static str,
    pub paused_label: &'static str,
}

const TRAY_LOCALE_EN: TrayLocale = TrayLocale {
    show: "Show Planner",
    lock_active: "End Focus",
    lock_inactive: "End Focus (Not Started)",
    break_active: "End Break",
    break_inactive: "End Break (Not Started)",
    pause: "Pause Focus",
    resume: "Resume Focus",
    pause_inactive: "Pause Focus (Not Started)",
    settings: "Settings",
    quit: "Quit",
    planning: "Planning",
    focus_prefix: "Focusing",
    break_prefix: "On Break",
    paused_label: "Paused",
};

const TRAY_LOCALE_ZH: TrayLocale = TrayLocale {
    show: "显示计划窗口",
    lock_active: "结束专注",
    lock_inactive: "结束专注 (未开始)",
    break_active: "结束休息",
    break_inactive: "结束休息 (未开始)",
    pause: "暂停专注",
    resume: "继续专注",
    pause_inactive: "暂停专注 (未开始)",
    settings: "设置",
    quit: "退出",
    planning: "计划中",
    focus_prefix: "专注中",
    break_prefix: "休息中",
    paused_label: "已暂停",
};

pub fn tray_locale(locale: &str) -> &'static TrayLocale {
    if locale_is_en(locale) {
        &TRAY_LOCALE_EN
    } else {
        &TRAY_LOCALE_ZH
    }
}

pub fn tray_title_focus(locale: &str, hours: u64, mins: u64, secs: u64) -> String {
    let tl = tray_locale(locale);
    if hours > 0 {
        format!("{} {:02}:{:02}:{:02}", tl.focus_prefix, hours, mins, secs)
    } else {
        format!("{} {:02}:{:02}", tl.focus_prefix, mins, secs)
    }
}

pub fn tray_title_break(locale: &str, mins: u64, secs: u64) -> String {
    let tl = tray_locale(locale);
    format!("{} {:02}:{:02}", tl.break_prefix, mins, secs)
}

pub fn tray_title_break_minutes(locale: &str, mins: u64) -> String {
    let tl = tray_locale(locale);
    format!("{} {:02}:00", tl.break_prefix, mins)
}

// ---------------------------------------------------------------------------
// Tray Menu State — helpers to reduce repetition
// ---------------------------------------------------------------------------

pub struct TrayMenuState {
    pub locale: String,
    pub show_item: Option<IconMenuItem<tauri::Wry>>,
    pub lock_item: Option<IconMenuItem<tauri::Wry>>,
    pub pause_item: Option<IconMenuItem<tauri::Wry>>,
    pub end_break_item: Option<IconMenuItem<tauri::Wry>>,
    pub settings_item: Option<IconMenuItem<tauri::Wry>>,
    pub quit_item: Option<IconMenuItem<tauri::Wry>>,
}

impl Default for TrayMenuState {
    fn default() -> Self {
        Self::new()
    }
}

impl TrayMenuState {
    pub fn new() -> Self {
        Self {
            locale: "system".to_string(),
            show_item: None,
            lock_item: None,
            pause_item: None,
            end_break_item: None,
            settings_item: None,
            quit_item: None,
        }
    }

    pub fn set_locale(&mut self, locale: String) {
        self.locale = crate::state::normalize_locale(&locale).to_string();
        self.refresh_static_labels();
    }

    fn refresh_static_labels(&self) {
        if let Some(item) = &self.show_item {
            let _ = item.set_text(tray_locale(&self.locale).show);
        }
        if let Some(item) = &self.settings_item {
            let _ = item.set_text(tray_locale(&self.locale).settings);
        }
        if let Some(item) = &self.quit_item {
            let _ = item.set_text(tray_locale(&self.locale).quit);
        }
    }

    pub fn set_focus_active(&self) {
        if let Some(item) = &self.lock_item {
            let _ = item.set_enabled(true);
            let _ = item.set_text(tray_locale(&self.locale).lock_active);
        }
        if let Some(item) = &self.pause_item {
            let _ = item.set_enabled(true);
            let _ = item.set_text(tray_locale(&self.locale).pause);
        }
        if let Some(item) = &self.end_break_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text(tray_locale(&self.locale).break_inactive);
        }
    }

    /// Focus session is paused — the pause item becomes "Resume".
    pub fn set_paused(&self) {
        if let Some(item) = &self.pause_item {
            let _ = item.set_enabled(true);
            let _ = item.set_text(tray_locale(&self.locale).resume);
        }
        if let Some(item) = &self.lock_item {
            let _ = item.set_enabled(true);
            let _ = item.set_text(tray_locale(&self.locale).lock_active);
        }
    }

    pub fn set_focus_inactive(&self) {
        if let Some(item) = &self.lock_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text(tray_locale(&self.locale).lock_inactive);
        }
        if let Some(item) = &self.pause_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text(tray_locale(&self.locale).pause_inactive);
        }
    }

    pub fn set_break_active(&self) {
        if let Some(item) = &self.end_break_item {
            let _ = item.set_enabled(true);
            let _ = item.set_text(tray_locale(&self.locale).break_active);
        }
        if let Some(item) = &self.lock_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text(tray_locale(&self.locale).lock_inactive);
        }
    }

    pub fn set_break_inactive(&self) {
        if let Some(item) = &self.end_break_item {
            let _ = item.set_enabled(false);
            let _ = item.set_text(tray_locale(&self.locale).break_inactive);
        }
    }
}
