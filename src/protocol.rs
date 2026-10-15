//! Timer wire protocol and state model for one race heat.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const BAUD_RATE: u32 = 9_600;
pub const OUTPUT_PORTS: [&str; 1] = ["COM19"];
pub const SPLIT_OPTIONS: [(u8, &str); 6] = [
    (1, "25/50 m"),
    (2, "100 m"),
    (4, "200 m"),
    (8, "400 m"),
    (16, "800 m"),
    (30, "1500 m"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerError {
    InvalidAction(u8),
    InvalidDiscipline(u16),
    InvalidHeat(u8),
    InvalidLane(u8),
    InvalidOrder(u8),
    NegativeTime,
    TimeTooLarge,
    MalformedRecord(String),
}

impl fmt::Display for TimerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAction(n) => write!(f, "Typ akce musí být 1 až 4 (bylo {n})."),
            Self::InvalidDiscipline(n) => write!(f, "Disciplína musí být 1–999 (bylo {n})."),
            Self::InvalidHeat(n) => write!(f, "Rozplavba musí být 1–99 (bylo {n})."),
            Self::InvalidLane(n) => write!(f, "Dráha musí být 0–9 (bylo {n})."),
            Self::InvalidOrder(n) => write!(f, "Pořadí musí být 0–99 (bylo {n})."),
            Self::NegativeTime => write!(f, "Čas nesmí být záporný."),
            Self::TimeTooLarge => write!(f, "Čas přesahuje limit 99:59.99."),
            Self::MalformedRecord(s) => write!(f, "Neplatný 15znakový záznam: {s}"),
        }
    }
}

impl std::error::Error for TimerError {}

/// One decoded timer message, expressed in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    pub action: u8,
    pub discipline: u16,
    pub heat: u8,
    pub lane: u8,
    pub order: u8,
    pub time_ms: u32,
}

impl Record {
    pub fn encode(self) -> Result<String, TimerError> {
        if !(1..=4).contains(&self.action) {
            return Err(TimerError::InvalidAction(self.action));
        }
        if !(1..=999).contains(&self.discipline) {
            return Err(TimerError::InvalidDiscipline(self.discipline));
        }
        if !(1..=99).contains(&self.heat) {
            return Err(TimerError::InvalidHeat(self.heat));
        }
        if self.lane > 9 {
            return Err(TimerError::InvalidLane(self.lane));
        }
        if self.order > 99 {
            return Err(TimerError::InvalidOrder(self.order));
        }
        let cs = self.time_ms / 10;
        let minutes = cs / 6000;
        if minutes > 99 {
            return Err(TimerError::TimeTooLarge);
        }
        let seconds = (cs % 6000) / 100;
        let hundredths = cs % 100;
        Ok(format!(
            "{}{:03}{:02}{}{:02}{:02}{:02}{:02}",
            self.action,
            self.discipline,
            self.heat,
            self.lane,
            self.order,
            minutes,
            seconds,
            hundredths
        ))
    }

