use std::{
    fs::File,
    io::{BufReader},
};
use mteam_dashboard_action_processor::process_csv;
use mteam_dashboard_action_processor::scatter_points::{ActionPlotPoint, PeriodType};
use mteam_dashboard_action_processor::debug_message::print_debug_message;
// fn read_csv_file_from_input() -> String {
//     println!("Enter the CSV file name:");
//     let mut input = String::new();
//     io::stdin().read_line(&mut input).unwrap();
//     input.trim().to_string()
// }

fn main() {
    // let file_name = read_csv_file_from_input();
    let file_name = "timeline-multiplayer-09182024.csv";
    match File::open(file_name) {
        Ok(file) => {
            let buffered = BufReader::new(file);
           for (row_idx, result) in process_csv(buffered, 10).enumerate() {
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