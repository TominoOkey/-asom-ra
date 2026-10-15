use super::*;
use crate::selection_control;
use casomira::SPLIT_OPTIONS;

pub(super) fn show(app: &mut App, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading(egui::RichText::new("ČASOMÍRA").size(24.0).strong());
        ui.separator();
        ui.colored_label(
            if app.connected {
                egui::Color32::from_rgb(52, 211, 153)
            } else {
                egui::Color32::from_rgb(248, 113, 113)
            },
            if app.connected {
                "● PŘIPOJENO"
            } else {
                "● ODPOJENO"
            },
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if app.connected {
                if ui.button("Odpojit").clicked() {
                    app.disconnect();
                }
            } else if ui.button("Připojit").clicked() {
                app.connect();
            }
            egui::ComboBox::from_id_salt("input-port")
                .selected_text(if app.input_name.is_empty() {
                    "Vyberte port"
                } else {
                    &app.input_name
                })
                .show_ui(ui, |ui| {
                    for port in &app.ports {
                        ui.selectable_value(&mut app.input_name, port.clone(), port);
                    }
                });
            ui.label("Vstup:");
        });
    });
    ui.horizontal(|ui| {
        if ui.button("Startovní listina").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("CSV / text", &["csv", "txt"])
                .pick_file()
            {
                app.import_startlist(&path);
            }
        }
        if ui.button("Import disciplín").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("CSV / text", &["csv", "txt"])
                .pick_file()
            {
                app.import_disciplines(&path);
            }
        }
        if ui.button("Nový závod").clicked() {
            app.create_new_race();
        }
        if ui.button("Složka závodu").clicked() {
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                if let Err(error) = app.load_race_folder(&path) {
                    app.status = error;
                }
            }
        }
        ui.label(format!("Logy: {}", app.event_name));
        if ui.button("Obnovit porty").clicked() {
            app.refresh_ports();
        }
    });
    ui.label(&app.status);
    if let Some(err) = &app.log_error {
        ui.colored_label(egui::Color32::RED, format!("Chyba logu: {err}"));
    }
    ui.separator();
    let ready = !app.startlist.is_empty() && !app.discipline_map.is_empty();
    ui.colored_label(
        if ready {
            egui::Color32::GREEN
        } else {
            egui::Color32::RED
        },
        if ready {
            "Startovní listina a disciplíny načtené"
        } else {
            "Načtěte startovní listinu a disciplíny"
        },
    );
    egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                let discipline_flash = app.selection_flash.as_ref().filter(|(id, _, at)| id == "discipline" && at.elapsed() < Duration::from_millis(380)).map(|(_, plus, at)| (*plus, *at));
                let (minus, plus) = selection_control(ui, "DISCIPLÍNA", app.discipline.to_string(), discipline_flash);
                if minus.clicked() {
                    app.selection_flash = Some(("discipline".into(), false, Instant::now()));
                }
                if minus.clicked() && app.discipline > 1 {
                    let next = app.discipline - 1;
                    if app.heat_activity { app.pending_confirm = Some((next, 1, format!("Opravdu přepnout disciplínu z {} na {next}? Rozplavba se nastaví na 1.", app.discipline))); }
                    else { app.apply_selection(next, 1); }
                }
                if plus.clicked() {
                    app.selection_flash = Some(("discipline".into(), true, Instant::now()));
                }
                if plus.clicked() && app.discipline < 999 {
                    let next = app.discipline + 1;
                    if app.heat_activity { app.pending_confirm = Some((next, 1, format!("Opravdu přepnout disciplínu z {} na {next}? Rozplavba se nastaví na 1.", app.discipline))); }
                    else { app.apply_selection(next, 1); }
                }
                ui.separator();
                let heat_flash = app.selection_flash.as_ref().filter(|(id, _, at)| id == "heat" && at.elapsed() < Duration::from_millis(380)).map(|(_, plus, at)| (*plus, *at));
                let (minus, plus) = selection_control(ui, "ROZPLAVBA", app.heat_no.to_string(), heat_flash);
                if minus.clicked() {
                    app.selection_flash = Some(("heat".into(), false, Instant::now()));
                }
                if minus.clicked() && app.heat_no > 1 {
                    let next = app.heat_no - 1;
                    if app.heat_activity { app.pending_confirm = Some((app.discipline, next, format!("Opravdu přepnout rozplavbu z {} na {next}?", app.heat_no))); }
                    else { app.apply_selection(app.discipline, next); }
                }
                if plus.clicked() {
                    app.selection_flash = Some(("heat".into(), true, Instant::now()));
                }
                if plus.clicked() && app.heat_no < 99 {
                    let next = app.heat_no + 1;
                    if app.heat_activity { app.pending_confirm = Some((app.discipline, next, format!("Opravdu přepnout rozplavbu z {} na {next}?", app.heat_no))); }
                    else { app.apply_selection(app.discipline, next); }
                }
                ui.separator();
                let split_label = SPLIT_OPTIONS.iter().find(|(n, _)| *n == app.splits_target).map(|(n, l)| format!("{n} · {l}")).unwrap_or_default();
                let split_flash = app.selection_flash.as_ref().filter(|(id, _, at)| id == "splits" && at.elapsed() < Duration::from_millis(380)).map(|(_, plus, at)| (*plus, *at));
                let (minus, plus) = selection_control(ui, "MEZIČASY / TRAŤ", split_label, split_flash);
                let split_pos = SPLIT_OPTIONS
                    .iter()
                    .position(|(n, _)| *n == app.splits_target)
                    .unwrap_or(0);
                if minus.clicked() {
                    app.selection_flash = Some(("splits".into(), false, Instant::now()));
                    app.splits_target = SPLIT_OPTIONS[split_pos.saturating_sub(1)].0;
                    app.heat.splits_target = app.splits_target;
                }
                if plus.clicked() {
                    app.selection_flash = Some(("splits".into(), true, Instant::now()));
                    app.splits_target =
                        SPLIT_OPTIONS[(split_pos + 1).min(SPLIT_OPTIONS.len() - 1)].0;
                    app.heat.splits_target = app.splits_target;
                }
                ui.separator();
                if ui.button("↻ Resetovat rozplavbu").clicked() {
                    app.reset_confirm_pending = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut app.simulator_visible, "Simulátor");
                    ui.checkbox(&mut app.show_live, "LIVE");
                    if ui.button("⚙ Nastavení").clicked() { app.show_settings = true; }
                    if app.show_live { ui.label("OBS Browser: http://127.0.0.1:47831/live"); }
                });
            });
            });
}
