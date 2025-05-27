use clap::{Arg, Command};
use regex::Regex;
use serde_json::Value;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::process;
use std::str::FromStr;

#[derive(Debug)]
struct Args {
    file: Option<String>,
    keys: Vec<String>,
    key: Vec<String>,
    value: Vec<String>,
    relative_key: Vec<String>,
    relative_delimiter: String,
    ignore_case: bool,
    extended_keys: Vec<String>,
    extended_values: Vec<String>,
    delimiter: String,
    indent: Option<usize>,
    count: bool,
}

fn parse_args() -> Args {
    let matches = Command::new("jgrep")
        .about("JSON grep utility for key or value search")
        .arg(
            Arg::new("file")
                .help("File to search (or pipe from stdin)")
                .required(false),
        )
        .arg(
            Arg::new("keys")
                .help("Full match regex patterns for key paths (e.g., 'a::b::c')")
                .num_args(0..)
                .required(false),
        )
        .arg(
            Arg::new("key")
                .long("key")
                .help("Additional full match regex pattern for key path")
                .num_args(1)
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("value")
                .long("value")
                .help("Regex pattern for values")
                .num_args(1)
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("relative_key")
                .long("relative-key")
                .short('r')
                .help("Find a key relative to the found key (Use `..` with the delimiter to traverse back nested keys)")
                .num_args(1)
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("relative_delimiter")
                .long("relative-delimiter")
                .default_value("..")
                .help("Delimiter used in the relative key to traverse back nested keys"),
        )
        .arg(
            Arg::new("ignore_case")
                .long("ignore-case")
                .short('i')
                .help("Case-insensitive search")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("extended_keys")
                .long("extended-keys")
                .short('e')
                .help("Regex search allowing partial matches to keys")
                .num_args(1)
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("extended_values")
                .long("extended-values")
                .short('x')
                .help("Regex search allowing partial matches to values")
                .num_args(1)
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("delimiter")
                .long("delimiter")
                .default_value("::")
                .help("Delimiter for nested key search"),
        )
        .arg(
            Arg::new("indent")
                .long("indent")
                .help("Indent level for JSON output")
                .value_parser(clap::value_parser!(usize)),
        )
        .arg(
            Arg::new("count")
                .long("count")
                .short('c')
                .help("Print count of matches")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    Args {
        file: matches.get_one::<String>("file").cloned(),
        keys: matches
            .get_many::<String>("keys")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default(),
        key: matches
            .get_many::<String>("key")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default(),
        value: matches
            .get_many::<String>("value")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default(),
        relative_key: matches
            .get_many::<String>("relative_key")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default(),
        relative_delimiter: matches
            .get_one::<String>("relative_delimiter")
            .cloned()
            .unwrap_or("..".to_string()),
        ignore_case: matches.get_flag("ignore_case"),
        extended_keys: matches
            .get_many::<String>("extended_keys")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default(),
        extended_values: matches
            .get_many::<String>("extended_values")
            .map(|vals| vals.cloned().collect())
            .unwrap_or_default(),
        delimiter: matches
            .get_one::<String>("delimiter")
            .cloned()
            .unwrap_or("::".to_string()),
        indent: matches.get_one::<usize>("indent").copied(),
        count: matches.get_flag("count"),
    }
}

fn has_stdin() -> bool {
    !atty::is(atty::Stream::Stdin)
}

fn load_json(file_path: &Option<String>) -> Result<Value, Box<dyn std::error::Error>> {
    let json_str = match file_path {
        Some(path) if path != "-" => {
            let file = File::open(path)?;
            let mut contents = String::new();
            io::BufReader::new(file).read_to_string(&mut contents)?;
            contents
        }
        _ => {
            let mut contents = String::new();
            io::BufReader::new(io::stdin()).read_to_string(&mut contents)?;
            contents
        }
    };

    serde_json::from_str(&json_str).map_err(|e| {
        eprintln!(
            "Error: Invalid JSON in {}: {}",
            file_path.as_ref().map_or("stdin", |p| p),
            e
        );
        e.into()
    })
}

fn parse_key(key_str: &str, delimiter: &str) -> Vec<String> {
    key_str.split(delimiter).map(String::from).collect()
}

fn resolve_path<'a>(
    data: &'a Value,
    path: &[String],
    delimiter: &str,
) -> Option<(&'a Value, Vec<String>)> {
    let mut current = data;
    let mut resolved_path = Vec::new();

    for part in path {
        match current {
            Value::Object(map) => {
                if let Some(value) = map.get(part) {
                    current = value;
                    resolved_path.push(part.clone());
                } else {
                    eprintln!(
                        "Warning: Failed to resolve relative path '{}' on '{}'",
                        path.join(delimiter),
                        part
                    );
                    return None;
                }
            }
            Value::Array(arr) => {
                if let Ok(idx) = usize::from_str(part) {
                    if let Some(value) = arr.get(idx) {
                        current = value;
                        resolved_path.push(part.clone());
                    } else {
                        eprintln!(
                            "Warning: Failed to resolve relative path '{}' on '{}'",
                            path.join(delimiter),
                            part
                        );
                        return None;
                    }
                } else {
                    eprintln!(
                        "Warning: Failed to resolve relative path '{}' on '{}'",
                        path.join(delimiter),
                        part
                    );
                    return None;
                }
            }
            _ => {
                eprintln!(
                    "Warning: Failed to resolve relative path '{}' on '{}'",
                    path.join(delimiter),
                    part
                );
                return None;
            }
        }
    }
    Some((current, resolved_path))
}

