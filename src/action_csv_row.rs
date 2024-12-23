use csv::Reader;
// This lets us write `#[derive(Deserialize)]`.
use serde::{Deserialize, Deserializer};
use std::io::Read;

/*
 * Used by serde macros to deserialize a non-empty string from a CSV file.
 */
fn non_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<String> = Option::deserialize(deserializer)?;
    match value {
        Some(s) if !s.trim().is_empty() => Ok(s),
        _ => Err(serde::de::Error::custom("Field cannot be empty")),
    }
}
pub static COLUMN_NAMES: [&str; 9] = [
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
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")] //interpret each field in PascalCase, where the first letter of the field is capitalized
pub(crate) struct ActionCsvRow {
    #[serde(rename = "Time Stamp[Hr:Min:Sec]", deserialize_with = "non_empty_string")]
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
    speech_command: Option<String>,
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
        use crate::action_csv_row::validate_header;
        use crate::action_csv_row::tests::assert_header_check;

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
        use std::io::{self, Read};
        use csv::Reader;
        use crate::action_csv_row::apply_validation;

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
