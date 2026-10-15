#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use casomira::{
    BAUD_RATE, Heat, OUTPUT_PORTS, Record, StartList, TimerInput, format_short_time,
    parse_discipline_import, parse_startlist, split_count_for_distance,
};
mod app;
mod overlay;
mod race_files;
mod serial_link;
mod shortcuts;

use app::App;
use eframe::egui;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use overlay::{OverlayData, OverlayLane, OverlayState, empty_overlay_state, start_overlay_server};
use race_files::{RACE_FILES, read_discipline_map, read_text_file, serialize_disciplines};
use serde::{Deserialize, Serialize};
use serial_link::SerialLink;
use serialport::available_ports;
use shortcuts::{
    ShortcutAction, ShortcutSettings, format_shortcut, load_shortcuts, save_shortcuts,
    shortcut_matches,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::{Duration, Instant};

fn selection_control(
    ui: &mut egui::Ui,
    title: &str,
    value: String,
    flash: Option<(bool, Instant)>,
) -> (egui::Response, egui::Response) {
    let mut minus = egui::Button::new(egui::RichText::new("−").size(22.0).strong())
        .min_size(egui::vec2(38.0, 38.0));
    let mut plus = egui::Button::new(egui::RichText::new("+").size(22.0).strong())
        .min_size(egui::vec2(38.0, 38.0));
    if let Some((is_plus, at)) = flash {
        let fraction = (1.0 - at.elapsed().as_secs_f32() / 0.38).clamp(0.0, 1.0);
        let target = if is_plus {
            egui::Color32::from_rgb(34, 197, 94)
        } else {
            egui::Color32::from_rgb(239, 68, 68)
        };
        let base = ui.visuals().widgets.inactive.bg_fill;
        let color = egui::Color32::from_rgba_unmultiplied(
            target.r(),
            target.g(),
            target.b(),
            (fraction * 220.0) as u8,
        );
        if is_plus {
            plus = plus.fill(color.gamma_multiply(fraction).blend(base));
        } else {
            minus = minus.fill(color.gamma_multiply(fraction).blend(base));
        }
        ui.ctx().request_repaint_after(Duration::from_millis(16));
    }
    let responses = ui.vertical(|ui| {
        ui.label(
            egui::RichText::new(title)
                .size(11.0)
                .strong()
                .color(egui::Color32::LIGHT_GRAY),
        );
        let responses = ui.horizontal(|ui| {
            let minus_response = ui.add(minus);
            ui.allocate_ui_with_layout(
                egui::vec2(62.0, 38.0),
                egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                |ui| {
                    ui.label(
                        egui::RichText::new(value)
                            .size(20.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    );
                },
            );
            let plus_response = ui.add(plus);
            (minus_response, plus_response)
        });
        responses.inner
    });
    responses.inner
}

fn format_monitor_time(time_ms: u32) -> String {
    let minutes = time_ms / 60_000;
    let seconds = (time_ms / 1_000) % 60;
    let centiseconds = (time_ms % 1_000) / 10;
    format!("{minutes:02}:{seconds:02},{centiseconds:02}")
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1420.0, 850.0])
            .with_min_inner_size([1080.0, 760.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Časomíra",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}
