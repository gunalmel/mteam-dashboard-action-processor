use crate::action_csv_row::ActionCsvRow;
use crate::scatter_points::CsvRowTime;
use chrono::{Datelike, TimeZone, Timelike, Utc};
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref ACTION_NAME_REGEX: Regex = Regex::new(r"^\s*\((\d+)\)\s*(.+?)\s*\(action\)\s*$").unwrap();
}
pub const ERROR_MARKER_TIME_THRESHOLD: u32 = 2;
pub fn parse_time(input: &str) -> Option<CsvRowTime> {
    // Split the input into hours, minutes, and seconds
    let parts: Vec<&str> = input.split(':').collect();
    if parts.len() != 3 {
        return None; // Input format is invalid
    }

    // Parse hours, minutes, and seconds
    let hours: u32 = parts[0].parse().ok()?;
    let minutes: u32 = parts[1].parse().ok()?;
    let seconds: u32 = parts[2].parse().ok()?;

    // Validate the ranges
    if minutes >= 60 || seconds >= 60 {
        return None; // Invalid time input
    }

    // Calculate total seconds
    let total_seconds = hours * 3600 + minutes * 60 + seconds;

    // Get today's UTC date
    let today = Utc::now();
    let date_string = format!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02}",
        today.year(),
        today.month(),
        today.day(),
        hours,
        minutes,
        seconds
    );

    // Format the input into HH:MM:SS
    let formatted_input = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);

    Some(CsvRowTime {
        total_seconds,
        date_string,
        timestamp: formatted_input,
    })
}
pub fn extract_stage_name(input: &str) -> Option<(u32, String)> {
    ACTION_NAME_REGEX.captures(input).and_then(|captures| {
        let number = captures.get(1)?.as_str().parse::<u32>().ok()?;
        let action_name = captures.get(2).map(|matched| {
            matched
                .as_str()
                .trim()
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
        })?;
        Some((number, action_name))
    })
}
pub fn is_action_row(csv_row: &ActionCsvRow) -> bool {
    csv_row.parsed_stage.is_some() &&
    !csv_row.subaction_time.trim().is_empty() && 
    !csv_row.subaction_name.is_empty()
}
pub fn is_stage_boundary(csv_row: &ActionCsvRow) -> bool {
    csv_row.parsed_stage.is_some() &&
    csv_row.subaction_time.trim().is_empty() && 
    csv_row.subaction_name.is_empty() && 
    csv_row.score.is_empty() && 
    csv_row.old_value.is_empty() && 
    csv_row.new_value.is_empty()
}
pub fn is_error_action_marker(csv_row: &ActionCsvRow) -> bool {
    csv_row.old_value.trim() == "Error-Triggered" && 
    csv_row.score.trim() == "Action-Was-Performed"
}
pub fn is_missed_action(csv_row: &ActionCsvRow) -> bool {
    csv_row.old_value.trim() == "Error-Triggered" && 
    csv_row.score.trim() == "Action-Was-Not-Performed"
}
pub fn can_mark_each_other(csv_row1: &ActionCsvRow, csv_row2: &ActionCsvRow) -> bool{
    let marker_time: u32 = csv_row1.timestamp.clone().unwrap_or(CsvRowTime::default()).total_seconds;
    let current_time: u32 = csv_row2.timestamp.clone().unwrap_or(CsvRowTime::default()).total_seconds;
    
    marker_time.abs_diff(current_time)<=ERROR_MARKER_TIME_THRESHOLD
}
pub fn is_erroneous_action(csv_row: &ActionCsvRow, error_marker_row: &ActionCsvRow) -> bool{
    // let marker_time: u32 = error_marker_row.timestamp.clone().unwrap_or(CsvRowTime::default()).total_seconds;
    // let current_time: u32 = csv_row.timestamp.clone().unwrap_or(CsvRowTime::default()).total_seconds;
    
    csv_row.action_point && error_marker_row.username == csv_row.action_vital_name && 
    can_mark_each_other(csv_row, error_marker_row)
    // marker_time.abs_diff(current_time)<=ERROR_MARKER_TIME_THRESHOLD
}

