//! Every setting the user can change, and what changing it does. Shared by the Settings window (settingswin.rs) and
//! the in-notch Settings slide (settingsview.rs), so a change is applied the same way from both.

use super::*;
use crate::settings::{LEAD_MAX, LEAD_MIN, LEAD_STEP};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum Opt {
    Minimal,
    BusyOnMax,
    Monitor,
    LyricLead,
    NotchSize,
    IdleDisplay,
    Notifications,
    NotifAvatar,
    Autostart,
    SlideHotkeys,
    Lang,
    /// Button: paste the calendar link from the clipboard, or remove it (press twice).
    Calendar,
    HudVolume,
    HudBrightness,
    LockToast,
    PrivacyDot,
    Weather,
    City,
    Prayer,
    Accent,
    /// Toggle once the dictionary is there, a download button before that.
    Romaji,
    /// Button: delete the downloaded dictionary (press twice).
    RomajiDict,
}

impl App {
    /// On/off state of a toggle setting.
    pub(crate) fn opt_on(&self, o: Opt) -> bool {
        match o {
            Opt::Minimal => self.minimal,
            Opt::BusyOnMax => self.cfg.busy_on_maximized,
            Opt::Notifications => self.notif_on,
            Opt::NotifAvatar => self.cfg.notif_avatar,
            Opt::Autostart => self.sui.autostart,
            Opt::SlideHotkeys => self.cfg.slide_hotkeys,
            Opt::HudVolume => self.cfg.hud_volume,
            Opt::HudBrightness => self.cfg.hud_brightness,
            Opt::LockToast => self.cfg.lock_toast,
            Opt::PrivacyDot => self.cfg.privacy_dot,
            Opt::Weather => self.cfg.weather_on,
            Opt::Prayer => self.cfg.prayer_on,
            Opt::Romaji => self.cfg.romaji_lyrics,
            _ => false,
        }
    }

    /// Current choice of a choice setting (index into its list of choices).
    pub(crate) fn opt_choice(&self, o: Opt) -> usize {
        match o {
            Opt::Monitor => self.cfg.monitor.min(monitors::mode_count() - 1),
            Opt::NotchSize => self.cfg.notch_size,
            Opt::IdleDisplay => self.cfg.idle_display,
            Opt::Lang => self.cfg.lang,
            Opt::City => self.cfg.weather_city,
            Opt::Accent => self.cfg.accent,
            _ => 0,
        }
    }

    /// Flip a toggle, or press a button setting. Saves.
    pub(crate) fn opt_press(&mut self, o: Opt) {
        match o {
            Opt::Minimal => self.minimal = !self.minimal,
            Opt::BusyOnMax => self.cfg.busy_on_maximized = !self.cfg.busy_on_maximized,
            Opt::Notifications => {
                self.notif_on = !self.notif_on;
                notif::set_enabled(self.notif_on);
                if self.notif_on {
                    if !self.notif_started {
                        self.notif_started = true;
                        notif::start(self.hwnd, self.notif_slot.clone());
                    }
                } else {
                    self.notifs.clear();
                }
            }
            Opt::NotifAvatar => {
                self.cfg.notif_avatar = !self.cfg.notif_avatar;
                notif::set_avatar(self.cfg.notif_avatar);
            }
            Opt::Autostart => {
                sys::set_autostart(!self.sui.autostart);
                self.sui.autostart = sys::autostart_enabled();
            }
            Opt::SlideHotkeys => {
                self.cfg.slide_hotkeys = !self.cfg.slide_hotkeys;
                hotkeys::set_slide_hotkeys(self.hwnd, self.cfg.slide_hotkeys);
            }
            Opt::Calendar => {
                let clip = calendar::clipboard_text(self.hwnd);
                self.cal_button(clip);
            }
            Opt::HudVolume => self.cfg.hud_volume = !self.cfg.hud_volume,
            Opt::HudBrightness => self.cfg.hud_brightness = !self.cfg.hud_brightness,
            Opt::LockToast => {
                self.cfg.lock_toast = !self.cfg.lock_toast;
                keys::set_enabled(self.hwnd, self.cfg.lock_toast);
            }
            Opt::PrivacyDot => {
                self.cfg.privacy_dot = !self.cfg.privacy_dot;
                privacy::set_enabled(self.cfg.privacy_dot);
                self.priv_t = Instant::now();
            }
            Opt::Weather => {
                self.cfg.weather_on = !self.cfg.weather_on;
                self.weather_apply();
            }
            Opt::Prayer => {
                self.cfg.prayer_on = !self.cfg.prayer_on;
                self.prayer_refresh();
            }
            Opt::Romaji => {
                if romaji::available() {
                    self.cfg.romaji_lyrics = !self.cfg.romaji_lyrics;
                    romaji::set_enabled(self.cfg.romaji_lyrics);
                    self.romaji_refresh();
                } else {
                    dict::start(self.hwnd);
                }
            }
            Opt::RomajiDict => {
                if !self.sui.dict_confirming() {
                    self.sui.dict_confirm = Some(Instant::now());
                } else {
                    self.sui.dict_confirm = None;
                    if dict::remove() {
                        self.cfg.romaji_lyrics = false;
                        romaji::set_enabled(false);
                        self.romaji_refresh();
                    }
                }
            }
            Opt::Monitor | Opt::LyricLead | Opt::NotchSize | Opt::IdleDisplay | Opt::Lang | Opt::City | Opt::Accent => {}
        }
        self.save_settings();
    }

