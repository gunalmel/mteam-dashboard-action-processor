use crate::action_csv_row::ActionCsvRow;

#[derive(Debug, Clone, Default)]
pub struct CsvRowTime {
    pub total_seconds: u32,
    pub date_string: String,
    pub timestamp: String,
}

#[derive(Debug, Default)]
pub struct PlotLocation {
    pub timestamp: CsvRowTime,
    pub stage: (u32, String)
}

impl PlotLocation {
    pub fn new(row: &ActionCsvRow) -> Self {
        Self {
            timestamp: row.timestamp.clone().unwrap_or(CsvRowTime::default()),
            stage: row.parsed_stage.clone().unwrap_or(PlotLocation::default().stage),
        }
    }
}

#[derive(Debug)]
pub struct ErrorInfo {
    pub action_rule: String,
    pub violation: String,
    pub advice: String
}

impl ErrorInfo {
    pub fn new(row: &ActionCsvRow) -> Self {
        Self {
            action_rule: row.subaction_name.clone(),
            violation: row.score.clone(),
            advice: row.speech_command.clone(),
        }
    }
}

#[derive(Debug)]
pub struct StageBoundary {
    pub location: PlotLocation
}

impl StageBoundary {
    pub fn new(row: &ActionCsvRow) -> Self {
        Self { 
            location: PlotLocation::new(row)
        }
    }
}

#[derive(Debug)]
pub struct Action {
    pub location: PlotLocation,
    pub category_name: String,
    pub name: String,
    pub shock_value: String
}

impl Action {
    pub fn new(row: &ActionCsvRow) -> Self {
        Self {
            location:PlotLocation {
                timestamp: row.timestamp.clone().unwrap_or(CsvRowTime::default()),
                stage: row.parsed_stage.clone().unwrap_or(PlotLocation::default().stage),
            },
            category_name: row.action_vital_name.clone(),
            name: row.subaction_name.clone(),
            shock_value: "this needs to be implemented".to_string(),
        }
    }
}

#[derive(Debug)]
pub struct ErroneousAction {
    pub location: PlotLocation,
    pub category_name: String,
    pub name: String,
    pub shock_value: String,
    pub error_info: ErrorInfo
}

impl ErroneousAction {
    pub fn new(action_row: &ActionCsvRow, error_marker_row: &ActionCsvRow) -> Self {
        Self {
            location: PlotLocation::new(action_row),
            category_name: action_row.action_vital_name.clone(),
            name: action_row.subaction_name.clone(),
            shock_value: "this needs to be implemented".to_string(),
            error_info: ErrorInfo::new(error_marker_row)
        }
    }
}

#[derive(Debug)]
pub struct ErrorActionMarker {
    pub location: PlotLocation,
    pub error_info: ErrorInfo
}

#[derive(Debug)]
pub struct MissedAction {
    pub location: PlotLocation,
    pub error_info: ErrorInfo
}

impl MissedAction {
    pub(crate) fn new(row: &ActionCsvRow) -> MissedAction {
        MissedAction {
            location: PlotLocation::new(row),
            error_info: ErrorInfo::new(row)
        }
    }
}

#[derive(Debug)]
pub enum ActionPlotPoint {
    Error(ErroneousAction),
    Action(Action),
    StageBoundary(StageBoundary),
    MissedAction(MissedAction),
    CPRMarker(PlotLocation, PlotLocation)
}