// pub fn process_row(csv_row: &ActionCsvRow, buffer: &Vec<ActionCsvRow>, max_time_diff: NaiveTime) -> Option<ActionPlotPoint> {
//    if csv_row.action_point{
//        return Some(ActionPlotPoint::Action(ActionPoint{})
//     }else if csv_row.stage_boundary{
//        return Some(ActionPlotPoint::StageBoundary(csv_row))
//     }else if csv_row.error_action_marker{
//        return Some(ActionPlotPoint::ErrorActionMarker(csv_row))
//     }else if csv_row.missed_action_marker{
//        return Some(ActionPlotPoint::MissedAction(csv_row))
//     }
// }
#[cfg(test)]
mod tests {
    mod test_parse_time {
        use super::super::*;

        #[test]
        fn valid_time() {
            let timestamp = "12:34:56";
            let parsed_time = parse_time(timestamp).unwrap();
            assert_eq!(parsed_time.timestamp, timestamp);
            assert_eq!(parsed_time.total_seconds, 12 * 3600 + 34 * 60 + 56);
            assert_eq!(parsed_time.date_string,"2024-12-24 12:34:56");
        }

        #[test]
        fn invalid_time() {
            let timestamp = "invalid time";
            let result = parse_time(timestamp);
            assert!(result.is_none());
        }

        #[test]
        fn empty_time() {
            let timestamp = "";
            let result = parse_time(timestamp);
            assert!(result.is_none());
        }
    }
    
    mod test_extract_stage_name {
        use super::super::*;

        #[test]
        fn valid() {
            let input = "(123)example(action)";
            let expected = Some((123, "example".to_string()));
            assert_eq!(extract_stage_name(input), expected);
        }

        #[test]
        fn no_action() {
            let input = "(123)example";
            let expected = None;
            assert_eq!(extract_stage_name(input), expected);
        }

        #[test]
        fn empty() {
            let input = "";
            let expected = None;
            assert_eq!(extract_stage_name(input), expected);
        }

        #[test]
        fn no_match() {
            let input = "example(action)";
            let expected = None;
            assert_eq!(extract_stage_name(input), expected);
        }

        #[test]
        fn with_spaces() {
            let input = "  (123)   example   with spaces   (action)   ";
            let expected = Some((123,"example with spaces".to_string()));
            assert_eq!(extract_stage_name(input), expected);
        }
    }

    mod test_is_action_row {
        use super::super::*;
        use crate::action_csv_row::ActionCsvRow;

        #[test]
        fn is_true() {
            let csv_row = ActionCsvRow {
                parsed_stage: Some((1,"Action".to_string())),
                subaction_time: "12:34".to_string(),
                subaction_name: "SubAction".to_string(),
                ..Default::default()
            };
            assert!(is_action_row(&csv_row));
        }

        #[test]
        fn is_false_no_parsed_action_name() {
            let csv_row = ActionCsvRow {
                parsed_stage: None,
                subaction_time: "12:34".to_string(),
                subaction_name: "SubAction".to_string(),
                ..Default::default()
            };
            assert!(!is_action_row(&csv_row));
        }

        #[test]
        fn is_false_empty_subaction_time() {
            let csv_row = ActionCsvRow {
                parsed_stage: Some((1,"Action".to_string())),
                subaction_time: "".to_string(),
                subaction_name: "SubAction".to_string(),
                ..Default::default()
            };
            assert!(!is_action_row(&csv_row));
        }

        #[test]
        fn is_false_empty_subaction_name() {
            let csv_row = ActionCsvRow {
                parsed_stage: Some((1,"Action".to_string())),
                subaction_time: "12:34".to_string(),
                subaction_name: "".to_string(),
                ..Default::default()
            };
            assert!(!is_action_row(&csv_row));
        }
    }

    mod test_is_stage_boundary {
        use super::super::*;
        use crate::action_csv_row::ActionCsvRow;

        #[test]
    fn is_true() {
        let csv_row = ActionCsvRow {
            parsed_stage: Some((1,"Action".to_string())),
            subaction_time: "".to_string(),
            subaction_name: "".to_string(),
            score: "".to_string(),
            old_value: "".to_string(),
            new_value: "".to_string(),
            ..Default::default()
        };
        assert!(is_stage_boundary(&csv_row));
    }

