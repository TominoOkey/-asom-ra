use super::*;

impl App {
    pub(super) fn import_startlist(&mut self, path: &Path) {
        match read_text_file(path).and_then(|text| {
            parse_startlist(&text).map_err(|error| format!("Neplatné rozlosování: {error}"))
        }) {
            Ok((swimmers, events)) => {
                let destination = self.log_dir.join("startovka.txt");
                if path != destination
                    && let Err(error) = fs::copy(path, &destination)
                {
                    self.status = format!("Nelze uložit rozlosování do závodu: {error}");
                    return;
                }
                self.startlist = swimmers;
                self.events = events;
                self.event_index = self
                    .events
                    .iter()
                    .position(|(discipline, heat, _)| {
                        *discipline == self.discipline && *heat == self.heat_no
                    })
                    .unwrap_or(0);
                self.status = format!("Načteno {} závodníků", self.startlist.len());
            }
            Err(error) => self.status = error,
        }
    }

    pub(super) fn import_disciplines(&mut self, path: &Path) {
        match read_text_file(path) {
            Ok(text) => {
                let (parsed, unsupported) = parse_discipline_import(&text);
                if parsed.distances.is_empty() {
                    self.status = "Soubor neobsahuje rozpoznatelné disciplíny".into();
                    return;
                }
                self.discipline_map = parsed
                    .distances
                    .iter()
                    .map(|(n, d)| (*n, (*d, parsed.labels.get(n).cloned().unwrap_or_default())))
                    .collect();
                if let Err(e) = fs::write(
                    self.log_dir.join("discipliny_import.json"),
                    serialize_disciplines(&self.discipline_map),
                ) {
                    self.log_error = Some(e.to_string());
                }
                if let Some((distance, _)) = self.discipline_map.get(&self.discipline) {
                    self.splits_target =
                        split_count_for_distance(*distance).unwrap_or(self.splits_target);
                    self.heat.splits_target = self.splits_target;
                }
                self.status = format!(
                    "Načteno {} disciplín{}",
                    self.discipline_map.len(),
                    if unsupported > 0 {
                        format!("; přeskočeno {unsupported} nepodporovaných")
                    } else {
                        String::new()
                    }
                );
                self.log(
                    "EVENT",
                    "DISCIPLINES_IMPORTED",
                    &format!("{}; načteno {}", path.display(), self.discipline_map.len()),
                );
            }
            Err(e) => self.status = format!("Nelze načíst disciplíny: {e}"),
        }
    }
}
