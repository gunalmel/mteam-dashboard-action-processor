use csv::Reader;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::{
    fs::File,
    io::{BufReader, Read},
};

mod action_csv_row;
mod debug_message;
mod util;
mod scatter_points;

use crate::action_csv_row::validate_csv_header;
use crate::scatter_points::{Action, ErroneousAction, MissedAction, PeriodType, PlotLocation};
use crate::util::{can_mark_each_other, check_cpr, is_erroneous_action, merge_plot_location_range, ERROR_MARKER_TIME_THRESHOLD};
use action_csv_row::ActionCsvRow;
use debug_message::print_debug_message;
use scatter_points::ActionPlotPoint;

// fn read_csv_file_from_input() -> String {
//     println!("Enter the CSV file name:");
//     let mut input = String::new();
//     io::stdin().read_line(&mut input).unwrap();
//     input.trim().to_string()
// }

fn build_csv_reader<R: Read>(reader: R) -> Reader<R> {
    csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(reader)
}
fn stream_csv_with_errors<R>(
    reader: R,
    max_rows_to_check: usize // Maximum rows to store in memory to check previous records to find erroneous action.
) -> impl Iterator<Item = Result<Option<ActionPlotPoint>, String>>
where
    R: Read
{
    let mut rdr = build_csv_reader(reader);
    let mut recent_rows: VecDeque<ActionCsvRow> = VecDeque::with_capacity(max_rows_to_check);
    let mut cpr_points: Vec<ActionPlotPoint> = Vec::new();
    let mut stage_boundaries: Vec<PlotLocation> = Vec::new();
    let pending_error_marker = RefCell::new(None);//let pending_error_marker = RefCell::new(None);

    if let Err(e) = validate_csv_header(&mut rdr) {
        print_debug_message!("Header parsing errors: {:?}", e);
    }
    
    stage_boundaries.push(PlotLocation::default()); // stage boundaries should start from 0;
    
    let rows = rdr.into_records().enumerate().map(move |(row_idx, result)| {
        match result {
            Ok(raw_row) => {
                let current_row: ActionCsvRow = raw_row.deserialize(None).and_then(|mut cr: ActionCsvRow| {
                    cr.post_deserialize();
                    Ok(cr)
                }).map_err(|e| format!("Could not deserialize row, error: {}", e))?;

                // Trim recent rows to keep a manageable size.
                if recent_rows.len() >= max_rows_to_check {
                    recent_rows.pop_front();
                }
                recent_rows.push_back(current_row.clone());

                if let Some(value) = process_if_stage_boundary(&mut stage_boundaries, &current_row) {
                    return value;
                }
                if let Some(value) = process_if_cpr(&mut cpr_points, &current_row) {
                    return value;
                }
                // Check if there's a pending error marker from a previous iteration.
                if let Some(error_point) = is_current_row_erroneous_action(&pending_error_marker, row_idx, &current_row) {
                    return Ok(Some(error_point));
                }
                // If current row is an error marker, check if it points to an erroneous action row within the previous rows.
                if current_row.error_action_marker {
                    match seek_erroneous_action_in_visited_rows(&recent_rows, &current_row, row_idx) {
                        Some(erroneous_action_in_visited_rows) => return Ok(Some(erroneous_action_in_visited_rows)),
                        None => {*pending_error_marker.borrow_mut() = Some((row_idx, current_row.clone()));}
                    }
                }
                
                if current_row.action_point {
                    return Ok(Some(ActionPlotPoint::Action(Action::new(&current_row))));
                }
                if current_row.missed_action_marker {
                    return Ok(Some(ActionPlotPoint::MissedAction(MissedAction::new(&current_row))));
                }

                // The remaining rows are not points to be plotted so return None.
                Ok(None)
            }
            Err(e) => Err(format!("Could not parse row, error: {}", e)),
        }
    });

    rows
}

fn process_if_stage_boundary(stage_boundary_points: &mut Vec<PlotLocation>, csv_row: &ActionCsvRow) -> Option<Result<Option<ActionPlotPoint>, String>> {
    if csv_row.stage_boundary {
        match stage_boundary_points.pop() {
            Some(plot_location) => {
                let mut start_location = plot_location;
                start_location.stage = csv_row.parsed_stage.clone().unwrap();
                stage_boundary_points.push(PlotLocation::new(csv_row)); //will be the starting point for the next stage
                Some(Ok(Some(ActionPlotPoint::Period(PeriodType::Stage, Some(start_location), Some(PlotLocation::new(csv_row))))))
            },
            None => {
                stage_boundary_points.push(PlotLocation::new(csv_row));
                None
            }
        }
    } else {
        None
    }
}

