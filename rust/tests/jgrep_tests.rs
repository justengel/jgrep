use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

// Static JSON content matching test_jgrep.py
const EXAMPLE_JSON_CONTENT: &str = r#"
{
    "name": "Jane Smith",
    "age": 25,
    "address": {
        "street": "123 Main St",
        "city": "New York",
        "postalCode": "10001"
    },
    "phoneNumbers": [
        {
            "type": "home",
            "number": "212-555-1234"
        },
        {
            "type": "work",
            "number": "646-555-5678"
        }
    ]
}
"#;

// Deserialize JSON content for indexing
lazy_static::lazy_static! {
    static ref JSON_VALUE: Value = serde_json::from_str(EXAMPLE_JSON_CONTENT).unwrap();
}

// Format output to match Python's format_output, sorting keys
fn format_output(pairs: &[(&str, Value)], indent: Option<usize>) -> String {
    let mut lines = Vec::new();
    for (key, value) in pairs {
        let json_str = if let Some(_indent_level) = indent {
            let pretty = serde_json::to_string_pretty(value).unwrap();
            // Mimic main.rs's adjust_indent
            let mut result = Vec::new();
            for line in pretty.lines() {
                let trimmed = line.trim_start();
                let leading_spaces = line.len() - trimmed.len();
                let indent_level = leading_spaces / 2;
                let new_indent = " ".repeat(indent_level * _indent_level);
                result.push(format!("{}{}", new_indent, trimmed));
            }
            result.join("\n")
        } else {
            serde_json::to_string(value).unwrap()
        };
        lines.push((key.to_string(), json_str));
    }
    // Sort lines by key to match Python's order
    lines.sort_by(|a, b| a.0.cmp(&b.0));
    lines
        .into_iter()
        .map(|(key, json_str)| format!("{}: {}", key, json_str))
        .collect::<Vec<_>>()
        .join("\n") + "\n"
}

// Setup a temporary JSON file, mimicking setup_example_json
fn setup_example_json() -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(temp_file, "{}", EXAMPLE_JSON_CONTENT).expect("Failed to write to temp file");
    temp_file
}

// Run jgrep with arguments, capturing output and exit code
fn run_jgrep(args: &[&str], input_data: Option<&str>) -> (String, String, i32) {
    let mut command = Command::new("cargo")
        .args(&["run", "--"])
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::piped())
        .spawn()
        .expect("Failed to spawn cargo run");

    if let Some(input) = input_data {
        let mut stdin = command.stdin.take().expect("Failed to open stdin");
        write!(stdin, "{}", input).expect("Failed to write to stdin");
        stdin.flush().expect("Failed to flush stdin");
        drop(stdin); // Close stdin to prevent hanging
    }

    let output = command.wait_with_output().expect("Failed to wait for command");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let exit_code = output.status.code().unwrap_or(-1);

    (stdout, stderr, exit_code)
}

#[test]
fn test_single_key_file() {
    let temp_file = setup_example_json();
    let file_path = temp_file.path().to_str().unwrap();

    // Test: Match single key
    let expected = format_output(&[("phoneNumbers", JSON_VALUE["phoneNumbers"].clone())], None);
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "phoneNumbers"], None);
    assert_eq!(stdout, expected, "Single key match failed");
    assert_eq!(rc, 0, "Exit code mismatch for single key");

    // Test: Match nested key
    let expected = format_output(&[("address::street", JSON_VALUE["address"]["street"].clone())], None);
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "street"], None);
    assert_eq!(stdout, expected, "Nested key match failed");
    assert_eq!(rc, 0, "Exit code mismatch for nested key");

    // Test: No match for partial regex without -e
    let expected = "";
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "stre.*"], None);
    assert_eq!(stdout, expected, "Partial regex without -e failed");
    assert_eq!(rc, 0, "Exit code mismatch for partial regex");

    // Test: Match with extended regex
    let expected = format_output(&[("address::street", JSON_VALUE["address"]["street"].clone())], None);
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "-e", "stre.*"], None);
    assert_eq!(stdout, expected, "Extended regex match failed");
    assert_eq!(rc, 0, "Exit code mismatch for extended regex");
}

