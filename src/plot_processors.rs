use crate::action_csv_row::ActionCsvRow;
use crate::debug_message::print_debug_message;
use crate::detection::{can_mark_each_other, check_cpr, is_erroneous_action, is_error_action_marker, is_missed_action, is_stage_boundary, ERROR_MARKER_TIME_THRESHOLD};
use crate::plot_structures::{Action, ActionPlotPoint, ErroneousAction, MissedAction, PeriodType, PlotLocation};
use crate::processing_state::CsvProcessingState;
use std::cell::RefCell;
use std::collections::VecDeque;

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

pub fn process_stage_boundary(stage_boundary_points: &mut Vec<PlotLocation>, csv_row: &ActionCsvRow) -> Option<Result<ActionPlotPoint, String>> {
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
        start_location,
        PlotLocation::new(csv_row), // No more Option here
    )))
}

pub fn process_cpr_lines(cpr_points: &mut Vec<(PlotLocation, PlotLocation)>, csv_row: &ActionCsvRow) -> Option<Result<ActionPlotPoint, String>> {
    match check_cpr(&csv_row) {
        Some(_) => {
            let location = PlotLocation::new(csv_row);
            match cpr_points.pop() {
                Some(previous_cpr) => {
                    // Merge logic. We assume the first location in previous_cpr is the start
                    // and the current location is the end.
                    Some(Ok(ActionPlotPoint::Period(PeriodType::CPR, previous_cpr.0, location)))
                },
                None => {
                    // Start of CPR, store both start and "end" as the current location,
                    // end will be updated later.
                    cpr_points.push((location.clone(), location));
                    None
                }
            }
        }
        None => None
    }
}

#[cfg(test)]
mod tests{
    mod process_stage_boundary {
        use super::super::*;
        use crate::action_csv_row::ActionCsvRow;
        use crate::plot_structures::ActionPlotPoint;

        #[test]
        fn stage_begin() {
            let mut stage_boundary_points = Vec::new();
            
            let csv_row = ActionCsvRow {
                action_vital_name: "Stage 1".to_string(),
                parsed_stage: Some((1, "Stage 1".to_string())),
                ..Default::default()
            };

            let result = process_stage_boundary(&mut stage_boundary_points, &csv_row);
            
            assert!(result.is_some());
            if let Some(Ok(ActionPlotPoint::Period(PeriodType::Stage, start, end))) = result {
                assert_eq!(start.stage, (1, "Stage 1".to_string()));
                assert_eq!(end.stage, (1, "Stage 1".to_string()));
            } else {
                panic!("Expected ActionPlotPoint::Period with PeriodType::Stage");
            }
        }

        #[test]
        fn stage_end() {
            let mut stage_boundary_points = vec![PlotLocation::new(&ActionCsvRow {
                action_vital_name: "Stage 1".to_string(),
                parsed_stage: Some((1, "Stage 1".to_string())),
                ..Default::default()
            })];
            let csv_row = ActionCsvRow {
                action_vital_name: "Stage 2".to_string(),
                parsed_stage: Some((2, "Stage 2".to_string())),
                ..Default::default()
            };

            let result = process_stage_boundary(&mut stage_boundary_points, &csv_row);
           
            assert!(result.is_some());
            if let Some(Ok(ActionPlotPoint::Period(PeriodType::Stage, start, end))) = result {
                assert_eq!(start.stage, (2, "Stage 2".to_string()));
                assert_eq!(end.stage, (2, "Stage 2".to_string()));
            } else {
                panic!("Expected ActionPlotPoint::Period with PeriodType::Stage");
            }
        }

        #[test]
        fn not_stage_boundary() {
            let mut stage_boundary_points = Vec::new();
            let csv_row = ActionCsvRow {
                action_vital_name: "Not a stage boundary".to_string(),
                parsed_stage: None,
                ..Default::default()
            };

            let result = process_stage_boundary(&mut stage_boundary_points, &csv_row);
            assert!(result.is_none());
        }
    }
    
    mod process_cpr_lines {
        use super::super::*;
        use crate::action_csv_row::ActionCsvRow;
        use crate::plot_structures::{ActionPlotPoint, CsvRowTime, PeriodType, PlotLocation};
        #[test]
        fn start_cpr_period() {
            let mut cpr_points = Vec::new();
            let csv_row = ActionCsvRow {
                subaction_name: "Begin CPR".to_string(),
                // Add necessary fields to make check_cpr return Some value
                timestamp: Some(CsvRowTime{
                    total_seconds:120,
                    timestamp: "00:02:00".to_string(),
                    date_string: "2021-01-01 00:02:00".to_string(),
                }),
                ..Default::default()
            };

            let result = process_cpr_lines(&mut cpr_points, &csv_row);

            assert!(result.is_none());
            assert_eq!(cpr_points.len(), 1);
            assert_eq!(cpr_points[0].0.timestamp.total_seconds, csv_row.timestamp.unwrap().total_seconds);
        }

        #[test]
        fn end_cpr_period() {
            let mut cpr_points = vec![(PlotLocation::new(&ActionCsvRow {
                subaction_name: "End CPR".to_string(),
                ..Default::default()
            }), PlotLocation::new(&ActionCsvRow {
                subaction_name: "Begin CPR".to_string(),
                ..Default::default()
            }))];
            let csv_row = ActionCsvRow {
                subaction_name: "End CPR".to_string(),
                timestamp: Some(CsvRowTime{
                    total_seconds:120,
                    timestamp: "00:02:00".to_string(),
                    date_string: "2021-01-01 00:02:00".to_string(),
                }),
                ..Default::default()
            };

            let result = process_cpr_lines(&mut cpr_points, &csv_row);

            assert!(result.is_some());
            if let Some(Ok(ActionPlotPoint::Period(PeriodType::CPR, start, end))) = result {
                assert_eq!(0, start.timestamp.total_seconds);
                assert_eq!(csv_row.timestamp.unwrap().total_seconds, end.timestamp.total_seconds);
            } else {
                panic!("Expected ActionPlotPoint::Period with PeriodType::CPR");
            }
        }

        #[test]
        fn non_cpr_row() {
            let mut cpr_points = Vec::new();
            let csv_row = ActionCsvRow {
                action_vital_name: "Non-CPR".to_string(),
                // Add necessary fields to make check_cpr return None
                ..Default::default()
            };

            let result = process_cpr_lines(&mut cpr_points, &csv_row);

            assert!(result.is_none());
            assert!(cpr_points.is_empty());
        }
    }

    mod process_action_point{
        use super::super::*;
        use crate::action_csv_row::ActionCsvRow;
        use crate::plot_structures::ActionPlotPoint;

        #[test]
        fn action_point() {
            let csv_row = ActionCsvRow {
                action_point: true,
                action_name: "Action Point".to_string(),
                action_vital_name: "Action Point".to_string(),
                ..Default::default()
            };

            let result = process_action_point(&csv_row);

            assert!(result.is_some());
            if let Some(Ok(ActionPlotPoint::Action(action))) = result {
                assert_eq!(csv_row.action_name, action.name);
            } else {
                panic!("Expected ActionPlotPoint::Action");
            }
        }

        #[test]
        fn not_action_point() {
            let csv_row = ActionCsvRow {
                action_point: false,
                action_vital_name: "Not an Action Point".to_string(),
                ..Default::default()
            };

            let result = process_action_point(&csv_row);

            assert!(result.is_none());
        }
    }
    
    mod process_erroneous_action {
        
    }
}