fn apply_relative_path(
    key: &str,
    rel_key: &str,
    delimiter: &str,
    relative_delimiter: &str,
) -> Vec<String> {
    let current_path = parse_key(key, delimiter);
    let rel_parts = parse_key(rel_key, delimiter);
    let mut path_queue = VecDeque::from(current_path);

    for part in rel_parts {
        if part == relative_delimiter {
            path_queue.pop_back();
        } else {
            path_queue.push_back(part);
        }
    }

    path_queue.into_iter().collect()
}

fn apply_relative_keys(
    data: &Value,
    matches: &[Match],
    relative_keys: &[String],
    delimiter: &str,
    relative_delimiter: &str,
) -> Vec<Match> {
    let mut rel_matches = Vec::new();

    for m in matches {
        for relative_key in relative_keys {
            let new_path = apply_relative_path(
                &m.key,
                relative_key,
                delimiter,
                relative_delimiter,
            );
            if let Some((value, resolved_path)) = resolve_path(data, &new_path, delimiter) {
                rel_matches.push(Match {
                    key: resolved_path.join(delimiter),
                    value: value.clone(),
                });
            }
        }
    }

    rel_matches
}

#[derive(Debug, Clone)]
struct Match {
    key: String,
    value: Value,
}

fn normalize_key(key: &str, delimiter: &str) -> String {
    let parts: Vec<&str> = key.split(delimiter).filter(|s| !s.is_empty()).collect();
    parts.join(delimiter)
}

fn search_keys(
    data: &Value,
    key_patterns: &[Regex],
    delimiter: &str,
    prefix: &str,
) -> Vec<Match> {
    let mut matches = Vec::new();

    match data {
        Value::Object(map) => {
            for (k, v) in map {
                let current_prefix = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}{}{}", prefix, delimiter, k)
                };
                for pattern in key_patterns {
                    let pattern_str = pattern.to_string();
                    let clean_pattern = pattern_str
                        .strip_prefix('^')
                        .and_then(|s| s.strip_suffix('$'))
                        .unwrap_or(&pattern_str);
                    if pattern.is_match(&current_prefix) || k == clean_pattern {
                        matches.push(Match {
                            key: normalize_key(&current_prefix, delimiter),
                            value: v.clone(),
                        });
                    }
                }
                let new_prefix = current_prefix;
                matches.extend(search_keys(v, key_patterns, delimiter, &new_prefix));
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let current_prefix = if prefix.is_empty() {
                    i.to_string()
                } else {
                    format!("{}{}{}", prefix, delimiter, i)
                };
                for pattern in key_patterns {
                    if pattern.is_match(&current_prefix) {
                        matches.push(Match {
                            key: normalize_key(&current_prefix, delimiter),
                            value: v.clone(),
                        });
                    }
                }
                let new_prefix = current_prefix;
                matches.extend(search_keys(v, key_patterns, delimiter, &new_prefix));
            }
        }
        _ => {}
    }

    matches
}

fn search_values(
    data: &Value,
    value_patterns: &[Regex],
    delimiter: &str,
    prefix: &str,
) -> Vec<Match> {
    let mut matches = Vec::new();

    match data {
        Value::Object(map) => {
            for (k, v) in map {
                let new_prefix = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}{}{}", prefix, delimiter, k)
                };
                matches.extend(search_values(v, value_patterns, delimiter, &new_prefix));
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let new_prefix = if prefix.is_empty() {
                    i.to_string()
                } else {
                    format!("{}{}{}", prefix, delimiter, i)
                };
                matches.extend(search_values(v, value_patterns, delimiter, &new_prefix));
            }
        }
        Value::String(s) => {
            for pattern in value_patterns {
                if pattern.is_match(s) {
                    matches.push(Match {
                        key: normalize_key(prefix.trim_end_matches(delimiter), delimiter),
                        value: data.clone(),
                    });
                }
            }
        }
        Value::Number(_) | Value::Bool(_) => {
            let str_data = data.to_string();
            for pattern in value_patterns {
                if pattern.is_match(&str_data) {
                    matches.push(Match {
                        key: normalize_key(prefix.trim_end_matches(delimiter), delimiter),
                        value: data.clone(),
                    });
                }
            }
        }
        _ => {}
    }

    matches
}

