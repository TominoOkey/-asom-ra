//! Shortcut definitions, persistence and egui key conversion.

use eframe::egui;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum ShortcutAction {
    PreviousDiscipline,
    NextDiscipline,
    PreviousHeat,
    NextHeat,
    ConfirmSelection,
    ToggleLive,
}

impl ShortcutAction {
    pub(super) const ALL: [Self; 6] = [
        Self::PreviousDiscipline,
        Self::NextDiscipline,
        Self::PreviousHeat,
        Self::NextHeat,
        Self::ConfirmSelection,
        Self::ToggleLive,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::PreviousDiscipline => "Předchozí disciplína",
            Self::NextDiscipline => "Další disciplína",
            Self::PreviousHeat => "Předchozí rozplavba",
            Self::NextHeat => "Další rozplavba",
            Self::ConfirmSelection => "Potvrdit výběr",
            Self::ToggleLive => "Zapnout/vypnout OBS LIVE",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct ShortcutSettings {
    previous_discipline: String,
    next_discipline: String,
    previous_heat: String,
    next_heat: String,
    confirm_selection: String,
    toggle_live: String,
}

impl Default for ShortcutSettings {
    fn default() -> Self {
        Self {
            previous_discipline: "Ctrl+Alt+Left".into(),
            next_discipline: "Ctrl+Alt+Right".into(),
            previous_heat: "Ctrl+Left".into(),
            next_heat: "Ctrl+Right".into(),
            confirm_selection: "Ctrl+Enter".into(),
            toggle_live: "Ctrl+F8".into(),
        }
    }
}

impl ShortcutSettings {
    pub(super) fn get(&self, action: ShortcutAction) -> &str {
        match action {
            ShortcutAction::PreviousDiscipline => &self.previous_discipline,
            ShortcutAction::NextDiscipline => &self.next_discipline,
            ShortcutAction::PreviousHeat => &self.previous_heat,
            ShortcutAction::NextHeat => &self.next_heat,
            ShortcutAction::ConfirmSelection => &self.confirm_selection,
            ShortcutAction::ToggleLive => &self.toggle_live,
        }
    }

    pub(super) fn set(&mut self, action: ShortcutAction, value: String) {
        match action {
            ShortcutAction::PreviousDiscipline => self.previous_discipline = value,
            ShortcutAction::NextDiscipline => self.next_discipline = value,
            ShortcutAction::PreviousHeat => self.previous_heat = value,
            ShortcutAction::NextHeat => self.next_heat = value,
            ShortcutAction::ConfirmSelection => self.confirm_selection = value,
            ShortcutAction::ToggleLive => self.toggle_live = value,
        }
    }
}

fn shortcuts_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|home| home.join(".casomira_shortcuts.json"))
}