    /// Move a choice setting one step back or forward (the lyric timing by `LEAD_STEP`). Saves.
    pub(crate) fn opt_step(&mut self, o: Opt, plus: bool) {
        let i = self.opt_choice(o);
        match o {
            Opt::LyricLead => {
                let d = if plus { LEAD_STEP } else { -LEAD_STEP };
                self.cfg.lyric_lead_ms = (self.cfg.lyric_lead_ms + d).clamp(LEAD_MIN, LEAD_MAX);
                self.save_settings();
            }
            Opt::Monitor => self.opt_pick(o, monitors::step_mode(i, plus)),
            Opt::NotchSize => self.opt_pick(o, if plus { (i + 1).min(scale::SIZES.len() - 1) } else { i.saturating_sub(1) }),
            Opt::IdleDisplay => {
                self.opt_pick(o, if plus { (i + 1).min(settings::IDLE_DISPLAY_LABELS.len() - 1) } else { i.saturating_sub(1) })
            }
            Opt::Lang => self.opt_pick(o, i18n::step(i, plus)),
            Opt::City => {
                let n = weather::CITIES.len();
                self.opt_pick(o, if plus { (i + 1) % n } else { (i + n - 1) % n })
            }
            Opt::Accent => self.opt_pick(o, accent::step(i, plus)),
            _ => {}
        }
    }

    /// Set a choice setting to choice `i` directly. Saves.
    pub(crate) fn opt_pick(&mut self, o: Opt, i: usize) {
        match o {
            Opt::Monitor => {
                self.cfg.monitor = i.min(monitors::mode_count() - 1);
                self.place(true);
            }
            Opt::NotchSize => {
                self.cfg.notch_size = i.min(scale::SIZES.len() - 1);
                self.place(true);
            }
            Opt::IdleDisplay => {
                self.cfg.idle_display = i.min(settings::IDLE_DISPLAY_LABELS.len() - 1);
                self.cur_phase = self.idle_phase();
                self.eye = EyeAnim::new();
            }
            Opt::Lang => {
                self.cfg.lang = i.min(i18n::LANGS.len() - 1);
                i18n::set(self.cfg.lang);
            }
            Opt::City => {
                self.cfg.weather_city = i.min(weather::CITIES.len() - 1);
                self.weather_apply();
                self.prayer_refresh();
            }
            Opt::Accent => self.cfg.accent = i.min(accent::ACCENTS.len() - 1),
            _ => return,
        }
        self.save_settings();
    }
}
