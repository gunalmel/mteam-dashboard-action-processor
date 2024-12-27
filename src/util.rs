use crate::action_csv_row::ActionCsvRow;
use crate::scatter_points::{ActionPlotPoint, CsvRowTime, PeriodType, PlotLocation};
use chrono::{Datelike, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use std::string::ToString;

lazy_static! {
    static ref ACTION_NAME_REGEX: Regex = Regex::new(r"^\s*\((\d+)\)\s*(.+?)\s*\(action\)\s*$").unwrap();
    static ref SHOCK_VALUE_REGEX: Regex = Regex::new(r"(.*?)(\b\d+[Jj]\b)(.*)").unwrap(); 
}
const CPR_START_MARKERS: [&'static str; 2] = ["begin cpr", "enter cpr"];
const CPR_END_MARKERS: [&'static str; 2]  = ["stop cpr", "end cpr"];
pub const ERROR_MARKER_TIME_THRESHOLD: u32 = 2;
fn normalize_whitespace(input: &str) -> String {
    input
        .trim()
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}
fn capitalize_words(input: &str) -> String {
    input
        .trim()
        .split_whitespace()
        .map(|word| {
            if word.chars().all(|c| c.is_numeric() || c.is_uppercase()) {
                return word.to_string();
            }

            if word.starts_with('(') {
                return format!("({}", capitalize_words(&word[1..word.len()])); // Recurse to handle nested parentheses
            }

            let mut chars = word.chars();
            let first_char = chars.next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
            let rest: String = chars.as_str().to_lowercase();
            first_char + &rest
        })
        .collect::<Vec<String>>()
        .join(" ")
        .replace(" ( ", " (")
        .replace(") ", ") ")
        .replace(" )", ")")
}

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
            normalize_whitespace(matched.as_str())
        })?;
        Some((number, action_name))
    })
}
fn extract_shock_value(input: &str) -> (String, String) {
    match SHOCK_VALUE_REGEX.captures(input).map(|captures| {
        let before = captures.get(1).map_or("", |m| m.as_str()).trim();
        let value = captures.get(2).map_or("", |m| m.as_str()).trim();
        let after = captures.get(3).map_or("", |m| m.as_str()).trim();

        (format!("{} {}", before, after).trim().to_string(), value.to_string())
    }){
        Some((action_name, joule)) => {
            if joule.is_empty() {
                (action_name, "".to_string())
            } else {
                (action_name, joule)
            }
        },
        None => (input.to_string(), "".to_string())
    }
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
pub fn check_cpr(csv_row: &ActionCsvRow) -> Option<ActionPlotPoint> {
    let normalized_action_name = normalize_whitespace(csv_row.subaction_name.to_lowercase().as_str());
    if CPR_START_MARKERS.contains(&&*normalized_action_name) {
        return Some(ActionPlotPoint::Period(PeriodType::CPR, Some(PlotLocation::new(csv_row)), None));
    }
    else if CPR_END_MARKERS.contains(&&*normalized_action_name) {
        return Some(ActionPlotPoint::Period(PeriodType::CPR, None, Some(PlotLocation::new(csv_row))));
    }
    None
}
pub fn merge_plot_location_range(app1: Option<ActionPlotPoint>, app2: Option<ActionPlotPoint>)->Result<ActionPlotPoint, String> {
    const ERR_MSG: &str = "Should have a begin and end point in order. Start/end markers are not encountered as expected for a valid range/duration.";
    match (app1, app2) {
        (Some(ActionPlotPoint::Period(period_type1, start1, end1)), Some(ActionPlotPoint::Period(period_type2, start2, end2))) => {
            if period_type1 != period_type2 {
                return Err("The periods you are trying to merge are of different types".to_string());
            }
            let merge = |pos1: Option<PlotLocation>, pos2: Option<PlotLocation>| -> Result<Option<PlotLocation>, String> {
                match (pos1, pos2) {
                    (None, None) => Ok(None),
                    (Some(pos), None) | (None, Some(pos)) => Ok(Some(pos)),
                    (Some(_), Some(_)) => Err(ERR_MSG.to_string()), // Overlap is not allowed
                }
            };

            let start = merge(start1, start2)?;
            let end = merge(end1, end2)?;

            Ok(ActionPlotPoint::Period(period_type1, start, end))
        }
        _ => Err(ERR_MSG.to_string()),
    }
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
pub fn process_action_name(input: &str) -> (String, String, String) {
    let (normalized_action_name, joule) = extract_shock_value(capitalize_words(input).replace("UNAVAILABLE", "").trim());
    let corrected_action_name = match normalized_action_name.as_str() {
        "Ascultate Lungs" => "Auscultate Lungs".to_string(),
        "SYNCHRONIZED Shock" => "Synchronized Shock".to_string(),
        _ => normalized_action_name,
    };
   
    let category = match corrected_action_name.as_str() {
        "Select Amiodarone" => "Medication".to_string(),
        "Select Calcium" => "Medication".to_string(),
        "Select Epinephrine" => "Medication".to_string(),
        "Select Lidocaine" => "Medication".to_string(),
        _ => corrected_action_name.clone(),
    };

    (corrected_action_name, category, joule)
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
    mod test_normalize_whitespace {
        use super::super::*;

        #[test]
        fn whitespace_basic() {
            let input = "   Hello   World   ";
            let expected = "Hello World";
            assert_eq!(normalize_whitespace(input), expected);
        }

        #[test]
        fn whitespace_multiple_spaces() {
            let input = "Rust    is     awesome!";
            let expected = "Rust is awesome!";
            assert_eq!(normalize_whitespace(input), expected);
        }

        #[test]
        fn whitespace_empty_string() {
            let input = "";
            let expected = "";
            assert_eq!(normalize_whitespace(input), expected);
        }

        #[test]
        fn whitespace_only_spaces() {
            let input = "      ";
            let expected = "";
            assert_eq!(normalize_whitespace(input), expected);
        }

        #[test]
        fn with_tabs_and_newlines() {
            let input = "Hello\t\tWorld\n\nRust    ";
            let expected = "Hello World Rust";
            assert_eq!(normalize_whitespace(input), expected);
        }
    }

    mod test_capitalize_words {
        use super::super::*;
        #[test]
        fn capitalize() {
            assert_eq!(capitalize_words("hello world"), "Hello World");
            assert_eq!(capitalize_words("   rust is awesome   "), "Rust Is Awesome");
            assert_eq!(capitalize_words("multiple   spaces   here"), "Multiple Spaces Here");
            assert_eq!(capitalize_words(""), "");
            assert_eq!(capitalize_words("   "), "");
            assert_eq!(capitalize_words("already Capitalized"), "Already Capitalized");
            assert_eq!(capitalize_words("single"), "Single");
            assert_eq!(capitalize_words("123 testing numbers"), "123 Testing Numbers");
            assert_eq!(capitalize_words("Defib (UNsynchronized Shock) 200J"), "Defib (Unsynchronized Shock) 200J");
            assert_eq!(capitalize_words(" Defib   ( UNsynchronized   Shock  )   100J "), "Defib (Unsynchronized Shock) 100J");
            assert_eq!(capitalize_words("(parentheses) around words"), "(Parentheses) Around Words");
            assert_eq!(capitalize_words("punctuation, should work!"), "Punctuation, Should Work!");
            assert_eq!(capitalize_words("100j test"), "100j Test");
            assert_eq!(capitalize_words("Order EKG"), "Order EKG");
            assert_eq!(capitalize_words("  Order  EKG    test  "), "Order EKG Test");
        }
    }
    
    mod test_parse_time {
        use super::super::*;
        use chrono::DateTime;

        #[test]
        fn valid_time() {
            let timestamp = "12:34:56";
            let parsed_time = parse_time(timestamp).unwrap();
            let now: DateTime<Utc> = Utc::now();
            let formatted_date_string = now.format("%Y-%m-%d").to_string() + " " + timestamp;
            
            assert_eq!(parsed_time.timestamp, timestamp);
            assert_eq!(parsed_time.total_seconds, 12 * 3600 + 34 * 60 + 56);
            assert_eq!(parsed_time.date_string,formatted_date_string);
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

    mod test_etract_shock_value {
        use super::super::*;

        #[test]
        fn basic() {
            assert_eq!(extract_shock_value("xyz rts 100J klm abc"), ("xyz rts klm abc".to_string(), "100J".to_string()));
        }

        #[test]
        fn lowercase_j() {
            assert_eq!(extract_shock_value("xyz rts 100j klm abc"), ("xyz rts klm abc".to_string(), "100j".to_string()));
        }

        #[test]
        fn no_value() {
            assert_eq!(extract_shock_value("no value here"), ("no value here".to_string(), "".to_string()));
        }

        #[test]
        fn at_beginning() {
            assert_eq!(extract_shock_value("123J at the beginning"), ("at the beginning".to_string(), "123J".to_string()));
        }

        #[test]
        fn at_end() {
            assert_eq!(extract_shock_value("at the end 456j"), ("at the end".to_string(), "456j".to_string()));
        }

        #[test]
        fn multiple_values() {
            assert_eq!(extract_shock_value("multiple 789J values 123j in string"), ("multiple values 123j in string".to_string(), "789J".to_string()));
        }

        #[test]
        fn leading_trailing_spaces() {
            assert_eq!(extract_shock_value(" leading and trailing spaces 100J "), ("leading and trailing spaces".to_string(), "100J".to_string()));
        }

        #[test]
        fn only_value() {
            assert_eq!(extract_shock_value("100J"), ("".to_string(), "100J".to_string()));
        }

        #[test]
        fn with_spaces_around() {
            assert_eq!(extract_shock_value("test   100J   test"), ("test test".to_string(), "100J".to_string()));
        }

        #[test]
        fn no_letters_around_value() {
            assert_eq!(extract_shock_value("100Jtest"), ("100Jtest".to_string(), "".to_string()));
        }

        #[test]
        fn at_the_very_end() {
            assert_eq!(extract_shock_value("test 100J"), ("test".to_string(), "100J".to_string()));
        }

        #[test]
        fn at_the_very_beginning() {
            assert_eq!(extract_shock_value("100J test"), ("test".to_string(), "100J".to_string()));
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

        fn create_csv_row(time: u32) -> (u32, ActionCsvRow) {
            
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
            (time, csv_row)
        }

        #[test]
        fn is_true() {
            let time = 3600;
            let (time, csv_row) = create_csv_row(time);
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
            let (time, csv_row) = create_csv_row(time);
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
            let (time, csv_row) = create_csv_row(time);
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
    
    mod test_check_cpr{
        use super::super::*;
        use crate::action_csv_row::ActionCsvRow;
        use crate::scatter_points::{CsvRowTime, PlotLocation};
        #[test]
        fn is_beginning() {
            let expected_plot_location = PlotLocation {
                timestamp: CsvRowTime {
                    total_seconds: 3600,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                },
                stage: (1,"Stage 1".to_string())
            };
            let mut csv_row = ActionCsvRow {
                timestamp: expected_plot_location.timestamp.clone().into(),
                parsed_stage: expected_plot_location.stage.clone().into(),
                subaction_name: "  BeGin   CpR  ".to_string(),
                ..Default::default()
            };
            let expected = Some(ActionPlotPoint::Period(PeriodType::CPR, Some(expected_plot_location), None));
            assert_eq!(expected, check_cpr(&csv_row));
            csv_row.subaction_name = "  enteR   cPR  ".to_string();
            assert_eq!(expected, check_cpr(&csv_row));
            csv_row.subaction_name = "Begin CPR".to_string();
            assert_eq!(expected, check_cpr(&csv_row));
            csv_row.subaction_name = "Enter CPR".to_string();
            assert_eq!(expected, check_cpr(&csv_row));
        }

        #[test]
        fn is_end() {
            let expected_plot_location = PlotLocation {
                timestamp: CsvRowTime {
                    total_seconds: 3600,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                },
                stage: (1,"Stage 1".to_string())
            };
            let mut csv_row = ActionCsvRow {
                timestamp: expected_plot_location.timestamp.clone().into(),
                parsed_stage: expected_plot_location.stage.clone().into(),
                subaction_name: "  Stop   CPR  ".to_string(),
                ..Default::default()
            };
            let expected = Some(ActionPlotPoint::Period(PeriodType::CPR, None, Some(expected_plot_location)));
            assert_eq!(expected, check_cpr(&csv_row));
            csv_row.subaction_name = "  enD   cPR  ".to_string();
            assert_eq!(expected, check_cpr(&csv_row));
            csv_row.subaction_name = "stoP CpR".to_string();
            assert_eq!(expected, check_cpr(&csv_row));
            csv_row.subaction_name = "End CPR".to_string();
            assert_eq!(expected, check_cpr(&csv_row));
        }
    }
    
    mod test_merge_plot_location_range {
        use crate::scatter_points::PeriodType;
        use super::super::*;
        #[test]
        fn success() {
            let plot_location1 = PlotLocation {
                timestamp: CsvRowTime {
                    total_seconds: 3600,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                },
                stage: (1, "Stage 1".to_string()),
            };
            let plot_location2 = PlotLocation {
                timestamp: CsvRowTime {
                    total_seconds: 3602,
                    date_string: "2024-12-24 01:00:02".to_string(),
                    timestamp: "01:00:02".to_string(),
                },
                stage: (2, "Stage 2".to_string()),
            };

            let period1 = ActionPlotPoint::Period(PeriodType::CPR, Some(plot_location1.clone()), None);
            let period2 = ActionPlotPoint::Period(PeriodType::CPR, None, Some(plot_location2.clone()));

            // Test merging with valid tuples
            let mut actual = merge_plot_location_range(Some(period1.clone()), Some(period2.clone()));
            assert_eq!(
                Ok(ActionPlotPoint::Period(PeriodType::CPR, Some(plot_location1.clone()), Some(plot_location2.clone()))),
                actual
            );

            // Test reversed inputs
            actual = merge_plot_location_range(Some(period2), Some(period1));
            assert_eq!(
                Ok(ActionPlotPoint::Period(PeriodType::CPR, Some(plot_location1.clone()), Some(plot_location2.clone()))),
                actual
            );
        }

        #[test]
        fn fail() {
            let plot_location1 = PlotLocation {
                timestamp: CsvRowTime {
                    total_seconds: 3600,
                    date_string: "2024-12-24 01:00:00".to_string(),
                    timestamp: "01:00:00".to_string(),
                },
                stage: (1, "Stage 1".to_string()),
            };
            let plot_location2 = PlotLocation {
                timestamp: CsvRowTime {
                    total_seconds: 3602,
                    date_string: "2024-12-24 01:00:02".to_string(),
                    timestamp: "01:00:02".to_string(),
                },
                stage: (2, "Stage 2".to_string()),
            };

            let period1 = ActionPlotPoint::Period(PeriodType::CPR, Some(plot_location1.clone()), None);
            let period2 = ActionPlotPoint::Period(PeriodType::CPR, Some(plot_location2.clone()), None);

            // Test overlapping starts
            let mut actual = merge_plot_location_range(Some(period1.clone()), Some(period2.clone()));
            assert!(actual.is_err());

            let period3 = ActionPlotPoint::Period(PeriodType::Stage, None, Some(plot_location1.clone()));
            let period4 = ActionPlotPoint::Period(PeriodType::Stage, None, Some(plot_location2.clone()));

            // Test overlapping ends
            actual = merge_plot_location_range(Some(period3), Some(period4));
            assert!(actual.is_err());

            // Test both start and end overlap
            let period5 = ActionPlotPoint::Period(PeriodType::Stage, Some(plot_location1.clone()), Some(plot_location2.clone()));
            let period6 = ActionPlotPoint::Period(PeriodType::Stage, Some(plot_location2.clone()), Some(plot_location1.clone()));

            actual = merge_plot_location_range(Some(period5), Some(period6));
            assert!(actual.is_err());

            // Test different types
            let period7 = ActionPlotPoint::Period(PeriodType::Stage, Some(plot_location1.clone()), None);
            let period8 = ActionPlotPoint::Period(PeriodType::CPR, None, Some(plot_location2.clone()));

            actual = merge_plot_location_range(Some(period7), Some(period8));
            assert!(actual.is_err());
        }
    }

    mod test_process_action_name {
        use super::super::*;

        #[test]
        fn process_action_names() {
            let test_cases = [
                ("Ascultate Lungs", ("Auscultate Lungs".to_string(), "Auscultate Lungs".to_string(), "".to_string())),
                ("Auscultate Lungs", ("Auscultate Lungs".to_string(), "Auscultate Lungs".to_string(), "".to_string())),
                ("Check Lab Tests", ("Check Lab Tests".to_string(), "Check Lab Tests".to_string(), "".to_string())),
                ("Defib (UNsynchronized Shock) 100J", ("Defib (Unsynchronized Shock)".to_string(), "Defib (Unsynchronized Shock)".to_string(), "100J".to_string())),
                ("Defib (UNsynchronized Shock) 200J", ("Defib (Unsynchronized Shock)".to_string(), "Defib (Unsynchronized Shock)".to_string(), "200J".to_string())),
                ("Defib (UNsynchronized Shock) 300J", ("Defib (Unsynchronized Shock)".to_string(), "Defib (Unsynchronized Shock)".to_string(), "300J".to_string())),
                ("Insert Bag Mask", ("Insert Bag Mask".to_string(), "Insert Bag Mask".to_string(), "".to_string())),
                ("Insert Lactated Ringers (1 Liter)", ("Insert Lactated Ringers (1 Liter)".to_string(), "Insert Lactated Ringers (1 Liter)".to_string(), "".to_string())),
                ("Insert Syringe on Right Hand", ("Insert Syringe On Right Hand".to_string(), "Insert Syringe On Right Hand".to_string(), "".to_string())),
                ("Measure Glucose Level", ("Measure Glucose Level".to_string(), "Measure Glucose Level".to_string(), "".to_string())),
                ("Order Chest X-ray", ("Order Chest X-ray".to_string(), "Order Chest X-ray".to_string(), "".to_string())),
                ("Order Cooling", ("Order Cooling".to_string(), "Order Cooling".to_string(), "".to_string())),
                ("Order EKG", ("Order EKG".to_string(), "Order EKG".to_string(), "".to_string())),
                ("Order Intubation", ("Order Intubation".to_string(), "Order Intubation".to_string(), "".to_string())),
                ("Order Needle Thoracostomy", ("Order Needle Thoracostomy".to_string(), "Order Needle Thoracostomy".to_string(), "".to_string())),
                ("Order new Labs UNAVAILABLE", ("Order New Labs".to_string(), "Order New Labs".to_string(), "".to_string())),
                ("Order Pericardiocentesis", ("Order Pericardiocentesis".to_string(), "Order Pericardiocentesis".to_string(), "".to_string())),
                ("Order Ultrasound", ("Order Ultrasound".to_string(), "Order Ultrasound".to_string(), "".to_string())),
                ("Perform Bag Mask Pump", ("Perform Bag Mask Pump".to_string(), "Perform Bag Mask Pump".to_string(), "".to_string())),
                ("Pulse Check", ("Pulse Check".to_string(), "Pulse Check".to_string(), "".to_string())),
                ("Select Amiodarone", ("Select Amiodarone".to_string(), "Medication".to_string(), "".to_string())),
                ("Select Calcium", ("Select Calcium".to_string(), "Medication".to_string(), "".to_string())),
                ("Select Epinephrine", ("Select Epinephrine".to_string(), "Medication".to_string(), "".to_string())),
                ("Select Lidocaine", ("Select Lidocaine".to_string(), "Medication".to_string(), "".to_string())),
                ("SYNCHRONIZED Shock 100J", ("Synchronized Shock".to_string(), "Synchronized Shock".to_string(), "100J".to_string())),
                ("SYNCHRONIZED Shock 200J", ("Synchronized Shock".to_string(), "Synchronized Shock".to_string(), "200J".to_string())),
                ("View Cardiac Arrest Guidelines", ("View Cardiac Arrest Guidelines".to_string(), "View Cardiac Arrest Guidelines".to_string(), "".to_string())),
            ];

            for (input, expected) in test_cases {
                let result = process_action_name(input);
                assert_eq!(result, expected);
            }
        }
    }
}