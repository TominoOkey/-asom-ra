use super::*;

impl App {
    pub(super) fn remember_race_folder(&self, folder: &Path) {
        if let Some(path) = app_settings_path() {
            let _ = fs::write(path, folder.to_string_lossy().as_bytes());
        }
    }

    pub(super) fn persist_timing_log(&self) {
        let Some(folder) = &self.race_folder else {
            return;
        };
        if let Ok(bytes) = serde_json::to_vec_pretty(&self.timing_log) {
            let _ = fs::write(folder.join("casomira_timekeeper.json"), bytes);
        }
    }

    pub(super) fn load_race_folder(&mut self, folder: &Path) -> Result<(), String> {
        if self.heat_activity || self.timer_armed {
            return Err("Nejdřív resetujte aktivní rozplavbu, potom lze změnit závod.".into());
        }
        let missing: Vec<_> = RACE_FILES
            .iter()
            .filter(|name| !folder.join(name).is_file())
            .copied()
            .collect();
        if !missing.is_empty() {
            return Err(format!(
                "Neplatný závod. Ve složce chybí: {}",
                missing.join(", ")
            ));
        }

        let startlist_path = folder.join("startovka.txt");
        let startlist_text = read_text_file(&startlist_path)?;
        let (startlist, events) = parse_startlist(&startlist_text)
            .map_err(|error| format!("Neplatné rozlosování: {error}"))?;
        let discipline_map = read_discipline_map(&folder.join("discipliny_import.json"));
        if discipline_map.is_empty() {
            return Err(
                "Neplatný závod. Soubor discipliny_import.json neobsahuje platné disciplíny."
                    .into(),
            );
        }
        if events.is_empty() || startlist.is_empty() {
            return Err("Neplatný závod. Rozlosování neobsahuje žádné závodníky.".into());
        }

        self.race_folder = Some(folder.to_path_buf());
        self.timing_log = fs::read(folder.join("casomira_timekeeper.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        self.log_dir = folder.to_path_buf();
        self.event_name = folder
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        self.startlist = startlist;
        self.events = events;
        self.event_index = 0;
        self.discipline_map = discipline_map;
        self.race_selected = true;
        self.remember_race_folder(folder);
        if let Some((discipline, heat, _)) = self.events.first() {
            self.discipline = *discipline;
            self.heat_no = *heat;
        }
        if let Some((distance, _)) = self.discipline_map.get(&self.discipline) {
            self.splits_target = split_count_for_distance(*distance).unwrap_or(1);
        }
        self.heat = Heat::new(self.discipline, self.heat_no, self.splits_target);
        self.clear_heat_state();
        self.ensure_logs();
        self.status = format!("Načten závod: {}", self.event_name);
        self.log("EVENT", "RACE_OPENED", &folder.display().to_string());
        if self.connected {
            self.schedule_selection();
        }
        Ok(())
    }

    pub(super) fn create_new_race(&mut self) {
        if self.heat_activity || self.timer_armed {
            self.status =
                "Nejdřív resetujte aktivní rozplavbu, potom lze vytvořit nový závod.".into();
            return;
        }
        let Some(parent) = rfd::FileDialog::new()
            .set_title("Vyberte umístění nového závodu")
            .pick_folder()
        else {
            return;
        };
        let folder = parent.join(format!(
            "zavod-{}",
            chrono_free_timestamp().replace([':', ' '], "-")
        ));
        if let Err(error) = fs::create_dir_all(&folder) {
            self.status = format!("Nový závod nelze vytvořit: {error}");
            return;
        }
        self.race_folder = Some(folder.clone());
        self.remember_race_folder(&folder);
        self.race_selected = false;
        self.log_dir = folder.clone();
        self.event_name = folder
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        self.ensure_logs();
        self.status = "Nový závod: vyberte disciplíny a rozlosování".into();

        if let Some(path) = rfd::FileDialog::new()
            .set_title("Import disciplín")
            .add_filter("CSV / text", &["csv", "txt"])
            .pick_file()
        {
            self.import_disciplines(&path);
        }
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Import rozlosování / startovní listiny")
            .add_filter("CSV / text", &["csv", "txt"])
            .pick_file()
        {
            self.import_startlist(&path);
        }
        match self.load_race_folder(&folder) {
            Ok(()) => {}
            Err(error) => {
                self.race_selected = false;
                self.status = error;
            }
        }
    }
}

fn app_settings_path() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .map(|home| home.join(".casomira_last_race"))
}

pub(super) fn remembered_race_folder() -> Option<PathBuf> {
    let value = fs::read_to_string(app_settings_path()?).ok()?;
    let folder = PathBuf::from(value.trim());
    folder.is_dir().then_some(folder)
}
