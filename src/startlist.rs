//! Parsers for meet start lists and discipline imports.

use crate::split_count_for_distance;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Swimmer {
    pub name: String,
    pub club: String,
    pub entry: String,
    pub entry_centiseconds: u32,
}

pub type StartList = BTreeMap<(u16, u8, u8), Swimmer>;

fn csv_semicolon_rows(text: &str) -> Vec<Vec<String>> {
    csv_delimited_rows(text, ';')
}

fn csv_delimited_rows(text: &str, delimiter: char) -> Vec<Vec<String>> {
    // Python's csv.reader behavior for the source formats, including quoted semicolons.
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut quoted = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            c if c == delimiter && !quoted => {
                row.push(std::mem::take(&mut field));
            }
            '\n' if !quoted => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            '\r' if !quoted => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            _ => field.push(ch),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

/// Parse the semicolon delimited meet export, whose first 19 rows are metadata.
pub fn parse_startlist(text: &str) -> Result<(StartList, Vec<(u16, u8, String)>), String> {
    let mut swimmers = StartList::new();
    let mut events = Vec::new();
    let mut seen = BTreeSet::new();
    for row in csv_semicolon_rows(text).into_iter().skip(19) {
        if row.len() < 12 {
            continue;
        }
        let Ok(discipline) = row[0].trim().parse::<u16>() else {
            continue;
        };
        let Ok(heat) = row[10].trim().parse::<u8>() else {
            continue;
        };
        let Ok(lane) = row[11].trim().parse::<u8>() else {
            continue;
        };
        let key = (discipline, heat);
        if seen.insert(key) {
            events.push((discipline, heat, format_discipline_name(&row[1], &row[6])));
        }
        let entry = row[8].trim().to_string();
        swimmers.insert(
            (discipline, heat, lane),
            Swimmer {
                name: format!("{} {}", row[4].trim().to_uppercase(), row[3].trim()),
                club: row[7].trim().to_uppercase(),
                entry_centiseconds: parse_time_centiseconds(&entry),
                entry,
            },
        );
    }
    if swimmers.is_empty() {
        return Err("Soubor neobsahuje rozpoznatelnou startovní listinu s rozplavbami.".into());
    }
    Ok((swimmers, events))
}

pub fn parse_time_centiseconds(text: &str) -> u32 {
    if !text.contains(':') && !text.contains(',') {
        return 999_999;
    }
    let normalized = text.replace(',', ":");
    let parts: Vec<_> = normalized.split(':').collect();
    match parts.as_slice() {
        [s, cs] => s
            .parse::<u32>()
            .ok()
            .zip(cs.parse::<u32>().ok())
            .map(|(s, c)| s * 100 + c)
            .unwrap_or(999_999),
        [m, s, cs] => m
            .parse::<u32>()
            .ok()
            .zip(s.parse::<u32>().ok())
            .zip(cs.parse::<u32>().ok())
            .map(|((m, s), c)| m * 6000 + s * 100 + c)
            .unwrap_or(999_999),
        _ => 999_999,
    }
}

pub fn format_discipline_name(code: &str, gender: &str) -> String {
    let distance: String = code.chars().filter(|c| c.is_ascii_digit()).collect();
    let stroke: String = code.chars().filter(|c| c.is_alphabetic()).collect();
    let stroke = match stroke.as_str() {
        "VZ" => "Volný způsob",
        "O" => "Polohový závod",
        "M" => "Motýlek",
        "Z" => "Znak",
        "P" => "Prsa",
        _ => &stroke,
    };
    let gender = match gender {
        "M" => "Muži",
        "Z" => "Ženy",
        _ => gender,
    };
    format!("{distance}m {stroke} | {gender}")
}

#[derive(Debug, Clone, Default)]
pub struct DisciplineMap {
    pub distances: HashMap<u16, u16>,
    pub labels: HashMap<u16, String>,
}

/// Parse the comma-delimited discipline import format used by the Python UI.
pub fn parse_discipline_import(text: &str) -> (DisciplineMap, usize) {
    let mut result = DisciplineMap::default();
    let mut unsupported = 0;
    for row in csv_delimited_rows(text, ',') {
        if row.len() < 5 {
            continue;
        }
        let Ok(number) = row[2].trim().parse::<u16>() else {
            continue;
        };
        let digits: String = row[4]
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let Some(distance) = digits.parse::<u16>().ok() else {
            unsupported += 1;
            continue;
        };
        if split_count_for_distance(distance).is_none() {
            unsupported += 1;
            continue;
        }
        result.distances.insert(number, distance);
        result.labels.insert(number, row[3].trim().into());
    }
    (result, unsupported)
}
