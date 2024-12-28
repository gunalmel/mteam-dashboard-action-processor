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
mod csv_reader;
mod row_processing;

use crate::scatter_points::{Action, ErroneousAction, MissedAction, PeriodType, PlotLocation};
use crate::util::{can_mark_each_other, check_cpr, is_erroneous_action, is_error_action_marker, is_missed_action, is_stage_boundary, merge_plot_location_range, ERROR_MARKER_TIME_THRESHOLD};
use action_csv_row::ActionCsvRow;
use debug_message::print_debug_message;
use scatter_points::ActionPlotPoint;
// fn read_csv_file_from_input() -> String {
//     println!("Enter the CSV file name:");
//     let mut input = String::new();
//     io::stdin().read_line(&mut input).unwrap();
//     input.trim().to_string()
// }

fn process_if_stage_boundary(
    stage_boundary_points: &mut Vec<PlotLocation>,
    csv_row: &ActionCsvRow,
) -> Option<Result<ActionPlotPoint, String>> {
    if !is_stage_boundary(csv_row) {
        return None;
    }

    let start_location = stage_boundary_points.pop().map_or_else(
        || PlotLocation::new(csv_row),
        |mut location| {
            location.stage = csv_row.parsed_stage.clone().unwrap();
            location
        },
    );

    stage_boundary_points.push(PlotLocation::new(csv_row));

    Some(Ok(ActionPlotPoint::Period(
        PeriodType::Stage,
        Some(start_location),
        Some(PlotLocation::new(csv_row)),
    )))
}

fn process_cpr_lines(cpr_points: &mut Vec<ActionPlotPoint>, csv_row: &ActionCsvRow) -> Option<Result<ActionPlotPoint, String>> {
    match check_cpr(&csv_row) {
        Some(cpr) => {
            match cpr_points.pop() {
                Some(previous_cpr) => {
                    Some(match merge_plot_location_range(Some(cpr), Some(previous_cpr)) {
                        Ok(merged_cpr) => {
                            Ok(merged_cpr)
                        },
                        Err(err_msg) => {
                            Err(err_msg)
                        }
                    })
                },
                None => {
                    cpr_points.push(cpr.clone());
                    None
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
fn seek_erroneous_action_in_visited_rows(
    visited_rows_buffer: &VecDeque<ActionCsvRow>,
    error_marker_row: &ActionCsvRow,
    error_marker_row_idx: usize,
) -> Option<PlotPointResult> {
    for (recent_index, recent_row) in visited_rows_buffer.iter().rev().enumerate() {
        if is_erroneous_action(recent_row, error_marker_row) {
            print_debug_message!(
                "Error marker at row {} points backward to erroneous action at row {}",
                error_marker_row_idx + 2,
                (error_marker_row_idx - recent_index) + 2
            );
            let point = ActionPlotPoint::Error(ErroneousAction::new(recent_row, error_marker_row));
            return Some(Ok(point)); // Wrap in Ok to match PlotPointResult
        }
    }
    None
}

type CsvResult = Result<ActionCsvRow, String>;
type PlotPointResult = Result<ActionPlotPoint, String>;

fn stream_csv_with_errors<'r, R>(
    reader: R,
    max_rows_to_check: usize,
) -> Box<dyn Iterator<Item = PlotPointResult> + 'r>
where
    R: Read + 'r,
{
    let csv_reader = match csv_reader::initialize_csv_reader(reader) {
        Ok(r) => r,
        Err(e) => return Box::new(vec![Err(e)].into_iter()),
    };

    let mut state = CsvProcessingState::new(max_rows_to_check);

    Box::new(
        csv_reader
            .into_records()
            .enumerate()
            .filter_map(move |(row_idx, result)| row_processing::process_csv_row(row_idx, result, &mut state)),
    )
}

/// State encapsulating CSV processing logic.
struct CsvProcessingState {
    max_rows_to_check: usize,
    recent_rows: VecDeque<ActionCsvRow>,
    stage_boundaries: Vec<PlotLocation>,
    cpr_points: Vec<ActionPlotPoint>,
    pending_error_marker: RefCell<Option<(usize, ActionCsvRow)>>,
}

impl CsvProcessingState {
    fn new(max_rows_to_check: usize) -> Self {
        Self {
            max_rows_to_check,
            recent_rows: VecDeque::with_capacity(max_rows_to_check),
            stage_boundaries: vec![PlotLocation::default()],
            cpr_points: Vec::new(),
            pending_error_marker: RefCell::new(None),
        }
    }
}

fn process_stage_boundaries(
    current_row: &ActionCsvRow,
    stage_boundaries: &mut Vec<PlotLocation>,
) -> Option<PlotPointResult> {
    process_if_stage_boundary(stage_boundaries, current_row)
}

fn process_erroneous_action(state: &CsvProcessingState, row_idx: usize, current_row: &ActionCsvRow, ) -> Option<PlotPointResult> {
    if let Some(error_point) = is_current_row_erroneous_action(
        &state.pending_error_marker,
        row_idx,
        current_row,
    ) {
        return Some(Ok(error_point));
    }

    if is_error_action_marker(current_row) {
        seek_erroneous_action_in_visited_rows(&state.recent_rows, current_row, row_idx)
            .or_else(|| {
                *state.pending_error_marker.borrow_mut() = Some((row_idx, current_row.clone()));
                None
            })
    } else if is_missed_action(current_row) {
        Some(Ok(ActionPlotPoint::MissedAction(MissedAction::new(current_row))))
    } else {
        None
    }
}

fn process_action_point(current_row: &ActionCsvRow) -> Option<PlotPointResult> {
    if current_row.action_point {
        Some(Ok(ActionPlotPoint::Action(Action::new(current_row))))
    } else {
        None
    }
}


fn main() {
    // let file_name = read_csv_file_from_input();
    let file_name = "timeline-multiplayer-09182024.csv";
    match File::open(file_name) {
        Ok(file) => {
            let buffered = BufReader::new(file);
           for (row_idx, result) in stream_csv_with_errors(buffered, 10).enumerate() {
               let item_number = row_idx + 1;
               match result {
                  // Ok(_)=> { print_debug_message!("{}", item_number); },
                  //  Ok(ActionPlotPoint::Error(error_point)) => {
                  //      print_debug_message!("{} Error: {:#?}", item_number, error_point);
                  //  }
                  //  Ok(ActionPlotPoint::Action(action_point)) => {
                  //      print_debug_message!("{} Action: {:#?}", item_number, action_point);
                  //  },
                   Ok(ActionPlotPoint::Period(PeriodType::Stage, start, end)) => { print_debug_message!("{} stage_boundary: {:#?}", item_number, (start,end)); },
                   // Ok(ActionPlotPoint::MissedAction(missed_action)) => { print_debug_message!("{} missed_action: {:?}", item_number, missed_action); },
                   // Ok(ActionPlotPoint::Period(PeriodType::CPR, start, end)) => { print_debug_message!("{} stage_boundary: {:#?}", item_number, (start,end)); },
                   Err(e) => {print_debug_message!("{} error: {}", item_number, e);},
                   _=> { }
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