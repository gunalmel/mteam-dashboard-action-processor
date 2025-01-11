use csv::StringRecord;
use std::collections::VecDeque;
use crate::action_csv_row::ActionCsvRow;
use crate::debug_message::print_debug_message;
use crate::plot_processors::{process_action_point, process_cpr_lines, process_erroneous_action, process_stage_boundary};
use crate::plot_structures::ActionPlotPoint;
use crate::processing_state::CsvProcessingState;

fn parse_csv_row(result: Result<StringRecord, csv::Error>) -> Result<ActionCsvRow, String> {
    result
        .and_then(|raw_row| {
            let mut csv_row: ActionCsvRow = raw_row.deserialize(None)?;
            csv_row.post_deserialize();
            Ok(csv_row)
        })
        .map_err(|e| format!("Could not deserialize row: {}", e))
}

pub fn process_csv_row(
    row_idx: usize,
    result: Result<StringRecord, csv::Error>,
    state: &mut CsvProcessingState,
) -> Option<Result<ActionPlotPoint, String>> {
    let current_row = match parse_csv_row(result) {
        Ok(row) => row,
        Err(e) => return Some(Err(e)),
    };

    update_recent_rows(&current_row, &mut state.recent_rows, state.max_rows_to_check);

    process_stage_boundary(&mut state.stage_boundaries, &current_row)
        .or_else(|| process_cpr_lines(&mut state.cpr_points, &current_row))
        .or_else(|| process_erroneous_action(state, row_idx, &current_row))
        .or_else(|| process_action_point(&current_row))
        // .or_else(|| log_skipped_row(row_idx))
}

fn update_recent_rows(current_row: &ActionCsvRow, recent_rows: &mut VecDeque<ActionCsvRow>, max_rows: usize) {
    if recent_rows.len() >= max_rows {
        recent_rows.pop_front();
    }
    recent_rows.push_back(current_row.clone());
}

fn log_skipped_row(row_idx: usize) -> Option<Result<ActionPlotPoint, String>> {
    print_debug_message!(
        "{} skipped line. Cannot be mapped to a point plotted on a graph.",
        row_idx + 2
    );
    None
}