    #[test]
    fn is_false_non_empty_subaction_time() {
        let csv_row = ActionCsvRow {
            parsed_stage: Some((1,"Action".to_string())),
            subaction_time: "12:34".to_string(),
            subaction_name: "".to_string(),
            score: "".to_string(),
            old_value: "".to_string(),
            new_value: "".to_string(),
            ..Default::default()
        };
        assert!(!is_stage_boundary(&csv_row));
    }

    #[test]
    fn is_false_non_empty_subaction_name() {
        let csv_row = ActionCsvRow {
            parsed_stage: Some((1,"Action".to_string())),
            subaction_time: "".to_string(),
            subaction_name: "SubAction".to_string(),
            score: "".to_string(),
            old_value: "".to_string(),
            new_value: "".to_string(),
            ..Default::default()
        };
        assert!(!is_stage_boundary(&csv_row));
    }

    #[test]
    fn is_false_non_empty_score() {
        let csv_row = ActionCsvRow {
            parsed_stage: Some((1,"Action".to_string())),
            subaction_time: "".to_string(),
            subaction_name: "".to_string(),
            score: "Score".to_string(),
            old_value: "".to_string(),
            new_value: "".to_string(),
            ..Default::default()
        };
        assert!(!is_stage_boundary(&csv_row));
    }

    #[test]
    fn is_false_non_empty_old_value() {
        let csv_row = ActionCsvRow {
            parsed_stage: Some((1,"Action".to_string())),
            subaction_time: "".to_string(),
            subaction_name: "".to_string(),
            score: "".to_string(),
            old_value: "OldValue".to_string(),
            new_value: "".to_string(),
            ..Default::default()
        };
        assert!(!is_stage_boundary(&csv_row));
    }

    #[test]
    fn is_false_non_empty_new_value() {
        let csv_row = ActionCsvRow {
            parsed_stage: Some((1,"Action".to_string())),
            subaction_time: "".to_string(),
            subaction_name: "".to_string(),
            score: "".to_string(),
            old_value: "".to_string(),
            new_value: "NewValue".to_string(),
            ..Default::default()
        };
        assert!(!is_stage_boundary(&csv_row));
    }
}

    mod test_is_error_action_marker {
        use super::super::*;
        use crate::action_csv_row::ActionCsvRow;

        #[test]
        fn is_true() {
            let csv_row = ActionCsvRow {
                old_value: "Error-Triggered".to_string(),
                score: "Action-Was-Performed".to_string(),
                ..Default::default()
            };
            assert!(is_error_action_marker(&csv_row));
        }

        #[test]
        fn is_false_wrong_old_value() {
            let csv_row = ActionCsvRow {
                old_value: "Not-Error".to_string(),
                score: "Action-Was-Performed".to_string(),
                ..Default::default()
            };
            assert!(!is_error_action_marker(&csv_row));
        }

        #[test]
        fn is_false_wrong_score() {
            let csv_row = ActionCsvRow {
                old_value: "Error-Triggered".to_string(),
                score: "Not-Performed".to_string(),
                ..Default::default()
            };
            assert!(!is_error_action_marker(&csv_row));
        }

        #[test]
        fn is_false_both_wrong() {
            let csv_row = ActionCsvRow {
                old_value: "Not-Error".to_string(),
                score: "Not-Performed".to_string(),
                ..Default::default()
            };
            assert!(!is_error_action_marker(&csv_row));
        }

        #[test]
        fn is_false_empty_values() {
            let csv_row = ActionCsvRow {
                old_value: "".to_string(),
                score: "".to_string(),
                ..Default::default()
            };
            assert!(!is_error_action_marker(&csv_row));
        }
    }

    mod test_is_missed_action {
        use super::super::*;

        #[test]
        fn is_true() {
            let csv_row = ActionCsvRow {
                old_value: "Error-Triggered".to_string(),
                score: "Action-Was-Not-Performed".to_string(),
                ..Default::default()
            };
            assert!(is_missed_action(&csv_row));
        }

        #[test]
        fn is_false_wrong_old_value() {
            let csv_row = ActionCsvRow {
                old_value: "Not-Error".to_string(),
                score: "Action-Was-Not-Performed".to_string(),
                ..Default::default()
            };
            assert!(!is_missed_action(&csv_row));
        }

