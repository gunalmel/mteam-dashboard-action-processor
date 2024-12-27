use crate::scatter_points::CsvRowTime;
use crate::util::{extract_stage_name, is_action_row, is_error_action_marker, is_missed_action, is_stage_boundary, parse_time, process_action_name};
use csv::Reader;
// This lets us write `#[derive(Deserialize)]`.
use serde::{Deserialize, Deserializer};
use std::fmt::{Display, Formatter};
use std::io::Read;
/*
 * Used by serde macros to deserialize a non-empty string from a CSV file.
 */
fn non_empty_string<'de, D>(deserializer: D) -> Result<Option<CsvRowTime>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<String> = Option::deserialize(deserializer)?;
    match value {
        Some(s) if !s.trim().is_empty() => Ok(parse_time(&s[..])),
        _ => Err(serde::de::Error::custom("Field cannot be empty")),
    }
}

pub const COLUMN_NAMES: [&str; 9] = [
    "Time Stamp[Hr:Min:Sec]",
    "Action/Vital Name",
    "SubAction Time[Min:Sec]",
    "SubAction Name",
    "Score",
    "Old Value",
    "New Value",
    "Username",
    "Speech Command",
];
#[derive(Default, Debug, Deserialize, Clone)]
#[serde(rename_all = "PascalCase")] //interpret each field in PascalCase, where the first letter of the field is capitalized
pub struct ActionCsvRow {
    #[serde(rename = "Time Stamp[Hr:Min:Sec]", deserialize_with = "non_empty_string")]
    pub timestamp: Option<CsvRowTime>,
    #[serde(rename = "Action/Vital Name")]
    pub action_vital_name: String,
    #[serde(default, rename = "SubAction Time[Min:Sec]")]
    pub subaction_time: String,
    #[serde(default, rename = "SubAction Name")]
    pub subaction_name: String,
    #[serde(default, rename = "Score")]
    pub score: String,
    #[serde(default, rename = "Old Value")]
    pub old_value: String,
    #[serde(default, rename = "New Value")]
    pub new_value: String,
    #[serde(default)]
    pub username: String,
    #[serde(default, rename = "Speech Command")]
    pub speech_command: String,
    
    #[serde(skip)]
    pub parsed_stage: Option<(u32, String)>,
    #[serde(skip)]
    pub action_name: String,
    #[serde(skip)]
    pub action_category: String,
    #[serde(skip)]
    pub shock_value: String,
    #[serde(skip)]
    pub action_point: bool,
    #[serde(skip)]
    pub stage_boundary: bool,
    #[serde(skip)]
    pub error_action_marker: bool,
    #[serde(skip)]
    pub missed_action_marker: bool,
}

impl Display for ActionCsvRow {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ActionCsvRow {{ timestamp: {:?}, action_vital_name: {:?}, subaction_time: {:?}, subaction_name: {:?}, score: {:?}, old_value: {:?}, new_value: {:?}, username: {:?}, speech_command: {:?}, parsed_stage: {:?}, action_name: {:?}, action_category: {:?}, shock_value: {:?}, action_point: {:?}, stage_boundary: {:?}, error_action_marker: {:?}, missed_action_marker: {:?} }}",
            self.timestamp,
            self.action_vital_name,
            self.subaction_time,
            self.subaction_name,
            self.score,
            self.old_value,
            self.new_value,
            self.username,
            self.speech_command,
            self.parsed_stage,
            self.action_name,
            self.action_category,
            self.shock_value,
            self.action_point,
            self.stage_boundary,
            self.error_action_marker,
            self.missed_action_marker
        )
    }
}

impl ActionCsvRow {
    pub fn post_deserialize(&mut self) {
        self.parsed_stage = extract_stage_name(&self.action_vital_name);
        self.action_point = is_action_row(&self);
        self.stage_boundary = is_stage_boundary(&self);
        self.error_action_marker = is_error_action_marker(&self);
        self.missed_action_marker = is_missed_action(&self);
        let processed_action_name = process_action_name(&self.subaction_name);
        self.action_name = processed_action_name.0;
        self.action_category = processed_action_name.1;
        self.shock_value = processed_action_name.2;
    }
}
type HeaderValidatorType = fn(&[&str], &[&str]) -> Result<(), String>;
fn validate_header(headers: &[&str], expected_headers: &[&str]) -> Result<(), String> {
    let mut headers_iter = headers.iter().map(|h| h.to_lowercase());
    let mut expected_iter = expected_headers.iter().map(|h| h.to_lowercase());

    if expected_iter.all(|expected| headers_iter.next() == Some(expected)) {
        Ok(())
    } else {
        let err = format!(
            "Line {:?}: expected {:?} as the header row of csv but got {:?}",
            1, expected_headers, headers
        );
        Err(err)
    }
}

fn apply_validation<R: Read>(reader: &mut Reader<R>, validate: HeaderValidatorType) -> Result<(), String> {
    match reader.headers() {
        Ok(headers) => {
            let headers = headers.iter().collect::<Vec<_>>();
            validate(&headers, &COLUMN_NAMES)
        }
        Err(e) => Err(e.to_string())
    }
}

