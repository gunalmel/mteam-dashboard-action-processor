use crate::action_csv_row::ActionCsvRow;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CsvRowTime {
    pub total_seconds: u32,
    pub date_string: String,
    pub timestamp: String,
}

#[derive(Debug, Default, Clone, PartialEq)]
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

#[derive(Debug, PartialEq)]
#[derive(Clone)]
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

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, Clone)]
#[derive(PartialEq)]
pub struct Action {
    pub location: PlotLocation,
    pub name: String,
    pub action_category: String,
    pub shock_value: String
}

impl Action {
    pub fn new(row: &ActionCsvRow) -> Self {
        Self {
            location:PlotLocation {
                timestamp: row.timestamp.clone().unwrap_or(CsvRowTime::default()),
                stage: row.parsed_stage.clone().unwrap_or(PlotLocation::default().stage),
            },
            name: row.action_name.clone(),
            action_category: row.action_category.clone(),
            shock_value: row.shock_value.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ErroneousAction {
    pub location: PlotLocation,
    pub name: String,
    pub action_category: String,
    pub shock_value: String,
    pub error_info: ErrorInfo
}

impl ErroneousAction {
    pub fn new(action_row: &ActionCsvRow, error_marker_row: &ActionCsvRow) -> Self {
        Self {
            location: PlotLocation::new(action_row),
            name: action_row.action_name.clone(),
            action_category: action_row.action_category.clone(),
            shock_value: action_row.shock_value.clone(),
            error_info: ErrorInfo::new(error_marker_row)
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct MissedAction {
    pub location: PlotLocation,
    pub action_name: String,
    pub error_info: ErrorInfo
}

impl MissedAction {
    pub(crate) fn new(row: &ActionCsvRow) -> MissedAction {
        MissedAction {
            location: PlotLocation::new(row),
            action_name: row.action_vital_name.clone(),
            error_info: ErrorInfo::new(row)
        }
    }
}

#[derive(Debug)]
#[derive(PartialEq, Clone)]
pub enum ActionPlotPoint {
    Error(ErroneousAction),
    Action(Action),
    StageBoundary(StageBoundary),
    MissedAction(MissedAction),
    CPR(Option<PlotLocation>, Option<PlotLocation>)
}