fn process_json(
    file_path: &Option<String>,
    keys: &[String],
    values: &[String],
    extended_keys: &[String],
    extended_values: &[String],
    delimiter: &str,
    relative_keys: &[String],
    relative_delimiter: &str,
    ignore_case: bool,
) -> Result<Vec<Match>, Box<dyn std::error::Error>> {
    let data = load_json(file_path)?;
    let mut matches = Vec::new();

    if !keys.is_empty() || !extended_keys.is_empty() {
        let mut key_patterns = Vec::new();
        for p in keys {
            let pattern = if ignore_case {
                Regex::new(&format!("(?i)^{}$", p))
            } else {
                Regex::new(&format!("^{}$", p))
            }?;
            key_patterns.push(pattern);
        }
        for p in extended_keys {
            let pattern = if ignore_case {
                Regex::new(&format!("(?i){}", p))
            } else {
                Regex::new(p)
            }?;
            key_patterns.push(pattern);
        }
        matches.extend(search_keys(&data, &key_patterns, delimiter, ""));
    }

    if !values.is_empty() || !extended_values.is_empty() {
        let mut value_patterns = Vec::new();
        for p in values {
            let pattern = if ignore_case {
                Regex::new(&format!("(?i)^{}$", p))
            } else {
                Regex::new(&format!("^{}$", p))
            }?;
            value_patterns.push(pattern);
        }
        for p in extended_values {
            let pattern = if ignore_case {
                Regex::new(&format!("(?i){}", p))
            } else {
                Regex::new(p)
            }?;
            value_patterns.push(pattern);
        }
        matches.extend(search_values(&data, &value_patterns, delimiter, ""));
    }

    if !relative_keys.is_empty() {
        matches = apply_relative_keys(
            &data,
            &matches,
            relative_keys,
            delimiter,
            relative_delimiter,
        );
    }

    Ok(matches)
}

fn adjust_indent(json_str: &str, indent: usize) -> String {
    let mut result = Vec::new();
    for line in json_str.lines() {
        let trimmed = line.trim_start();
        let leading_spaces = line.len() - trimmed.len();
        let indent_level = leading_spaces / 2; // Assuming 2 spaces per level from to_string_pretty
        let new_indent = " ".repeat(indent * indent_level);
        result.push(format!("{}{}", new_indent, trimmed));
    }
    result.join("\n")
}

fn main() {
    let args = parse_args();
    let mut keys = args.keys;
    keys.extend(args.key);
    let values = args.value;
    let relative_keys = args.relative_key;
    let extended_keys = args.extended_keys;
    let extended_values = args.extended_values;
    let delimiter = args.delimiter;
    let relative_delimiter = args.relative_delimiter;
    let ignore_case = args.ignore_case;
    let indent = args.indent;
    let count = args.count;

    let file = match args.file.as_deref() {
        // Use the given path
        Some(path) if Path::new(path).is_file() => args.file.clone(),
        // Use stdin if "-" or None without adding to keys.
        Some("-") | None if has_stdin() => Some("-".to_string()),
        // Use stdin and assume args.file is a key
        Some(_) if has_stdin() => {
            keys.insert(0, args.file.unwrap_or_default());
            Some("-".to_string())
        }
        // Invalid file path with no stdin, exit with error.
        Some(path) => {
            eprintln!("Error: Invalid file '{}'", path);
            process::exit(1);
        }
        // No file provided and no stdin, exit with error.
        None => {
            eprintln!("Error: No file provided and no stdin available");
            process::exit(1);
        }
    };

    if keys.is_empty() && extended_keys.is_empty() && values.is_empty() && extended_values.is_empty()
    {
        eprintln!("Error: At least one key or value pattern must be specified");
        process::exit(1);
    }

    let matches = match process_json(
        &file,
        &keys,
        &values,
        &extended_keys,
        &extended_values,
        &delimiter,
        &relative_keys,
        &relative_delimiter,
        ignore_case,
    ) {
        Ok(m) => m,
        Err(_) => process::exit(1),
    };

    let total_count = matches.len();
    if count {
        println!("{}", total_count);
    } else if total_count > 0 {
        for m in &matches {
            let json_output = if let Some(indent_level) = indent {
                let pretty = serde_json::to_string_pretty(&m.value)
                    .unwrap_or_else(|_| m.value.to_string());
                adjust_indent(&pretty, indent_level)
            } else {
                serde_json::to_string(&m.value).unwrap_or_else(|_| m.value.to_string())
            };
            println!("{}: {}", normalize_key(&m.key, &delimiter), json_output);
        }
    }

    // Exit with 0 even if no matches, unless an error occurred
    process::exit(0);
}