fn build_csv_header_validator<R: Read>(validate: HeaderValidatorType) -> impl Fn(Box<&mut Reader<R>>) -> Result<(), String> {
    move |mut reader| apply_validation(reader.as_mut(), validate)
}

pub fn validate_csv_header<R: Read>(reader: &mut Reader<R>) -> Result<(), String> {
    build_csv_header_validator(validate_header)(Box::new(reader)) 
}

#[cfg(test)]
mod tests {
    fn assert_header_check(headers: &[&str], actual: Result<(), String>, expected_headers: &[&str]) {
        assert!(actual.is_err());
        let message: String = actual.unwrap_err();
        assert_eq!(message, format!("Line {:?}: expected {:?} as the header row of csv but got {:?}", 1, expected_headers, headers));
    }

    mod invalid_header_tests {
        use crate::action_csv_row::tests::assert_header_check;
        use crate::action_csv_row::validate_header;

        #[test]
        fn test_check_headers_missing_header() {
            let headers = ["Time Stamp[Hr:Min:Sec]", "Action/Vital Name"];
            let expected_headers = ["Time Stamp[Hr:Min:Sec]", "Action/Vital Name", "Score"];

            assert_header_check(
                &headers,
                validate_header(&headers, &expected_headers),
                &expected_headers,
            );
        }

        #[test]
        fn test_check_headers_different_order() {
            let headers = [
                "Action/Vital Name",
                "Time Stamp[Hr:Min:Sec]",
                "SubAction Time[Min:Sec]",
            ];
            let expected_headers = [
                "Time Stamp[Hr:Min:Sec]",
                "Action/Vital Name",
                "SubAction Time[Min:Sec]",
            ];

            assert_header_check(
                &headers,
                validate_header(&headers, &expected_headers),
                &expected_headers,
            );
        }

        #[test]
        fn test_check_headers_unknown_header() {
            let headers = [
                "Time Stamp[Hr:Min:Sec]",
                "Action/Vital Name",
                "Unknown Header",
                "SubAction Time[Min:Sec]",
            ];
            let expected_headers = [
                "Time Stamp[Hr:Min:Sec]",
                "Action/Vital Name",
                "SubAction Time[Min:Sec]",
            ];

            assert_header_check(
                &headers,
                validate_header(&headers, &expected_headers),
                &expected_headers,
            );
        }
    }

    mod valid_header_tests {
        use crate::action_csv_row::validate_header;

        #[test]
        fn test_check_headers_matching() {
            let headers = [
                "Time Stamp[Hr:Min:Sec]",
                "Action/Vital Name",
                "SubAction Time[Min:Sec]",
            ];
            let expected_headers = [
                "Time Stamp[Hr:Min:Sec]",
                "Action/Vital Name",
                "SubAction Time[Min:Sec]",
            ];

            assert!(validate_header(&headers, &expected_headers).is_ok());
        }

        #[test]
        fn test_check_headers_matching_case_insensitive() {
            let headers = [
                "time Stamp[Hr:Min:Sec]",
                "ActioN/Vital Name",
                "subAction time[min:sec]",
            ];
            let expected_headers = [
                "Time Stamp[Hr:Min:Sec]",
                "Action/Vital Name",
                "SubAction Time[Min:Sec]",
            ];

            assert!(validate_header(&headers, &expected_headers).is_ok());
        }

        #[test]
        fn test_check_headers_matching_extra_header() {
            let headers = [
                "time Stamp[Hr:Min:Sec]",
                "ActioN/Vital Name",
                "subAction time[min:sec]",
                "Extra Column",
            ];
            let expected_headers = [
                "Time Stamp[Hr:Min:Sec]",
                "Action/Vital Name",
                "SubAction Time[Min:Sec]",
            ];

            assert!(validate_header(&headers, &expected_headers).is_ok());
        }
    }

    mod tests_apply_validation {
        use crate::action_csv_row::apply_validation;
        use csv::Reader;
        use std::io::{self, Read};

        // Custom reader that always returns an error
        struct ValidReader;
        impl Read for ValidReader {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Ok(0)
            }
        }
        struct ErrorReader;

        impl Read for ErrorReader {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::Other, "Simulated read error"))
            }
        }

        #[test]
        fn test_could_not_read_headers() {
            let mut csv_reader = Reader::from_reader(ErrorReader);
            let mock_validate = |_: &[&str], _: &[&str]| -> Result<(), String> { unreachable!() };

            let result = apply_validation(&mut csv_reader, mock_validate);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Simulated read error");
        }

        #[test]
        fn test_read_invalid_headers() {
            let mut csv_reader = Reader::from_reader(ValidReader);
            let mock_validate = |_: &[&str], _: &[&str]| -> Result<(), String> {
                Err("Validation error".to_string())
            };

            let result = apply_validation(&mut csv_reader, mock_validate);

            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Validation error");
        }

        #[test]
        fn test_read_valid_headers() {
            let mut csv_reader = Reader::from_reader(ValidReader);
            let mock_validate = |_: &[&str], _: &[&str]| -> Result<(), String> {
                Ok(())
            };

            let result = apply_validation(&mut csv_reader, mock_validate);

            assert!(result.is_ok());
        }
    }
}
