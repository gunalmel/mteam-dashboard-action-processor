use std::cell::RefCell;
use std::collections::VecDeque;
use crate::action_csv_row::ActionCsvRow;
use crate::debug_message::print_debug_message;
use crate::scatter_points::{Action, ActionPlotPoint, ErroneousAction, MissedAction, PeriodType, PlotLocation};
use crate::state_management::CsvProcessingState;
use crate::util::{can_mark_each_other, check_cpr, is_erroneous_action, is_error_action_marker, is_missed_action, is_stage_boundary, merge_plot_location_range, ERROR_MARKER_TIME_THRESHOLD};

pub fn process_if_stage_boundary(stage_boundary_points: &mut Vec<PlotLocation>, csv_row: &ActionCsvRow) -> Option<Result<ActionPlotPoint, String>> {
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

pub fn process_cpr_lines(cpr_points: &mut Vec<ActionPlotPoint>, csv_row: &ActionCsvRow) -> Option<Result<ActionPlotPoint, String>> {
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

fn check_pending_erroneous_action_marker(pending_error_marker: &RefCell<Option<(usize, ActionCsvRow)>>, row_idx: usize, current_row: &ActionCsvRow) -> Option<ActionPlotPoint> {
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

pub fn process_erroneous_action(state: &CsvProcessingState, row_idx: usize, current_row: &ActionCsvRow, ) -> Option<Result<ActionPlotPoint, String>> {
    if let Some(error_point) = check_pending_erroneous_action_marker(
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

pub fn process_action_point(current_row: &ActionCsvRow) -> Option<Result<ActionPlotPoint, String>> {
    if current_row.action_point {
        Some(Ok(ActionPlotPoint::Action(Action::new(current_row))))
    } else {
        None
    }
}

fn seek_erroneous_action_in_visited_rows(visited_rows_buffer: &VecDeque<ActionCsvRow>, error_marker_row: &ActionCsvRow, error_marker_row_idx: usize) -> Option<Result<ActionPlotPoint, String>> {
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