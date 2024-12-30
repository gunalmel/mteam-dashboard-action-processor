use std::collections::VecDeque;
use std::cell::RefCell;
use crate::action_csv_row::ActionCsvRow;
use crate::scatter_points::{ActionPlotPoint, PlotLocation};

pub struct CsvProcessingState {
    pub max_rows_to_check: usize,
    pub recent_rows: VecDeque<ActionCsvRow>,
    pub stage_boundaries: Vec<PlotLocation>,
    pub cpr_points: Vec<ActionPlotPoint>,
    pub pending_error_marker: RefCell<Option<(usize, ActionCsvRow)>>,
}

impl CsvProcessingState {
    pub fn new(max_rows_to_check: usize) -> Self {
        Self {
            max_rows_to_check,
            recent_rows: VecDeque::with_capacity(max_rows_to_check),
            stage_boundaries: vec![PlotLocation::default()],
            cpr_points: Vec::new(),
            pending_error_marker: RefCell::new(None),
        }
    }
}