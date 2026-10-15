use super::*;

impl App {
    pub(super) fn apply_selection(&mut self, discipline: u16, heat: u8) {
        let held_results = self
            .results_mode
            .then(|| self.results_snapshot.clone())
            .flatten();
        let results_due = self.results_due;
        let discipline_changed = discipline != self.discipline;
        self.discipline = discipline;
        self.heat_no = if discipline_changed { 1 } else { heat };
        self.event_index = self
            .events
            .iter()
            .position(|(event_discipline, event_heat, _)| {
                *event_discipline == self.discipline && *event_heat == self.heat_no
            })
            .unwrap_or(0);
        if let Some((distance, _)) = self.discipline_map.get(&discipline) {
            self.splits_target = split_count_for_distance(*distance).unwrap_or(self.splits_target);
            self.heat.splits_target = self.splits_target;
        }
        self.clear_heat_state();
        if let Some(snapshot) = held_results {
            self.results_mode = true;
            self.results_due = results_due;
            self.results_snapshot = Some(snapshot);
            self.live_presentation = false;
        }
        if self.connected {
            self.schedule_selection();
        } else {
            self.selection_reset_pending = false;
        }
    }

    pub(super) fn request_selection(&mut self, discipline: u16, heat: u8) {
        if self.heat_activity {
            let message = if discipline != self.discipline {
                format!(
                    "Opravdu přepnout disciplínu z {} na {discipline}? Rozplavba se nastaví na 1.",
                    self.discipline
                )
            } else {
                format!("Opravdu přepnout rozplavbu z {} na {heat}?", self.heat_no)
            };
            self.pending_confirm = Some((discipline, heat, message));
        } else {
            self.apply_selection(discipline, heat);
        }
    }

    pub(super) fn run_shortcut(&mut self, action: ShortcutAction) {
        match action {
            ShortcutAction::PreviousDiscipline if self.discipline > 1 => {
                self.request_selection(self.discipline - 1, 1);
            }
            ShortcutAction::NextDiscipline if self.discipline < 999 => {
                self.request_selection(self.discipline + 1, 1);
            }
            ShortcutAction::PreviousHeat if self.heat_no > 1 => {
                self.request_selection(self.discipline, self.heat_no - 1);
            }
            ShortcutAction::NextHeat if self.heat_no < 99 => {
                self.request_selection(self.discipline, self.heat_no + 1);
            }
            ShortcutAction::ConfirmSelection => {
                if let Some((discipline, heat, _)) = self.pending_confirm.take() {
                    self.apply_selection(discipline, heat);
                }
            }
            ShortcutAction::ToggleLive => self.show_live = !self.show_live,
            _ => {}
        }
    }

    pub(super) fn register_global_shortcuts(&mut self) {
        self.hotkey_manager = None;
        self.hotkey_actions.clear();
        let manager = match GlobalHotKeyManager::new() {
            Ok(manager) => manager,
            Err(error) => {
                self.hotkey_error = Some(format!("Globální zkratky nejsou dostupné: {error}"));
                return;
            }
        };
        let mut hotkeys = Vec::new();
        let mut actions = BTreeMap::new();
        for action in ShortcutAction::ALL {
            let shortcut = self.shortcuts.get(action);
            match shortcut.parse::<HotKey>() {
                Ok(hotkey) => {
                    if actions.insert(hotkey.id(), action).is_some() {
                        self.hotkey_error =
                            Some("Dvě akce mají stejnou zkratku. Upravte je v Nastavení.".into());
                        return;
                    }
                    hotkeys.push(hotkey);
                }
                Err(error) => {
                    self.hotkey_error =
                        Some(format!("Neplatná globální zkratka „{shortcut}“: {error}"));
                    return;
                }
            }
        }
        if let Err(error) = manager.register_all(&hotkeys) {
            self.hotkey_error = Some(format!(
                "Nelze zaregistrovat globální zkratky (možná kolize): {error}"
            ));
            return;
        }
        self.hotkey_manager = Some(manager);
        self.hotkey_actions = actions;
        self.hotkey_error = None;
    }
}
