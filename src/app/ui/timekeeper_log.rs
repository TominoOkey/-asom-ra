use super::*;

pub(super) fn show(app: &App, ctx: &egui::Context, log_width: f32) {
    egui::SidePanel::right("timekeeper-log")
        .exact_width(log_width)
        .resizable(false)
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgb(9, 14, 21))
                .inner_margin(10),
        )
        .show(ctx, |ui| {
            ui.heading(egui::RichText::new("LOG ČASOMĚŘIČE").size(17.0).strong());
            ui.separator();
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if app.timing_log.is_empty() {
                        ui.colored_label(
                            egui::Color32::from_rgb(135, 150, 164),
                            "Záznamy se zobrazí po zahájení rozplavby.",
                        );
                    }
                    for heat in &app.timing_log {
                        ui.add_space(5.0);
                        ui.colored_label(
                            egui::Color32::from_rgb(0, 212, 255),
                            format!("====== {} / {} ======", heat.discipline, heat.heat),
                        );
                        if heat.started {
                            ui.label(
                                egui::RichText::new(format!("Zahájena: {}", heat.title)).strong(),
                            );
                        }
                        for (split_index, (split, lane_times)) in heat.splits.iter().enumerate() {
                            ui.add_space(if split_index == 0 { 2.0 } else { 10.0 });
                            let mut ordered: Vec<_> = lane_times.iter().collect();
                            ordered.sort_by_key(|(_, time)| **time);
                            for (rank, (lane, time_ms)) in ordered.into_iter().enumerate() {
                                ui.monospace(format!(
                                    "{}  {}  {} - {} m",
                                    rank + 1,
                                    lane,
                                    format_monitor_time(*time_ms),
                                    split * 50
                                ));
                            }
                        }
                        ui.separator();
                    }
                });
        });
}
