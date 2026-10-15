//! Domain logic for the swimming meet timer.
//!
//! The original application exchanged fixed-width, 15 byte ASCII records with
//! timing hardware. This crate keeps that wire format stable while exposing
//! typed data structures to the desktop application.

mod protocol;
mod startlist;

pub use protocol::{
    BAUD_RATE, Heat, OUTPUT_PORTS, Record, SPLIT_OPTIONS, TimerError, TimerInput,
    format_short_time, split_count_for_distance,
};
pub use startlist::{
    DisciplineMap, StartList, Swimmer, format_discipline_name, parse_discipline_import,
    parse_startlist, parse_time_centiseconds,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_wire_format_round_trip() {
        let record = Record {
            action: 4,
            discipline: 123,
            heat: 7,
            lane: 8,
            order: 30,
            time_ms: 99_599,
        };
        let encoded = record.encode().unwrap();
        assert_eq!(encoded, "412307830013959");
        let rounded = Record {
            time_ms: 99_590,
            ..record
        };
        assert_eq!(Record::decode(&encoded).unwrap(), rounded);
    }

    #[test]
    fn timer_special_start_and_split_units_match_python() {
        assert_eq!(TimerInput::parse("255;255"), Some(TimerInput::Start));
        assert_eq!(TimerInput::parse("1;255").unwrap().time_ms(), Some(255));
        assert_eq!(TimerInput::parse("1;12").unwrap().time_ms(), Some(120));
        assert_eq!(TimerInput::parse("1;256").unwrap().time_ms(), Some(2560));
    }

    #[test]
    fn heat_emits_ordered_split_and_finish_records() {
        let mut heat = Heat::new(10, 2, 2);
        let first = heat.apply(TimerInput::Split {
            lane_mask: 3,
            raw_time: 100,
        });
        assert_eq!(first.iter().map(|r| r.action).collect::<Vec<_>>(), [3, 3]);
        let finish = heat.apply(TimerInput::Split {
            lane_mask: 3,
            raw_time: 200,
        });
        assert_eq!(finish.iter().map(|r| r.action).collect::<Vec<_>>(), [4, 4]);
        assert_eq!(heat.finish_order(), [1, 2]);
    }

    #[test]
    fn time_and_discipline_helpers() {
        assert_eq!(parse_time_centiseconds("1:23,45"), 8_345);
        assert_eq!(format_short_time(Some(83_450)), "1:23,45");
        assert_eq!(split_count_for_distance(1500), Some(30));
        assert_eq!(
            format_discipline_name("100VZ", "M"),
            "100m Volný způsob | Muži"
        );
    }

    #[test]
    fn protocol_validates_field_limits_and_truncates_milliseconds() {
        let record = Record {
            action: 1,
            discipline: 1,
            heat: 1,
            lane: 0,
            order: 0,
            time_ms: 19,
        };
        assert_eq!(record.encode().unwrap(), "100101000000001");
        assert!(
            Record {
                discipline: 1000,
                ..record
            }
            .encode()
            .is_err()
        );
        assert!(
            Record {
                time_ms: 6_000_000,
                ..record
            }
            .encode()
            .is_err()
        );
        assert!(Record::decode("10010100000000x").is_err());
    }

    #[test]
    fn selection_reset_record_matches_hardware_protocol() {
        let record = Record {
            action: 1,
            discipline: 1,
            heat: 1,
            lane: 0,
            order: 0,
            time_ms: 0,
        };
        let frame = record.encode().unwrap();
        assert_eq!(frame, "100101000000000");
        assert_eq!(frame.len(), 15);
    }

    #[test]
    fn discipline_import_uses_csv_quoting_and_skips_unsupported_distances() {
        let text = "a,b,12,\"100m Freestyle, Women\",100 meters\na,b,13,Relay,4x50\n";
        let (map, unsupported) = parse_discipline_import(text);
        assert_eq!(map.distances.get(&12), Some(&100));
        assert_eq!(
            map.labels.get(&12).map(String::as_str),
            Some("100m Freestyle, Women")
        );
        assert_eq!(unsupported, 1);
    }
}
