use std::io::Read;
use crate::csv_reader::initialize_csv_reader;
use crate::row_processing::process_csv_row;
use crate::scatter_points::ActionPlotPoint;
use crate::state_management::CsvProcessingState;
pub fn process_csv<'r, R>(
    reader: R,
    max_rows_to_check: usize,
) -> Box<dyn Iterator<Item = Result<ActionPlotPoint, String>> + 'r>
where
    R: Read + 'r,
{
    let csv_reader = match initialize_csv_reader(reader) {
        Ok(r) => r,
        Err(e) => return Box::new(vec![Err(e)].into_iter()),
    };

    let mut state = CsvProcessingState::new(max_rows_to_check);

    Box::new(
        csv_reader
            .into_records()
            .enumerate()
            .filter_map(move |(row_idx, result)| process_csv_row(row_idx, result, &mut state)),
    )
}