    pub fn decode(s: &str) -> Result<Self, TimerError> {
        if s.len() != 15 || !s.bytes().all(|b| b.is_ascii_digit()) {
            return Err(TimerError::MalformedRecord(s.to_owned()));
        }
        let n = |range: std::ops::Range<usize>| {
            s[range]
                .parse::<u32>()
                .map_err(|_| TimerError::MalformedRecord(s.to_owned()))
        };
        let record = Self {
            action: n(0..1)? as u8,
            discipline: n(1..4)? as u16,
            heat: n(4..6)? as u8,
            lane: n(6..7)? as u8,
            order: n(7..9)? as u8,
            time_ms: ((n(9..11)? * 6000 + n(11..13)? * 100 + n(13..15)?) * 10),
        };
        // Reuse the canonical validation logic.
        record.encode()?;
        Ok(record)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerInput {
    Start,
    Split { lane_mask: u8, raw_time: u32 },
}

impl TimerInput {
    pub fn parse(line: &str) -> Option<Self> {
        let mut fields = line.trim().split(';');
        let mask = fields.next()?.trim().parse::<u16>().ok()?;
        let raw_time = fields.next()?.trim().parse::<u32>().ok()?;
        if fields.next().is_some() || mask > 255 {
            return None;
        }
        if mask == 255 && raw_time == 255 {
            Some(Self::Start)
        } else {
            Some(Self::Split {
                lane_mask: mask as u8,
                raw_time,
            })
        }
    }

    pub fn time_ms(self) -> Option<u32> {
        match self {
            Self::Start => None,
            Self::Split { raw_time: 255, .. } => Some(255),
            Self::Split { raw_time, .. } => Some(raw_time.saturating_mul(10)),
        }
    }
}

pub fn split_count_for_distance(distance: u16) -> Option<u8> {
    match distance {
        25 | 50 => Some(1),
        100 => Some(2),
        200 => Some(4),
        400 => Some(8),
        800 => Some(16),
        1500 => Some(30),
        _ => None,
    }
}

pub fn format_short_time(time_ms: Option<u32>) -> String {
    let Some(ms) = time_ms else {
        return "—".into();
    };
    let cs = ms / 10;
    let minutes = cs / 6000;
    let seconds = (cs % 6000) / 100;
    let hundredths = cs % 100;
    if minutes > 0 {
        format!("{minutes}:{seconds:02},{hundredths:02}")
    } else {
        format!("{seconds:02},{hundredths:02}")
    }
}

/// Stateful race model, independent from the GUI and serial device.
#[derive(Debug, Clone)]
pub struct Heat {
    pub discipline: u16,
    pub heat: u8,
    pub splits_target: u8,
    pub started: bool,
    pub orders: BTreeMap<u8, u8>,
    pub last_times: BTreeMap<u8, u32>,
    pub previous_times: BTreeMap<u8, u32>,
    pub finished_lanes: BTreeSet<u8>,
}

impl Heat {
    pub fn new(discipline: u16, heat: u8, splits_target: u8) -> Self {
        Self {
            discipline,
            heat,
            splits_target,
            started: false,
            orders: BTreeMap::new(),
            last_times: BTreeMap::new(),
            previous_times: BTreeMap::new(),
            finished_lanes: BTreeSet::new(),
        }
    }

    pub fn reset(&mut self, discipline: u16, heat: u8) {
        self.discipline = discipline;
        self.heat = heat;
        self.started = false;
        self.orders.clear();
        self.last_times.clear();
        self.previous_times.clear();
        self.finished_lanes.clear();
    }

    /// Apply a timer input and return protocol records emitted by the original application.
    pub fn apply(&mut self, input: TimerInput) -> Vec<Record> {
        match input {
            TimerInput::Start => {
                self.started = true;
                self.orders.clear();
                self.last_times.clear();
                self.previous_times.clear();
                self.finished_lanes.clear();
                vec![Record {
                    action: 2,
                    discipline: self.discipline,
                    heat: self.heat,
                    lane: 0,
                    order: 0,
                    time_ms: 0,
                }]
            }
            TimerInput::Split {
                lane_mask,
                raw_time,
            } => {
                let time_ms = if raw_time == 255 {
                    255
                } else {
                    raw_time.saturating_mul(10)
                };
                let mut records = Vec::new();
                for lane in 1..=8 {
                    if lane_mask & (1 << (lane - 1)) == 0 || self.finished_lanes.contains(&lane) {
                        continue;
                    }
                    let order = self
                        .orders
                        .get(&lane)
                        .copied()
                        .unwrap_or(0)
                        .saturating_add(1);
                    if order > 99 {
                        continue;
                    }
                    self.orders.insert(lane, order);
                    if let Some(previous) = self.last_times.get(&lane).copied() {
                        self.previous_times.insert(lane, previous);
                    }
                    self.last_times.insert(lane, time_ms);
                    let action = if order >= self.splits_target {
                        self.finished_lanes.insert(lane);
                        4
                    } else {
                        3
                    };
                    records.push(Record {
                        action,
                        discipline: self.discipline,
                        heat: self.heat,
                        lane,
                        order,
                        time_ms,
                    });
                }
                records
            }
        }
    }

    pub fn segment_ms(&self, lane: u8) -> Option<u32> {
        let current = *self.last_times.get(&lane)?;
        Some(current.saturating_sub(self.previous_times.get(&lane).copied().unwrap_or(0)))
    }

    pub fn finish_order(&self) -> Vec<u8> {
        let mut lanes: Vec<_> = self.finished_lanes.iter().copied().collect();
        lanes.sort_by_key(|lane| {
            (
                self.last_times.get(lane).copied().unwrap_or(u32::MAX),
                *lane,
            )
        });
        lanes
    }
}