        #[test]
        fn is_false_wrong_score() {
            let csv_row = ActionCsvRow {
                old_value: "Error-Triggered".to_string(),
                score: "Not-Performed".to_string(),
                ..Default::default()
            };
            assert!(!is_missed_action(&csv_row));
        }

        #[test]
        fn is_false_both_wrong() {
            let csv_row = ActionCsvRow {
                old_value: "Not-Error".to_string(),
                score: "Not-Performed".to_string(),
                ..Default::default()
            };
            assert!(!is_missed_action(&csv_row));
        }

        #[test]
        fn is_false_empty_values() {
            let csv_row = ActionCsvRow {
                old_value: "".to_string(),
                score: "".to_string(),
                ..Default::default()
            };
            assert!(!is_missed_action(&csv_row));
        }
    }

    mod test_can_mark_each_other {
        use super::super::*;
        use crate::action_csv_row::ActionCsvRow;
        use crate::scatter_points::CsvRowTime;

        #[test]
        fn within_threshold() {
            let time = 3600;
            let csv_row1 = ActionCsvRow {
                timestamp: Some(CsvRowTime {
                    total_seconds: time,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                }),
                ..Default::default()
            };
            let csv_row2 = ActionCsvRow {
                timestamp: Some(CsvRowTime {
                    total_seconds: time + ERROR_MARKER_TIME_THRESHOLD,
                    date_string: "2024-12-24 01:00:02".to_string(),
                    timestamp: "01:00:02".to_string(),
                }),
                ..Default::default()
            };
            assert!(can_mark_each_other(&csv_row1, &csv_row2));
        }

        #[test]
        fn within_threshold_opposite_direction() {
            let time = 3600;
            let csv_row1 = ActionCsvRow {
                timestamp: Some(CsvRowTime {
                    total_seconds: 3600,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                }),
                ..Default::default()
            };
            let csv_row2 = ActionCsvRow {
                timestamp: Some(CsvRowTime {
                    total_seconds: time - ERROR_MARKER_TIME_THRESHOLD,
                    date_string: "2024-12-24 01:00:02".to_string(),
                    timestamp: "01:00:02".to_string(),
                }),
                ..Default::default()
            };
            assert!(can_mark_each_other(&csv_row1, &csv_row2));
        }

        #[test]
        fn exceeds_threshold() {
            let time = 3600;
            let csv_row1 = ActionCsvRow {
                timestamp: Some(CsvRowTime {
                    total_seconds: time,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                }),
                ..Default::default()
            };
            let csv_row2 = ActionCsvRow {
                timestamp: Some(CsvRowTime {
                    total_seconds: time + ERROR_MARKER_TIME_THRESHOLD + 1,
                    date_string: "2024-12-24 01:00:03".to_string(),
                    timestamp: "01:00:03".to_string(),
                }),
                ..Default::default()
            };
            assert!(!can_mark_each_other(&csv_row1, &csv_row2));
        }

        #[test]
        fn exceeds_threshold_opposite_direction() {
            let time = 3600;
            let csv_row1 = ActionCsvRow {
                timestamp: Some(CsvRowTime {
                    total_seconds: time,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                }),
                ..Default::default()
            };
            let csv_row2 = ActionCsvRow {
                timestamp: Some(CsvRowTime {
                    total_seconds: time - ERROR_MARKER_TIME_THRESHOLD - 1,
                    date_string: "2024-12-24 01:00:03".to_string(),
                    timestamp: "01:00:03".to_string(),
                }),
                ..Default::default()
            };
            assert!(!can_mark_each_other(&csv_row1, &csv_row2));
        }

        #[test]
        fn no_timestamp() {
            let csv_row1 = ActionCsvRow {
                timestamp: None,
                ..Default::default()
            };
            let csv_row2 = ActionCsvRow {
                timestamp: Some(CsvRowTime {
                    total_seconds: 3600,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                }),
                ..Default::default()
            };
            assert!(!can_mark_each_other(&csv_row1, &csv_row2));
        }

