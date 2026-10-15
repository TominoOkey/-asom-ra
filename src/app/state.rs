use super::*;

impl App {
    pub(super) fn schedule_selection(&mut self) {
        self.selection_reset_pending = true;
        self.selected_delayed = Some(Instant::now() + Duration::from_millis(600));
    }

    pub(super) fn race_lanes(&self) -> Vec<u8> {
        let occupied: Vec<_> = (1..=8)
            .filter(|lane| {
                self.startlist
                    .contains_key(&(self.discipline, self.heat_no, *lane))
            })
            .collect();
        if occupied.is_empty() {
            self.orders.keys().copied().collect()
        } else {
            occupied
        }
    }

    pub(super) fn clear_heat_state(&mut self) {
        self.started_at = None;
        self.heat_activity = false;
        self.orders.clear();
        self.latest.clear();
        self.previous.clear();
        self.finished.clear();
        self.heat.reset(self.discipline, self.heat_no);
        self.simulator_started = None;
        self.simulator_last.clear();
        self.results_mode = false;
        self.results_due = None;
        self.results_auto_next = false;
        self.live_presentation = true;
        self.results_snapshot = None;
    }

    pub(super) fn send_selection_reset(&mut self) {
        self.selection_reset_pending = false;
        self.transmit(
            Record {
                action: 1,
                discipline: self.discipline,
                heat: self.heat_no,
                lane: 0,
                order: 0,
                time_ms: 0,
            },
            None,
        );
        self.log(
            "EVENT",
            "SELECTION",
            &format!("Disciplína {}, rozplavba {}", self.discipline, self.heat_no),
        );
        if self.timer_armed {
            return;
        }
        self.send_timer_byte(50);
        self.selected_delayed = Some(Instant::now() + Duration::from_millis(300));
    }

    pub(super) fn reset_heat(&mut self) {
        self.clear_heat_state();
        self.send_selection_reset();
    }

    pub(super) fn process_input(&mut self, raw: String) {
        let Some(input) = TimerInput::parse(&raw) else {
            self.log("IN", &raw, "");
            return;
        };
        match input {
            TimerInput::Start => {
                if self.started_at.is_none() {
                    if self.race_selected {
                        if self.timing_log.len() >= 40 {
                            self.timing_log.pop_front();
                        }
                        let title = self
                            .events
                            .iter()
                            .find(|(discipline, heat, _)| {
                                *discipline == self.discipline && *heat == self.heat_no
                            })
                            .map(|(_, _, title)| title.clone())
                            .unwrap_or_else(|| format!("Disciplína {}", self.discipline));
                        self.timing_log.push_back(TimingHeat {
                            discipline: self.discipline,
                            heat: self.heat_no,
                            title,
                            started: true,
                            splits: BTreeMap::new(),
                        });
                        self.persist_timing_log();
                    }
                    self.started_at = Some(Instant::now());
                    self.race_elapsed_ms = None;
                    self.results_mode = false;
                    self.results_due = None;
                    self.results_auto_next = false;
                    self.live_presentation = false;
                    self.results_snapshot = None;
                    self.heat_activity = true;
                    self.timer_armed = false;
                    self.orders.clear();
                    self.latest.clear();
                    self.previous.clear();
                    self.finished.clear();
                }
                self.transmit(
                    Record {
                        action: 2,
                        discipline: self.discipline,
                        heat: self.heat_no,
                        lane: 0,
                        order: 0,
                        time_ms: 0,
                    },
                    Some(&raw),
                );
            }
            TimerInput::Split {
                lane_mask,
                raw_time,
            } => {
                self.log("IN", &raw, "");
                let time_ms = if raw_time == 255 {
                    255
                } else {
                    raw_time.saturating_mul(10)
                };
                for lane in 1..=8 {
                    if lane_mask & (1 << (lane - 1)) == 0 || self.finished.contains(&lane) {
                        continue;
                    }
                    self.heat_activity = true;
                    let order = self.orders.get(&lane).copied().unwrap_or(0) + 1;
                    if order > 99 {
                        continue;
                    }
                    self.orders.insert(lane, order);
                    if let Some(heat) = self.timing_log.back_mut()
                        && heat.discipline == self.discipline
                        && heat.heat == self.heat_no
                    {
                        heat.splits.entry(order).or_default().insert(lane, time_ms);
                    }
                    if let Some(last) = self.latest.get(&lane).copied() {
                        self.previous.insert(lane, last);
                    }
                    self.latest.insert(lane, time_ms);
                    let action = if order >= self.splits_target {
                        self.finished.insert(lane);
                        4
                    } else {
                        3
                    };
                    self.transmit(
                        Record {
                            action,
                            discipline: self.discipline,
                            heat: self.heat_no,
                            lane,
                            order,
                            time_ms,
                        },
                        None,
                    );
                }
                self.persist_timing_log();
                if self.results_due.is_none() && !self.finished.is_empty() {
                    let active = self.race_lanes();
                    if !active.is_empty() && active.iter().all(|lane| self.finished.contains(lane))
                    {
                        self.race_elapsed_ms = self
                            .started_at
                            .map(|start| start.elapsed().as_millis() as u32);
                        self.results_due = Some(Instant::now() + Duration::from_secs(1));
                        self.started_at = None;
                    }
                }
            }
        }
    }
}
