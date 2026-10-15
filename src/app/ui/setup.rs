use super::super::*;

impl App {
    pub(super) fn race_setup_screen(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(12, 18, 27))
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space((ui.available_height() * 0.16).max(20.0));
                    ui.heading(egui::RichText::new("ČASOMÍRA").size(30.0).strong());
                    ui.add_space(6.0);
                    ui.label("Začněte výběrem složky závodu nebo vytvořením nové.");
                    ui.add_space(22.0);
                    ui.horizontal(|ui| {
                        if ui.button("Otevřít složku závodu").clicked()
                            && let Some(folder) = rfd::FileDialog::new().pick_folder()
                        {
                            match self.load_race_folder(&folder) {
                                Ok(()) => {}
                                Err(error) => self.status = error,
                            }
                        }
                        if ui.button("Vytvořit nový závod").clicked() {
                            self.create_new_race();
                        }
                    });
                    if let Some(folder) = self.race_folder.clone() {
                        ui.add_space(16.0);
                        ui.label(format!("Pracovní složka: {}", folder.display()));
                        ui.horizontal(|ui| {
                            if ui.button("Importovat disciplíny").clicked()
                                && let Some(path) = rfd::FileDialog::new()
                                    .add_filter("CSV / text", &["csv", "txt"])
                                    .pick_file()
                            {
                                self.import_disciplines(&path);
                            }
                            if ui.button("Importovat rozlosování").clicked()
                                && let Some(path) = rfd::FileDialog::new()
                                    .add_filter("CSV / text", &["csv", "txt"])
                                    .pick_file()
                            {
                                self.import_startlist(&path);
                            }
                            if ui.button("Otevřít tento závod").clicked() {
                                if let Err(error) = self.load_race_folder(&folder) {
                                    self.status = error;
                                }
                            }
                        });
                    }
                    ui.add_space(18.0);
                    ui.label(
                        egui::RichText::new(
                            "Platný závod obsahuje 4 logy, discipliny_import.json a startovka.txt.",
                        )
                        .color(egui::Color32::from_rgb(145, 163, 180)),
                    );
                    ui.add_space(12.0);
                    let status_color = if self.status.starts_with("Neplatný závod")
                        || self.status.contains("nelze")
                    {
                        egui::Color32::from_rgb(248, 113, 113)
                    } else if self.status.starts_with("Nový závod:") {
                        egui::Color32::from_rgb(251, 191, 36)
                    } else {
                        egui::Color32::from_rgb(165, 180, 196)
                    };
                    ui.label(egui::RichText::new(&self.status).color(status_color));
                });
            });
    }
}