        #[test]
        fn both_no_timestamp() {
            let csv_row1 = ActionCsvRow {
                timestamp: None,
                ..Default::default()
            };
            let csv_row2 = ActionCsvRow {
                timestamp: None,
                ..Default::default()
            };
            assert!(can_mark_each_other(&csv_row1, &csv_row2));
        }
    }
    
    mod test_is_erroneous_action {
        use super::super::*;
        use crate::action_csv_row::ActionCsvRow;
        use crate::scatter_points::CsvRowTime;

        #[test]
        fn is_true() {
            let time = 3600;
            let csv_row = ActionCsvRow {
                action_point: true,
                action_vital_name: "User1".to_string(),
                timestamp: Some(CsvRowTime {
                    total_seconds: time,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                }),
                ..Default::default()
            };
            let error_marker_row = ActionCsvRow {
                username: "User1".to_string(),
                timestamp: Some(CsvRowTime {
                    total_seconds: time-ERROR_MARKER_TIME_THRESHOLD,
                    date_string: "2024-12-24 01:00:02".to_string(),
                    timestamp: "01:00:02".to_string(),
                }),
                ..Default::default()
            };
            assert!(is_erroneous_action(&csv_row, &error_marker_row));
        }

        #[test]
        fn is_false_different_stages() {
            let time = 3600;
            let csv_row = ActionCsvRow {
                action_point: true,
                action_vital_name: "X".to_string(),
                timestamp: Some(CsvRowTime {
                    total_seconds: time,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                }),
                ..Default::default()
            };
            let error_marker_row = ActionCsvRow {
                username: "(1)Stage A(action)".to_string(),
                timestamp: Some(CsvRowTime {
                    total_seconds: time+ERROR_MARKER_TIME_THRESHOLD,
                    date_string: "2024-12-24 01:00:02".to_string(),
                    timestamp: "01:00:02".to_string(),
                }),
                ..Default::default()
            };
            assert!(!is_erroneous_action(&csv_row, &error_marker_row));
        }

        #[test]
        fn is_false_time_threshold_exceeded() {
            let time = 3600;
            let csv_row = ActionCsvRow {
                action_point: true,
                action_vital_name: "User1".to_string(),
                timestamp: Some(CsvRowTime {
                    total_seconds: time,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                }),
                ..Default::default()
            };
            let error_marker_row = ActionCsvRow {
                username: "User1".to_string(),
                timestamp: Some(CsvRowTime {
                    total_seconds: time+ERROR_MARKER_TIME_THRESHOLD+1,
                    date_string: "2024-12-24 01:00:05".to_string(),
                    timestamp: "01:00:05".to_string(),
                }),
                ..Default::default()
            };
            assert!(!is_erroneous_action(&csv_row, &error_marker_row));
        }

        #[test]
        fn is_false_time_threshold_exceeded_opposite_direction() {
            let time = 3600;
            let csv_row = ActionCsvRow {
                action_point: true,
                action_vital_name: "User1".to_string(),
                timestamp: Some(CsvRowTime {
                    total_seconds: time,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                }),
                ..Default::default()
            };
            let error_marker_row = ActionCsvRow {
                username: "User1".to_string(),
                timestamp: Some(CsvRowTime {
                    total_seconds: time-ERROR_MARKER_TIME_THRESHOLD-1,
                    date_string: "2024-12-24 01:00:05".to_string(),
                    timestamp: "01:00:05".to_string(),
                }),
                ..Default::default()
            };
            assert!(!is_erroneous_action(&csv_row, &error_marker_row));
        }

        #[test]
        fn is_false_not_action_point() {
            let time = 3600;
            let csv_row = ActionCsvRow {
                action_point: false,
                action_vital_name: "User1".to_string(),
                timestamp: Some(CsvRowTime {
                    total_seconds: time,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                }),
                ..Default::default()
            };
            let error_marker_row = ActionCsvRow {
                username: "User1".to_string(),
                timestamp: Some(CsvRowTime {
                    total_seconds: time+ERROR_MARKER_TIME_THRESHOLD-1,
                    date_string: "2024-12-24 01:00:02".to_string(),
                    timestamp: "01:00:02".to_string(),
                }),
                ..Default::default()
            };
            assert!(!is_erroneous_action(&csv_row, &error_marker_row));
        }
    }
}