fn process_if_cpr(cpr_points: &mut Vec<ActionPlotPoint>, csv_row: &ActionCsvRow) -> Option<Result<Option<ActionPlotPoint>, String>> {
    match check_cpr(&csv_row) {
        Some(cpr) => {
            match cpr_points.pop() {
                Some(previous_cpr) => {
                    Some(match merge_plot_location_range(Some(cpr), Some(previous_cpr)) {
                        Ok(merged_cpr) => {
                            Ok(Some(merged_cpr))
                        },
                        Err(err_msg) => {
                            Err(err_msg)
                        }
                    })
                },
                None => {
                    cpr_points.push(cpr.clone());
                    Some(Ok(None))
                }
            }
        }
        None => None
    }
}

fn is_current_row_erroneous_action(pending_error_marker: &RefCell<Option<(usize, ActionCsvRow)>>, row_idx: usize, current_row: &ActionCsvRow) -> Option<ActionPlotPoint> {
    let pending_error_marker_value = pending_error_marker.borrow().clone();
    if let Some((marker_index, error_marker_row)) = pending_error_marker_value {
        // Check if the current row is an erroneous action row.
        if is_erroneous_action(&current_row, &error_marker_row) {
            print_debug_message!("Error marker at row {} points to erroneous action at row {}", marker_index+2, row_idx+2);
            *pending_error_marker.borrow_mut() = None; // Clear the state as the error has been resolved.
            let point = ActionPlotPoint::Error(ErroneousAction::new(&current_row, &error_marker_row));
            return Some(point);
        } else if !can_mark_each_other(&current_row, &error_marker_row) {
            // If row count threshold is exceeded, log and forget the marker.
            print_debug_message!("Error marker at row {} could not find an erroneous action row within {} sec time threshold", marker_index+2, ERROR_MARKER_TIME_THRESHOLD);
            *pending_error_marker.borrow_mut() = None;
        }
    }
    None
}
fn seek_erroneous_action_in_visited_rows(visited_rows_buffer: &VecDeque<ActionCsvRow>, error_marker_row: &ActionCsvRow, error_marker_row_idx: usize) ->Option<ActionPlotPoint> {
        for (recent_index, recent_row) in visited_rows_buffer.iter().rev().enumerate() {
            if is_erroneous_action(&recent_row, &error_marker_row) {
                print_debug_message!(
                                "Error marker at row {} points backward to erroneous action at row {}",
                                error_marker_row_idx+2,
                                (error_marker_row_idx - recent_index)+2
                            );
                let point = ActionPlotPoint::Error(ErroneousAction::new(&recent_row, &error_marker_row));
                return Some(point);
            }
        }
    None
}

fn main() {
    // let file_name = read_csv_file_from_input();
    let file_name = "timeline-multiplayer-09182024.csv";
    match File::open(file_name) {
        Ok(file) => {
            let buffered = BufReader::new(file);
           for (row_idx, result) in stream_csv_with_errors(buffered, 10).enumerate() {
               let line_number = row_idx + 2;
               match result {
                   // Ok(Some(ActionPlotPoint::Error(error_point))) => {
                   //     print_debug_message!("{} Error: {:#?}", line_number, error_point);
                   // }
                   // Ok(Some(ActionPlotPoint::Action(action_point))) => {
                   //     print_debug_message!("{} Action: {:#?}", line_number, action_point);
                   // },
                   Ok(Some(ActionPlotPoint::Period(PeriodType::Stage, start, end))) => { print_debug_message!("{} stage_boundary: {:#?}", line_number, (start,end)); },
                   // Ok(Some(ActionPlotPoint::MissedAction(missed_action))) => { print_debug_message!("{} missed_action: {:?}", line_number, missed_action); },
                   // Ok(Some(ActionPlotPoint::Period(PeriodType::CPR, start, end))) => { print_debug_message!("{} stage_boundary: {:#?}", line_number, (start,end)); },
                   // Ok(None) => {print_debug_message!("{} skipped line", line_number)},
                   // Err(e) => {print_debug_message!("{} error: {}", line_number, e);}
                   _ => {}
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