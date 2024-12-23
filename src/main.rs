use csv::Reader;
use std::{
    fs::File,
    io::{self, BufReader, Read},
};
use std::collections::VecDeque;
use std::time::Duration;
use chrono::{NaiveTime, TimeDelta};

mod action_csv_row;
mod debug_message;
use crate::action_csv_row::validate_csv_header;
use action_csv_row::ActionCsvRow;
use debug_message::print_debug_message;

fn read_csv_file_from_input() -> String {
    println!("Enter the CSV file name:");
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn build_csv_reader<R: Read>(reader: R) -> Reader<R> {
    csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(reader)
}


#[derive(Debug)]
struct ErrorPoint {
    timestamp: NaiveTime,
    action_rule: String,
    violation: String,
    advice: String,
}

#[derive(Debug)]
struct ActionPoint {
    timestamp: NaiveTime
}

#[derive(Debug)]
enum ActionPlotPoint {
    Error(ErrorPoint),
    Action(ActionPoint),
}

fn stream_csv_with_errors<R>(
    reader: R,
    memory_size: usize,
    max_time_diff_std: Duration,
) -> impl Iterator<Item = Result<ActionPlotPoint, String>>
where
    R: Read,
{
    let mut rdr = build_csv_reader(reader);
    let mut errors: Vec<String> = Vec::new();

    if let Err(e) = validate_csv_header(&mut rdr) {
        errors.push(e);
        print_debug_message!("Header parsing errors are pushed to errors vector: {:?}", errors);
    }
    let mut buffer: VecDeque<ActionCsvRow> = VecDeque::new();
    let records = rdr.into_records().map(move |result| {
        match result {
            Ok(raw_row) => {
                let mut record: ActionCsvRow = raw_row.deserialize(None).map_err(|e| format!("Could not deserialize row, error: {}", e))?;
                record.post_deserialize();
                // No need to clone here, we own the record now

                buffer.push_back(record.clone());

                if buffer.len() > memory_size {
                    buffer.pop_front();
                }
                // Convert StdDuration to TimeDelta *once*, outside the loop
                let max_time_diff = TimeDelta::from_std(max_time_diff_std).map_err(|e| format!("Invalid max_time_diff: {}", e))?;
                if record.error_trigger {
                    process_error_row(&record, &buffer, max_time_diff).ok_or("No matching action row found within time range".to_string())
                } else {
                    Ok(ActionPlotPoint::Action(ActionPoint {
                        timestamp: parse_time(&record.timestamp).unwrap_or_default()
                    }))
                }
            }
            Err(e) => Err(format!("Could not parse row, error: {}", e)),
        }
    });

    records
}

fn process_error_row(error_row: &ActionCsvRow, buffer: &VecDeque<ActionCsvRow>, max_time_diff: TimeDelta) -> Option<ActionPlotPoint> {
    let error_time = parse_time(&error_row.timestamp)?;
    let mut closest_action: Option<(&ActionCsvRow, TimeDelta)> = None;

    for action_row in buffer {
        if action_row.action_vital_name == error_row.username {
            let action_time = parse_time(&action_row.timestamp)?;

            // Calculate the difference as a ChronoDuration
            let time_diff_chrono = if action_time > error_time {
                action_time - error_time
            } else {
                error_time - action_time
            };

            // Convert ChronoDuration to std::time::Duration
            let time_diff_std = time_diff_chrono.to_std().ok()?;

            // Convert std::time::Duration to TimeDelta
            let time_diff = TimeDelta::from_std(time_diff_std).ok()?;

            if time_diff <= max_time_diff {
                if closest_action.is_none() || time_diff < closest_action.unwrap().1 {
                    closest_action = Some((action_row, time_diff));
                }
            }
        }
    }

    closest_action.map(|(action_row, _)| ActionPlotPoint::Error(ErrorPoint {
        timestamp: parse_time(&action_row.timestamp).unwrap(),
        action_rule: action_row.subaction_name.clone(),
        violation: error_row.score.clone(),
        advice: error_row.speech_command.clone(),
    }))
}

fn parse_time(time_str: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(time_str, "%H:%M:%S").ok()
}

fn main() {
    let file_name = read_csv_file_from_input();

    match File::open(file_name) {
        Ok(file) => {
            let buffered = BufReader::new(file);
           for result in stream_csv_with_errors(buffered, 10, Duration::from_secs(5)) {
               match result {
                   Ok(ActionPlotPoint::Error(error_point)) => {
                       print_debug_message!("Error: {:?}", error_point);
                   }
                   Ok(ActionPlotPoint::Action(_action_point)) => {
                       // print_debug_message!("Action: {:?}", action_point);
                   }
                   Err(_e) => {}//eprintln!("Error processing row: {}", e),
               }
           }
        }
        Err(e) => eprintln!("Error opening file: {}", e),
    }
}
/*
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_process_csv_with_valid_input() {
        let data = "Time Stamp[Hr:Min:Sec],Action/Vital Name,SubAction Time[Min:Sec],SubAction Name,Score,Old Value,New Value,Username,Speech Command\n\
                12:00:00,Action1,02:30,SubAction1,10,Old1,User1,New1,User1,Command1\n";
        let cursor = Cursor::new(data);

        let result = process_csv(cursor);
        assert!(result.is_ok(), "Expected process_csv to succeed for valid input");

        let errors = result.unwrap();
        assert!(errors.is_empty(), "Expected no errors for valid input");
    }

    #[test]
    fn test_process_csv_with_invalid_input() {
        let data = "Time Stamp[Hr:Min:Sec],Action/Vital Name,SubAction Time[Min:Sec],SubAction Name,Score,Old Value,New Value,Username,Speech Command\n\
                ,Action1,invalid_time,SubAction1,10,Old1,New1,User1,Command1\n";
        let cursor = Cursor::new(data);

        let result = process_csv(cursor);
        assert!(result.is_ok(), "Expected process_csv to succeed even with invalid rows");

        let errors = result.unwrap();
        assert_eq!(errors.len(), 1, "Expected one error for invalid input");
        assert!(errors[0].contains("Line 2: could not deserialize row ,Action1,invalid_time,"), "The only error should be on the second line");
    }

    #[test]
    fn test_process_csv_with_empty_input() {
        let data = "";
        let cursor = Cursor::new(data);

        let result = process_csv(cursor);
        assert!(result.is_ok(), "Expected process_csv to succeed for empty input");

        let errors = result.unwrap();
        assert!(errors[0].contains("Line 1: expected [\"Time Stamp[Hr:Min:Sec]\""), "Expected error for missing header");
    }

    #[test]
    fn test_process_csv_with_mixed_input() {
        let data = "Time Stamp[Hr:Min:Sec],Action/Vital Name,SubAction Time[Min:Sec],SubAction Name,Score,Old Value,New Value,Username,Speech Command\n\
                12:00:00,Action1,02:30,SubAction1,10,Old1,User1,New1,User1,Command1\n\
                12:30:00,Action2,invalid_time,SubAction2,20,Old2,User2,New2,User2,Command2\n";
        let cursor = Cursor::new(data);

        let result = process_csv(cursor);
        assert!(result.is_ok(), "Expected process_csv to succeed for mixed input");

        let errors = result.unwrap();
        assert!(errors.is_empty(), "Expected two errors for mixed input");
    }
}
*/