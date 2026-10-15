//! File formats and paths used by a race folder.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(super) const RACE_FILES: [&str; 6] = [
    "casomira.log",
    "casomira_hardware_vstup.log",
    "casomira_do_casomiry.log",
    "casomira_do_com.log",
    "discipliny_import.json",
    "startovka.txt",
];

pub(super) fn read_discipline_map(path: &Path) -> BTreeMap<u16, (u16, String)> {
    #[derive(serde::Deserialize)]
    struct Entry {
        distance: u16,
        #[serde(default)]
        label: String,
    }
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    let Ok(map) = serde_json::from_str::<BTreeMap<String, Entry>>(&text) else {
        return BTreeMap::new();
    };
    map.into_iter()
        .filter_map(|(key, value)| {
            let number = key.parse::<u16>().ok()?;
            casomira::split_count_for_distance(value.distance)?;
            Some((number, (value.distance, value.label)))
        })
        .collect()
}

pub(super) fn read_text_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("Nelze načíst soubor: {error}"))?;
    Ok(match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => encoding_rs::WINDOWS_1250
            .decode(error.as_bytes())
            .0
            .into_owned(),
    })
}

pub(super) fn serialize_disciplines(map: &BTreeMap<u16, (u16, String)>) -> String {
    let mut out = String::from("{\n");
    for (idx, (number, (distance, label))) in map.iter().enumerate() {
        let escaped = label.replace('\\', "\\\\").replace('"', "\\\"");
        out.push_str(&format!(
            "  \"{number}\": {{\"distance\": {distance}, \"label\": \"{escaped}\"}}{}\n",
            if idx + 1 == map.len() { "" } else { "," }
        ));
    }
    out.push_str("}\n");
    out
}
