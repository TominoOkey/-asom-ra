use super::*;

impl App {
    fn heat_position(&self, discipline: u16, heat: u8) -> (usize, usize) {
        let matching_heats: Vec<_> = self
            .events
            .iter()
            .filter(|(event_discipline, _, _)| *event_discipline == discipline)
            .collect();
        let index = matching_heats
            .iter()
            .position(|(_, event_heat, _)| *event_heat == heat)
            .map_or(0, |index| index + 1);
        (index, matching_heats.len())
    }

    pub(super) fn overlay_data(&self) -> OverlayData {
        let discipline_name = self
            .events
            .iter()
            .find(|(d, h, _)| *d == self.discipline && *h == self.heat_no)
            .map(|(_, _, name)| name.clone())
            .unwrap_or_else(|| "PLAVECKÉ ZÁVODY".into());
        let started = self.started_at.is_some();
        let (heat_index, heat_count) = self.heat_position(self.discipline, self.heat_no);
        let race_lanes = self.race_lanes();
        let mut lanes = Vec::new();
        for lane in 1..=8 {
            let swimmer = self.startlist.get(&(self.discipline, self.heat_no, lane));
            let active = if self.results_mode {
                race_lanes.contains(&lane) && self.finished.contains(&lane)
            } else {
                swimmer.is_some()
                    || self.orders.get(&lane).copied().unwrap_or(0) > 0
                    || self.finished.contains(&lane)
            };
            if !active {
                continue;
            }
            let time = self.latest.get(&lane).copied();
            let rank = time.map(|time| {
                race_lanes
                    .iter()
                    .filter(|other| {
                        self.finished.contains(other)
                            && (self.latest.get(other).copied().unwrap_or(u32::MAX), **other)
                                < (time, lane)
                    })
                    .count()
                    + 1
            });
            lanes.push(OverlayLane {
                lane,
                name: swimmer.map(|s| s.name.clone()).unwrap_or_default(),
                club: swimmer.map(|s| s.club.clone()).unwrap_or_default(),
                entry: swimmer.map(|s| s.entry.clone()).unwrap_or_default(),
                time,
                rank,
                swimming: started
                    && !self.finished.contains(&lane)
                    && self.orders.get(&lane).copied().unwrap_or(0) > 0,
                personal_record: self.results_mode
                    && time.is_some_and(|time| {
                        swimmer.is_some_and(|s| {
                            s.entry_centiseconds < 999_999
                                && time < s.entry_centiseconds.saturating_mul(10)
                        })
                    }),
            });
        }
        if self.results_mode {
            lanes.sort_by_key(|lane| {
                (
                    lane.rank.is_none(),
                    lane.rank.unwrap_or(usize::MAX),
                    lane.lane,
                )
            });
        } else {
            lanes.sort_by_key(|lane| lane.lane);
        }
        OverlayData {
            event_name: self.event_name.clone(),
            discipline_name,
            discipline: self.discipline,
            heat: self.heat_no,
            heat_index,
            heat_count,
            elapsed_ms: self
                .started_at
                .map(|s| s.elapsed().as_millis() as u32)
                .or(self.race_elapsed_ms),
            results_mode: self.results_mode,
            is_next: false,
            lanes,
        }
    }

    pub(super) fn presentation_overlay_data(&self) -> OverlayData {
        // Before a heat begins, keep its selected start list on NEXT. Once
        // racing has started, advance to the next populated heat and skip
        // empty entries in the imported start list.
        let current_index = self.events.iter().position(|(discipline, heat, _)| {
            *discipline == self.discipline && *heat == self.heat_no
        });
        let current_index = current_index.unwrap_or(self.event_index);
        let search_from = if self.heat_activity {
            current_index.saturating_add(1)
        } else {
            current_index
        };
        self.events
            .iter()
            .enumerate()
            .skip(search_from)
            .find(|(_, (discipline, heat, _))| {
                (1..=8).any(|lane| self.startlist.contains_key(&(*discipline, *heat, lane)))
            })
            .map(|(_, (discipline, heat, name))| {
                let lanes = (1..=8)
                    .filter_map(|lane| {
                        self.startlist
                            .get(&(*discipline, *heat, lane))
                            .map(|swimmer| OverlayLane {
                                lane,
                                name: swimmer.name.clone(),
                                club: swimmer.club.clone(),
                                entry: swimmer.entry.clone(),
                                time: None,
                                rank: None,
                                swimming: false,
                                personal_record: false,
                            })
                    })
                    .collect();
                let (heat_index, heat_count) = self.heat_position(*discipline, *heat);
                OverlayData {
                    event_name: self.event_name.clone(),
                    discipline_name: name.clone(),
                    discipline: *discipline,
                    heat: *heat,
                    heat_index,
                    heat_count,
                    elapsed_ms: None,
                    results_mode: false,
                    is_next: true,
                    lanes,
                }
            })
            .unwrap_or_else(|| OverlayData {
                event_name: self.event_name.clone(),
                discipline_name: "NÁSLEDUJE".into(),
                discipline: 0,
                heat: 0,
                heat_index: 0,
                heat_count: 0,
                elapsed_ms: None,
                results_mode: false,
                is_next: true,
                lanes: Vec::new(),
            })
    }
}