#[test]
fn test_single_key_regex_file() {
    let temp_file = setup_example_json();
    let file_path = temp_file.path().to_str().unwrap();

    // Test: Full match from start without end
    let expected = format_output(
        &[
            ("phoneNumbers", JSON_VALUE["phoneNumbers"].clone()),
            ("phoneNumbers::0", JSON_VALUE["phoneNumbers"][0].clone()),
            ("phoneNumbers::0::type", JSON_VALUE["phoneNumbers"][0]["type"].clone()),
            ("phoneNumbers::0::number", JSON_VALUE["phoneNumbers"][0]["number"].clone()),
            ("phoneNumbers::1", JSON_VALUE["phoneNumbers"][1].clone()),
            ("phoneNumbers::1::type", JSON_VALUE["phoneNumbers"][1]["type"].clone()),
            ("phoneNumbers::1::number", JSON_VALUE["phoneNumbers"][1]["number"].clone()),
        ],
        None,
    );
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "phone.*"], None);
    assert_eq!(stdout, expected, "Regex match from start failed");
    assert_eq!(rc, 0, "Exit code mismatch for regex start");

    // Test: Full match from start with end
    let expected = format_output(&[("phoneNumbers", JSON_VALUE["phoneNumbers"].clone())], None);
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "phone.*s"], None);
    assert_eq!(stdout, expected, "Regex match with end failed");
    assert_eq!(rc, 0, "Exit code mismatch for regex end");

    // Test: No match for non-starting regex
    let expected = "";
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "stre.*"], None);
    assert_eq!(stdout, expected, "Non-starting regex failed");
    assert_eq!(rc, 0, "Exit code mismatch for non-starting regex");

    // Test: Match any part of key
    let expected = format_output(&[("address::street", JSON_VALUE["address"]["street"].clone())], None);
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, ".*stre.*"], None);
    assert_eq!(stdout, expected, "Any part regex match failed");
    assert_eq!(rc, 0, "Exit code mismatch for any part regex");
}

#[test]
fn test_single_key_extended_regex_file() {
    let temp_file = setup_example_json();
    let file_path = temp_file.path().to_str().unwrap();

    // Test: Extended match from start without end
    let expected = format_output(
        &[
            ("phoneNumbers", JSON_VALUE["phoneNumbers"].clone()),
            ("phoneNumbers::0", JSON_VALUE["phoneNumbers"][0].clone()),
            ("phoneNumbers::0::type", JSON_VALUE["phoneNumbers"][0]["type"].clone()),
            ("phoneNumbers::0::number", JSON_VALUE["phoneNumbers"][0]["number"].clone()),
            ("phoneNumbers::1", JSON_VALUE["phoneNumbers"][1].clone()),
            ("phoneNumbers::1::type", JSON_VALUE["phoneNumbers"][1]["type"].clone()),
            ("phoneNumbers::1::number", JSON_VALUE["phoneNumbers"][1]["number"].clone()),
        ],
        None,
    );
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "-e", "phone.*"], None);
    assert_eq!(stdout, expected, "Extended regex start failed");
    assert_eq!(rc, 0, "Exit code mismatch for extended regex start");

    // Test: Extended match from start with end
    let expected = format_output(
        &[
            ("phoneNumbers", JSON_VALUE["phoneNumbers"].clone()),
            ("phoneNumbers::0", JSON_VALUE["phoneNumbers"][0].clone()),
            ("phoneNumbers::0::type", JSON_VALUE["phoneNumbers"][0]["type"].clone()),
            ("phoneNumbers::0::number", JSON_VALUE["phoneNumbers"][0]["number"].clone()),
            ("phoneNumbers::1", JSON_VALUE["phoneNumbers"][1].clone()),
            ("phoneNumbers::1::type", JSON_VALUE["phoneNumbers"][1]["type"].clone()),
            ("phoneNumbers::1::number", JSON_VALUE["phoneNumbers"][1]["number"].clone()),
        ],
        None,
    );
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "-e", "phone.*s"], None);
    assert_eq!(stdout, expected, "Extended regex with end failed");
    assert_eq!(rc, 0, "Exit code mismatch for extended regex end");

    // Test: Extended match non-starting
    let expected = format_output(&[("address::street", JSON_VALUE["address"]["street"].clone())], None);
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "-e", "stre.*"], None);
    assert_eq!(stdout, expected, "Extended non-starting regex failed");
    assert_eq!(rc, 0, "Exit code mismatch for non-starting extended regex");

    // Test: Extended match any part
    let expected = format_output(&[("address::street", JSON_VALUE["address"]["street"].clone())], None);
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "-e", ".*stre.*"], None);
    assert_eq!(stdout, expected, "Extended any part regex failed");
    assert_eq!(rc, 0, "Exit code mismatch for any part extended regex");
}

