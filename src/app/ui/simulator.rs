use super::super::*;

pub(super) fn show(ctx: &egui::Context, app: &mut App) {
    egui::Window::new("Simulátor časomíry")
        .default_pos([20.0, 500.0])
        .show(ctx, |ui| {
            let elapsed = app
                .simulator_started
                .map(|s| s.elapsed().as_millis() as u32)
                .unwrap_or(0);
            ui.horizontal(|ui| {
                ui.heading(format!(
                    "{:02}:{:02}.{:02}",
                    elapsed / 60000,
                    (elapsed / 1000) % 60,
                    (elapsed / 10) % 100
                ));
                if app.simulator_started.is_none()
                    && ui
                        .add_enabled(app.connected, egui::Button::new("Start závodu"))
                        .clicked()
                {
                    app.process_input("255;255".into());
                    app.simulator_started = Some(Instant::now());
                    app.simulator_last = "Závod běží — stiskněte tlačítko dráhy".into();
                }
            });
            ui.horizontal_wrapped(|ui| {
                for lane in 1..=8 {
                    if ui
                        .add_enabled(
                            app.simulator_started.is_some(),
                            egui::Button::new(format!("Dráha {lane}")),
                        )
                        .clicked()
                    {
                        let elapsed = app
                            .simulator_started
                            .map(|s| s.elapsed().as_millis() as u32)
                            .unwrap_or_default();
                        let mut raw = elapsed / 10;
                        if raw == 255 {
                            raw = 256;
                        }
                        let line = format!("{};{}", 1u16 << (lane - 1), raw);
                        app.process_input(line);
                        app.simulator_last = format!(
                            "Dráha {lane} — zaznamenáno v {}",
                            format_short_time(Some(elapsed))
                        );
                    }
                }
            });
            ui.label(&app.simulator_last);
            if app.simulator_started.is_some() && ui.button("Reset simulátoru").clicked() {
                app.simulator_started = None;
                app.simulator_last.clear();
            }
        });
}
