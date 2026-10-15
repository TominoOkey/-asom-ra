use super::*;

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(12, 18, 27);
        visuals.window_fill = egui::Color32::from_rgb(18, 27, 39);
        visuals.extreme_bg_color = egui::Color32::from_rgb(8, 13, 20);
        visuals.faint_bg_color = egui::Color32::from_rgb(18, 28, 40);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(22, 32, 45);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(35, 48, 64);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(43, 65, 84);
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 135, 164);
        visuals.selection.bg_fill = egui::Color32::from_rgb(0, 170, 200);
        visuals.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
        ctx.set_visuals(visuals);
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(6.0, 3.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        ctx.set_style(style);
        ctx.request_repaint_after(Duration::from_millis(50));
        let hotkey_events: Vec<_> = GlobalHotKeyEvent::receiver().try_iter().collect();
        for event in hotkey_events {
            if event.state() == HotKeyState::Pressed
                && let Some(action) = self.hotkey_actions.get(&event.id()).copied()
            {
                self.run_shortcut(action);
            }
        }
        let shortcut_pressed = ctx.input(|input| {
            ShortcutAction::ALL.into_iter().find(|action| {
                if self.shortcut_capture.is_some() {
                    return false;
                }
                shortcut_matches(input, self.shortcuts.get(*action))
            })
        });
        if let Some(action) = shortcut_pressed {
            self.run_shortcut(action);
        }
        if let Some(action) = self.shortcut_capture {
            let captured = ctx.input(|input| {
                input.events.iter().find_map(|event| match event {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } => Some(format_shortcut(*key, *modifiers)),
                    _ => None,
                })
            });
            if let Some(shortcut) = captured {
                self.shortcuts.set(action, shortcut);
                save_shortcuts(&self.shortcuts);
                self.shortcut_capture = None;
                self.register_global_shortcuts();
            }
        }
        if self.results_due.is_some_and(|due| Instant::now() >= due) {
            if self.results_mode {
                self.results_due = None;
                self.results_mode = false;
                self.results_auto_next = true;
                self.results_snapshot = None;
            } else {
                self.results_mode = true;
                self.results_snapshot = Some(self.overlay_data());
                self.results_due = Some(Instant::now() + Duration::from_secs(10));
            }
        }
        if let Some(rx) = &self.rx {
            let mut events = Vec::new();
            while let Ok(event) = rx.try_recv() {
                events.push(event);
            }
            for event in events {
                match event {
                    Ok(line) => self.process_input(line),
                    Err(e) => {
                        self.status = e;
                        self.disconnect();
                    }
                }
            }
        }
        if let Some(due) = self.selected_delayed {
            if Instant::now() >= due {
                self.selected_delayed = None;
                if self.selection_reset_pending {
                    if self.connected {
                        self.send_selection_reset();
                    } else {
                        self.selection_reset_pending = false;
                    }
                } else if self.connected && self.started_at.is_none() {
                    if self.input.is_some() && !self.timer_armed && self.heat_activity == false {
                        self.send_timer_byte(49);
                        self.timer_armed = true;
                    }
                }
            }
        }

        if !self.race_selected {
            self.race_setup_screen(ctx);
            if let Ok(mut state) = self.overlay_state.write() {
                state.live_enabled = false;
            }
            return;
        }

        self.draw_race_dashboard(ctx);
        if let Ok(mut state) = self.overlay_state.write() {
            *state = OverlayState {
                live_enabled: self.show_live,
                live: if self.results_mode {
                    self.results_snapshot
                        .clone()
                        .unwrap_or_else(|| self.overlay_data())
                } else if self.results_auto_next || self.live_presentation {
                    self.presentation_overlay_data()
                } else {
                    self.overlay_data()
                },
            };
        }
        if self.simulator_visible {
            simulator::show(ctx, self);
        }
        let mut reset_shortcuts_requested = false;
        if self.show_settings {
            egui::Window::new("Nastavení aplikace")
                .open(&mut self.show_settings)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Klávesové zkratky · Stream Deck");
                    ui.label("Vyberte Změnit a stiskněte novou klávesu nebo kombinaci. Zkratky fungují, když je aplikace aktivní.");
                    for action in ShortcutAction::ALL {
                        ui.horizontal(|ui| {
                            ui.label(action.label());
                            ui.label(self.shortcuts.get(action));
                            let capturing = self.shortcut_capture == Some(action);
                            if ui.button(if capturing { "Stiskněte klávesu…" } else { "Změnit" }).clicked() {
                                self.shortcut_capture = Some(action);
                            }
                        });
                    }
                    if ui.button("Obnovit výchozí zkratky").clicked() {
                        self.shortcuts = ShortcutSettings::default();
                        save_shortcuts(&self.shortcuts);
                        self.shortcut_capture = None;
                        reset_shortcuts_requested = true;
                    }
                    ui.separator();
                    ui.label("Globální zkratky fungují i při práci v jiné aplikaci. Stream Deck nastavte na stejné klávesové kombinace.");
                    ui.label("Enter potvrdí otevřený dialog změny závodu. Při běžném ovládání nic nemění.");
                    if let Some(error) = &self.hotkey_error {
                        ui.colored_label(egui::Color32::LIGHT_RED, error);
                    }
                });
        }
        if reset_shortcuts_requested {
            self.register_global_shortcuts();
        }
        if let Some((discipline, heat, message)) = self.pending_confirm.clone() {
            egui::Window::new("Potvrdit změnu závodu")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(message);
                    ui.horizontal(|ui| {
                        if ui.button("Potvrdit").clicked() {
                            self.pending_confirm = None;
                            self.apply_selection(discipline, heat);
                        }
                        if ui.button("Zrušit").clicked() {
                            self.pending_confirm = None;
                        }
                    });
                });
        }
        if self.reset_confirm_pending {
            egui::Window::new("Resetovat rozplavbu?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label("Vymažou se všechny mezičasy a závodní čas. Časomíra obdrží resetovací signál.");
                    ui.horizontal(|ui| {
                        if ui.button("Resetovat").clicked() {
                            self.reset_confirm_pending = false;
                            self.apply_selection(self.discipline, self.heat_no);
                        }
                        if ui.button("Zrušit").clicked() { self.reset_confirm_pending = false; }
                    });
                });
        }
    }
}

pub(super) mod controls;
pub(super) mod dashboard;
pub(super) mod setup;
pub(super) mod simulator;
pub(super) mod timekeeper_log;
