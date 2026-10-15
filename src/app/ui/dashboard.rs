use super::*;

impl App {
    pub(super) fn draw_race_dashboard(&mut self, ctx: &egui::Context) {
        let log_width = (ctx.content_rect().width() * 0.30).clamp(250.0, 520.0);
        super::timekeeper_log::show(self, ctx, log_width);

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(egui::Color32::from_rgb(12, 18, 27)).inner_margin(egui::Margin::same(7)))
            .show(ctx, |ui| {
            super::controls::show(self, ui);
            if let Some((_, _, name)) = self.events.iter().find(|(discipline, heat, _)| {
                *discipline == self.discipline && *heat == self.heat_no
            }) {
                ui.label(egui::RichText::new(name).color(egui::Color32::LIGHT_GRAY));
            }
            ui.separator();
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("PRŮBĚH ROZPLAVBY").size(18.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let elapsed = self
                        .started_at
                        .map(|s| s.elapsed().as_millis() as u32)
                        .unwrap_or(0);
                    ui.label(egui::RichText::new(format_short_time(Some(elapsed))).size(24.0).strong().color(egui::Color32::WHITE));
                });
            });
            let table_height = ui.available_height().max(120.0);
            let row_height = ((table_height - 32.0) / 8.0).clamp(26.0, 36.0);
            ui.spacing_mut().interact_size.y = 18.0;
            ui.spacing_mut().item_spacing.y = 1.0;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(table_height)
                .show(ui, |ui| {
                    let table_width = ui.available_width();
                    let content_width = (table_width - 16.0).max(100.0);
                    let lane_width = 66.0;
                    let club_width = 120.0;
                    let diff_width = 100.0;
                    let time_width = 116.0;
                    let warning_width = 48.0;
                    let action_width = 76.0;
                    let column_gap = 5.0;
                    let name_width = (content_width
                        - lane_width
                        - club_width
                        - diff_width
                        - time_width
                        - warning_width
                        - action_width
                        - column_gap * 7.0)
                        .max(120.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = column_gap;
                        for (label, width) in [
                            ("DRÁHA", lane_width),
                            ("KLUB", club_width),
                            ("ZÁVODNÍK", name_width),
                            ("ROZDÍL", diff_width),
                            ("MEZIČAS", time_width),
                            ("KONTROLA", warning_width),
                            ("RUČNÍ OPRAVA", action_width),
                        ] {
                            ui.add_sized(
                                egui::vec2(width, 18.0),
                                egui::Label::new(
                                    egui::RichText::new(label)
                                        .size(11.0)
                                        .strong()
                                        .color(egui::Color32::from_rgb(160, 178, 194)),
                                ),
                            );
                        }
                    });
                    ui.add_space(2.0);

                    let target = self.splits_target.max(1);
                    let row_content_height = (row_height - 11.0).max(17.0);
                    for lane in 1..=8 {
                        let order = self.orders.get(&lane).copied().unwrap_or(0);
                        let completed_splits = order.min(target);
                        let completed_distance = completed_splits as u32 * 50;
                        let swimmer = self.startlist.get(&(self.discipline, self.heat_no, lane));
                        let swimmer_name = swimmer
                            .map(|swimmer| swimmer.name.clone())
                            .unwrap_or_default();
                        let swimmer_club = swimmer
                            .map(|swimmer| swimmer.club.clone())
                            .unwrap_or_default();
                        let entry_centiseconds = swimmer
                            .map(|swimmer| swimmer.entry_centiseconds)
                            .unwrap_or(999_999);
                        let diff = self
                            .latest
                            .get(&lane)
                            .map(|time| {
                                format_short_time(Some(time.saturating_sub(
                                    self.previous.get(&lane).copied().unwrap_or(0),
                                )))
                            })
                            .unwrap_or_default();
                        let split_time = self
                            .latest
                            .get(&lane)
                            .map(|time| {
                                format!(
                                    "{:02}:{:02}.{:02}",
                                    time / 60000,
                                    (time / 1000) % 60,
                                    (time / 10) % 100
                                )
                            })
                            .unwrap_or_else(|| "--:--.--".into());
                        let warning = self.latest.get(&lane).is_some_and(|time| {
                            entry_centiseconds < 999_999
                                && *time > 0
                                && ((*time as f64)
                                    < (entry_centiseconds as f64
                                        * 10.0
                                        * order as f64
                                        / target as f64
                                        * 0.6)
                                    || (*time as f64)
                                        > (entry_centiseconds as f64
                                            * 10.0
                                            * order as f64
                                            / target as f64
                                            * 1.7))
                        });
                        let row_fill = if lane % 2 == 0 {
                            egui::Color32::from_rgb(19, 29, 41)
                        } else {
                            egui::Color32::from_rgb(15, 23, 34)
                        };
                        egui::Frame::new()
                            .fill(row_fill)
                            .corner_radius(egui::CornerRadius::same(4))
                            .inner_margin(egui::Margin::symmetric(5, 0))
                            .show(ui, |ui| {
                                ui.set_min_width(content_width);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = column_gap;
                                    ui.add_sized(
                                        egui::vec2(lane_width, row_content_height),
                                        egui::Label::new(
                                            egui::RichText::new(format!("{lane:02}"))
                                                .size(14.0)
                                                .strong()
                                                .color(egui::Color32::WHITE),
                                        ),
                                    );
                                    ui.add_sized(
                                        egui::vec2(club_width, row_content_height),
                                        egui::Label::new(
                                            egui::RichText::new(
                                                swimmer_club,
                                            )
                                            .size(12.0)
                                            .strong()
                                            .color(egui::Color32::from_rgb(0, 212, 255)),
                                        )
                                        .truncate(),
                                    );
                                    ui.add_sized(
                                        egui::vec2(name_width, row_content_height),
                                        egui::Label::new(
                                            egui::RichText::new(
                                                swimmer_name,
                                            )
                                            .size(13.0)
                                            .strong()
                                            .color(egui::Color32::WHITE),
                                        )
                                        .truncate(),
                                    );
                                    ui.add_sized(
                                        egui::vec2(diff_width, row_content_height),
                                        egui::Label::new(egui::RichText::new(diff).monospace().size(11.0)),
                                    );
                                    ui.add_sized(
                                        egui::vec2(time_width, row_content_height),
                                        egui::Label::new(
                                            egui::RichText::new(split_time).monospace().strong().size(12.0),
                                        ),
                                    );
                                    ui.add_sized(
                                        egui::vec2(warning_width, row_content_height),
                                        egui::Label::new(if warning { "❗" } else { "" }),
                                    );
                                    if ui
                                        .add_sized(
                                            egui::vec2(action_width, 16.0),
                                            egui::Button::new("Doplnit"),
                                        )
                                        .clicked()
                                        && self.connected
                                        && !self.finished.contains(&lane)
                                    {
                                        let next_order = order.saturating_add(1);
                                        if next_order <= 99 {
                                            self.orders.insert(lane, next_order);
                                            self.log(
                                                "EVENT",
                                                "MANUAL_SPLIT_SKIP",
                                                &format!("Dráha {lane}; mezičas {next_order}"),
                                            );
                                        }
                                    }
                                });

                                ui.add_space(0.0);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    let stats_width = 92.0;
                                    let progress_area_width = (content_width
                                        - action_width
                                        - warning_width
                                        - column_gap * 2.0)
                                        .max(100.0);
                                    let (track, _) = ui.allocate_exact_size(
                                        egui::vec2((progress_area_width - stats_width - 6.0).max(60.0), 5.0),
                                        egui::Sense::hover(),
                                    );
                                    let inactive = egui::Color32::from_rgb(55, 68, 81);
                                    let active = egui::Color32::from_rgb(0, 210, 157);
                                    ui.painter().rect_filled(track, 4.0, inactive);
                                    let active_width =
                                        track.width() * completed_splits as f32 / target as f32;
                                    if active_width > 0.0 {
                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_size(
                                                track.min,
                                                egui::vec2(active_width, track.height()),
                                            ),
                                            4.0,
                                            active,
                                        );
                                    }
                                    ui.painter().rect_stroke(
                                        track,
                                        4.0,
                                        egui::Stroke::new(
                                            1.0_f32,
                                            egui::Color32::from_rgb(100, 116, 132),
                                        ),
                                        egui::StrokeKind::Inside,
                                    );
                                    let divider = egui::Color32::from_rgb(132, 147, 160);
                                    for split in 1..target {
                                        let x = track.left()
                                            + track.width() * split as f32 / target as f32;
                                        let slant = track.height() * 0.18;
                                        ui.painter().line_segment(
                                            [
                                                egui::pos2(x - slant, track.bottom()),
                                                egui::pos2(x + slant, track.top()),
                                            ],
                                            egui::Stroke::new(1.0_f32, divider),
                                        );
                                    }
                                    ui.add_sized(
                                        egui::vec2(stats_width, 12.0),
                                        egui::Label::new(
                                            egui::RichText::new(format!(
                                                "{completed_splits}/{target}  ·  {completed_distance} m"
                                            ))
                                            .monospace()
                                            .size(10.0)
                                            .strong()
                                            .color(egui::Color32::from_rgb(0, 220, 180)),
                                        ),
                                    );
                                });
                            });
                        ui.add_space(1.0);
                    }
                });
            });
    }
}