#[test]
fn test_single_key_piped() {
    let input_data = EXAMPLE_JSON_CONTENT;
    let expected = format_output(&[("phoneNumbers", JSON_VALUE["phoneNumbers"].clone())], None);
    let (stdout, stderr, rc) = run_jgrep(&["phoneNumbers"], Some(input_data));
    assert_eq!(stdout, expected, "Piped input match failed");
    assert_eq!(rc, 0, "Exit code mismatch for piped input");

    let (stdout, stderr, rc) = run_jgrep(&["-", "phoneNumbers"], Some(input_data));
    assert_eq!(stdout, expected, "Piped input match failed");
    assert_eq!(rc, 0, "Exit code mismatch for piped input");
}

#[test]
fn test_multiple_keys_file() {
    let temp_file = setup_example_json();
    let file_path = temp_file.path().to_str().unwrap();

    let expected = format_output(
        &[
            ("name", JSON_VALUE["name"].clone()),
            ("age", JSON_VALUE["age"].clone()),
            ("phoneNumbers", JSON_VALUE["phoneNumbers"].clone()),
        ],
        None,
    );
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "name", "age", "--key", "phoneNumbers"], None);
    assert_eq!(stdout, expected, "Multiple keys match failed");
    assert_eq!(rc, 0, "Exit code mismatch for multiple keys");
}

#[test]
fn test_value_search_file() {
    let temp_file = setup_example_json();
    let file_path = temp_file.path().to_str().unwrap();

    let expected = format_output(&[("phoneNumbers::0::number", JSON_VALUE["phoneNumbers"][0]["number"].clone())], None);
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "--value", "212-.*"], None);
    assert_eq!(stdout, expected, "Value search failed");
    assert_eq!(rc, 0, "Exit code mismatch for value search");
}

#[test]
fn test_case_insensitive_key() {
    let temp_file = setup_example_json();
    let file_path = temp_file.path().to_str().unwrap();

    let expected = format_output(&[("phoneNumbers", JSON_VALUE["phoneNumbers"].clone())], None);
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "PHONENUMBERS", "-i"], None);
    assert_eq!(stdout, expected, "Case-insensitive key match failed");
    assert_eq!(rc, 0, "Exit code mismatch for case-insensitive key");
}

#[test]
fn test_indent_option() {
    let temp_file = setup_example_json();
    let file_path = temp_file.path().to_str().unwrap();

    let expected = format_output(&[("phoneNumbers", JSON_VALUE["phoneNumbers"].clone())], Some(2));
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "phoneNumbers", "--indent", "2"], None);
    assert_eq!(stdout, expected, "Indent option failed");
    assert_eq!(rc, 0, "Exit code mismatch for indent option");
}

#[test]
fn test_delimiter_nested_key() {
    let temp_file = setup_example_json();
    let file_path = temp_file.path().to_str().unwrap();

    let expected = format_output(&[("address::street", JSON_VALUE["address"]["street"].clone())], None);
    let (stdout, _stderr, rc) = run_jgrep(&[file_path, "address::street"], None);
    assert_eq!(stdout, expected, "Delimiter nested key failed");
    assert_eq!(rc, 0, "Exit code mismatch for delimiter nested key");
}

#[test]
fn test_invalid_json() {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    writeln!(temp_file, "{{invalid}}").expect("Failed to write to temp file");
    let file_path = temp_file.path().to_str().unwrap();

    let (stdout, stderr, rc) = run_jgrep(&[file_path, "name"], None);
    assert_eq!(stdout, "", "Invalid JSON output mismatch");
    assert!(stderr.contains("Error: Invalid JSON"), "Invalid JSON error message missing");
    assert_eq!(rc, 1, "Exit code mismatch for invalid JSON");
}

#[test]
fn test_no_keys_or_values() {
    let temp_file = setup_example_json();
    let file_path = temp_file.path().to_str().unwrap();

    let (stdout, stderr, rc) = run_jgrep(&[file_path], None);
    assert_eq!(stdout, "", "No keys/values output mismatch");
    assert!(stderr.contains("Error: At least one key or value pattern must be specified"), "No keys/values error message missing");
    assert_eq!(rc, 1, "Exit code mismatch for no keys/values");
}