pub(super) fn load_shortcuts() -> ShortcutSettings {
    shortcuts_path()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub(super) fn save_shortcuts(settings: &ShortcutSettings) {
    if let Some(path) = shortcuts_path()
        && let Ok(bytes) = serde_json::to_vec_pretty(settings)
    {
        let _ = fs::write(path, bytes);
    }
}

pub(super) fn format_shortcut(key: egui::Key, modifiers: egui::Modifiers) -> String {
    let mut parts = Vec::new();
    if modifiers.ctrl {
        parts.push("Ctrl".to_owned());
    }
    if modifiers.alt {
        parts.push("Alt".to_owned());
    }
    if modifiers.shift {
        parts.push("Shift".to_owned());
    }
    if modifiers.mac_cmd || modifiers.command {
        parts.push("Cmd".to_owned());
    }
    parts.push(match key {
        egui::Key::ArrowLeft => "Left".into(),
        egui::Key::ArrowRight => "Right".into(),
        egui::Key::ArrowUp => "Up".into(),
        egui::Key::ArrowDown => "Down".into(),
        egui::Key::Enter => "Enter".into(),
        egui::Key::Escape => "Esc".into(),
        egui::Key::Space => "Space".into(),
        egui::Key::Slash => "Slash".into(),
        egui::Key::Questionmark => "Slash".into(),
        egui::Key::F1 => "F1".into(),
        egui::Key::F2 => "F2".into(),
        egui::Key::F3 => "F3".into(),
        egui::Key::F4 => "F4".into(),
        egui::Key::F5 => "F5".into(),
        egui::Key::F6 => "F6".into(),
        egui::Key::F7 => "F7".into(),
        egui::Key::F8 => "F8".into(),
        egui::Key::F9 => "F9".into(),
        egui::Key::F10 => "F10".into(),
        egui::Key::F11 => "F11".into(),
        egui::Key::F12 => "F12".into(),
        egui::Key::Num0 => "0".into(),
        egui::Key::Num1 => "1".into(),
        egui::Key::Num2 => "2".into(),
        egui::Key::Num3 => "3".into(),
        egui::Key::Num4 => "4".into(),
        egui::Key::Num5 => "5".into(),
        egui::Key::Num6 => "6".into(),
        egui::Key::Num7 => "7".into(),
        egui::Key::Num8 => "8".into(),
        egui::Key::Num9 => "9".into(),
        egui::Key::A => "A".into(),
        egui::Key::B => "B".into(),
        egui::Key::C => "C".into(),
        egui::Key::D => "D".into(),
        egui::Key::E => "E".into(),
        egui::Key::F => "F".into(),
        egui::Key::G => "G".into(),
        egui::Key::H => "H".into(),
        egui::Key::I => "I".into(),
        egui::Key::J => "J".into(),
        egui::Key::K => "K".into(),
        egui::Key::L => "L".into(),
        egui::Key::M => "M".into(),
        egui::Key::N => "N".into(),
        egui::Key::O => "O".into(),
        egui::Key::P => "P".into(),
        egui::Key::Q => "Q".into(),
        egui::Key::R => "R".into(),
        egui::Key::S => "S".into(),
        egui::Key::T => "T".into(),
        egui::Key::U => "U".into(),
        egui::Key::V => "V".into(),
        egui::Key::W => "W".into(),
        egui::Key::X => "X".into(),
        egui::Key::Y => "Y".into(),
        egui::Key::Z => "Z".into(),
        egui::Key::F13 => "F13".into(),
        egui::Key::F14 => "F14".into(),
        egui::Key::F15 => "F15".into(),
        egui::Key::F16 => "F16".into(),
        egui::Key::F17 => "F17".into(),
        egui::Key::F18 => "F18".into(),
        egui::Key::F19 => "F19".into(),
        egui::Key::F20 => "F20".into(),
        egui::Key::F21 => "F21".into(),
        egui::Key::F22 => "F22".into(),
        egui::Key::F23 => "F23".into(),
        egui::Key::F24 => "F24".into(),
        egui::Key::Tab => "Tab".into(),
        egui::Key::Backspace => "Backspace".into(),
        egui::Key::Delete => "Delete".into(),
        egui::Key::Home => "Home".into(),
        egui::Key::End => "End".into(),
        egui::Key::PageUp => "PageUp".into(),
        egui::Key::PageDown => "PageDown".into(),
        egui::Key::Insert => "Insert".into(),
        _ => format!("{key:?}"),
    });
    parts.join("+")
}

pub(super) fn shortcut_matches(input: &egui::InputState, shortcut: &str) -> bool {
    let Some(key_name) = shortcut.rsplit('+').next() else {
        return false;
    };
    let key = match key_name {
        "Left" | "ArrowLeft" => egui::Key::ArrowLeft,
        "Right" | "ArrowRight" => egui::Key::ArrowRight,
        "Up" | "ArrowUp" => egui::Key::ArrowUp,
        "Down" | "ArrowDown" => egui::Key::ArrowDown,
        "Enter" => egui::Key::Enter,
        "Esc" | "Escape" => egui::Key::Escape,
        "Space" => egui::Key::Space,
        "Slash" | "/" => egui::Key::Slash,
        "Questionmark" => egui::Key::Questionmark,
        "F1" => egui::Key::F1,
        "F2" => egui::Key::F2,
        "F3" => egui::Key::F3,
        "F4" => egui::Key::F4,
        "F5" => egui::Key::F5,
        "F6" => egui::Key::F6,
        "F7" => egui::Key::F7,
        "F8" => egui::Key::F8,
        "F9" => egui::Key::F9,
        "F10" => egui::Key::F10,
        "F11" => egui::Key::F11,
        "F12" => egui::Key::F12,
        "F13" => egui::Key::F13,
        "F14" => egui::Key::F14,
        "F15" => egui::Key::F15,
        "F16" => egui::Key::F16,
        "F17" => egui::Key::F17,
        "F18" => egui::Key::F18,
        "F19" => egui::Key::F19,
        "F20" => egui::Key::F20,
        "F21" => egui::Key::F21,
        "F22" => egui::Key::F22,
        "F23" => egui::Key::F23,
        "F24" => egui::Key::F24,
        "0" => egui::Key::Num0,
        "1" => egui::Key::Num1,
        "2" => egui::Key::Num2,
        "3" => egui::Key::Num3,
        "4" => egui::Key::Num4,
        "5" => egui::Key::Num5,
        "6" => egui::Key::Num6,
        "7" => egui::Key::Num7,
        "8" => egui::Key::Num8,
        "9" => egui::Key::Num9,
        "A" => egui::Key::A,
        "B" => egui::Key::B,
        "C" => egui::Key::C,
        "D" => egui::Key::D,
        "E" => egui::Key::E,
        "F" => egui::Key::F,
        "G" => egui::Key::G,
        "H" => egui::Key::H,
        "I" => egui::Key::I,
        "J" => egui::Key::J,
        "K" => egui::Key::K,
        "L" => egui::Key::L,
        "M" => egui::Key::M,
        "N" => egui::Key::N,
        "O" => egui::Key::O,
        "P" => egui::Key::P,
        "Q" => egui::Key::Q,
        "R" => egui::Key::R,
        "S" => egui::Key::S,
        "T" => egui::Key::T,
        "U" => egui::Key::U,
        "V" => egui::Key::V,
        "W" => egui::Key::W,
        "X" => egui::Key::X,
        "Y" => egui::Key::Y,
        "Z" => egui::Key::Z,
        "Tab" => egui::Key::Tab,
        "Backspace" => egui::Key::Backspace,
        "Delete" => egui::Key::Delete,
        "Home" => egui::Key::Home,
        "End" => egui::Key::End,
        "PageUp" => egui::Key::PageUp,
        "PageDown" => egui::Key::PageDown,
        "Insert" => egui::Key::Insert,
        _ => return false,
    };
    let has = |name: &str| shortcut.split('+').any(|part| part == name);
    let cmd_pressed = input.modifiers.mac_cmd || input.modifiers.command;
    input.key_pressed(key)
        && input.modifiers.ctrl == has("Ctrl")
        && input.modifiers.alt == has("Alt")
        && input.modifiers.shift == has("Shift")
        && cmd_pressed == has("Cmd")
}
