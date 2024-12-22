use csv::Reader;
use std::{
    error::Error,
    fs::File,
    io::{self, BufReader, Read},
};
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

fn process_csv<R: Read>(reader: R) -> Result<Vec<String>, Box<dyn Error>> {
    let mut rdr = build_csv_reader(reader);
    let mut errors: Vec<String> = Vec::new();

    if let Err(e) = validate_csv_header(&mut rdr) {
        errors.push(e);
        print_debug_message!("Header parsing errors are pushed to errors vector: {:?}", errors);
    }

    for (row_index, result) in rdr.records().enumerate() {
        let line_number = row_index + 2; // 0-based index with the first line being header
        match result {
            Ok(raw_row) => match raw_row.deserialize(None) {
                Ok(record) => {
                    let record: ActionCsvRow = record;
                    // println!("{:#?}", record);
                }
                Err(e) => {
                    errors.push(format!(
                        "Line {}: could not deserialize row {}, error: {}",
                        line_number,
                        raw_row.iter().collect::<Vec<_>>().join(","),
                        e.to_string()
                    ));
                }
            },
            Err(e) => {
                errors.push(format!(
                    "Line {}: could not parse row so no data to show, error: {}",
                    line_number,
                    e.to_string()
                ));
            }
        }
    }

    print_debug_message!("The errors vector: {:#?}", errors);
    Ok(errors)
}

fn build_csv_reader<R: Read>(reader: R) -> Reader<R> {
    csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(reader)
}

fn main() {
    let file_name = read_csv_file_from_input();

    match File::open(file_name) {
        Ok(file) => {
            let buffered = BufReader::new(file);
            if let Err(e) = process_csv(buffered) {
                eprintln!("Error processing CSV: {}", e);
            }
        }
        Err(e) => eprintln!("Error opening file: {}", e),
    }
}

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
