//tutorial-read-01.rs
use std::{
    env,
    error::Error,
    ffi::OsString,
    fs::File,
    process,
};
// This lets us write `#[derive(Deserialize)]`.
use serde::Deserialize;

// We don't need to derive `Debug` (which doesn't require Serde), but it's a
// good habit to do it for all your types.
//
// Notice that the field names in this struct are NOT in the same order as
// the fields in the CSV data!
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")] //interpret each field in PascalCase, where the first letter of the field is capitalized
struct ActionRecord {
    #[serde(rename = "Time Stamp[Hr:Min:Sec]")]
    timestamp: String,
    #[serde(rename = "Action/Vital Name")]
    action_name: Option<String>,
    #[serde(rename = "SubAction Time[Min:Sec]")]
    subaction_time: Option<String>,
    #[serde(rename = "SubAction Name")]
    subaction_name: Option<String>,
    #[serde(rename = "Score")]
    score: Option<String>,
    #[serde(rename = "Old Value")]
    old_value: Option<String>,
    #[serde(rename = "New Value")]
    new_value: Option<String>,
    #[serde(deserialize_with = "csv::invalid_option")]
    username: Option<String>,
    #[serde(rename = "Speech Command", deserialize_with = "csv::invalid_option")]
    // #[serde(deserialize_with = "csv::invalid_option")]
    speech_command: Option<String>
}

fn run() -> Result<(), Box<dyn Error>> {
    let file_path = get_first_arg()?;
    let file = File::open(file_path)?;
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_reader(file);
    {
        // We nest this call in its own scope because of lifetimes.
        let headers = rdr.headers()?;
        println!("{:?}", headers);
    }
    // for result in rdr.records() {
    //     let record = result?;
    //     println!("{:?}", record);
    // }
    for result in rdr.deserialize() {
        let record: ActionRecord = result?;
        // println!("{:?}", record);
        // Try this if you don't like each record smushed on one line:
        println!("{:#?}", record);
    }
    // // We can ask for the headers at any time. There's no need to nest this
    // // call in its own scope because we never try to borrow the reader again.
    // let headers = rdr.headers()?;
    // println!("{:?}", headers);
    Ok(())
}

/// Returns the first positional argument sent to this process. If there are no
/// positional arguments, then this returns an error.
fn get_first_arg() -> Result<OsString, Box<dyn Error>> {
    match env::args_os().nth(1) {
        None => Err(From::from("expected 1 argument, but got none")),
        Some(file_path) => Ok(file_path),
    }
}

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        process::exit(1);
    }
}