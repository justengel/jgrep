use std::io::Read;
use std::path::PathBuf;
use anyhow::{Error, Ok, Result};
use clap::Parser;
use serde_json::{self, Map, Value};
use flatten_serde_json::flatten;


type JsonMap = Map<String, Value>;


fn read_input(path: Option<PathBuf>) -> Result<String, Error>{
    let mut content: String = String::new();
    match path {
        Some(path) => {
            content = std::fs::read_to_string(path)?;
        },
        None => {
            std::io::stdin().lock().read_to_string(&mut content)?;
        },
    };

    Ok(content)
}


fn read_json(path: Option<PathBuf>) -> Result<JsonMap, Error> {
    let content: String = read_input(path)?;
    let data: JsonMap = flatten(&serde_json::from_str(&content)?);
    Ok(data)
}


fn split_keys(relative: Option<String>) -> Result<Vec<String>, Error> {
    let mut results: Vec<String> = Vec::new();
    let r_key: String = relative.unwrap_or(".".to_string());
    for key in r_key.split("::") {
        for k in key.split_inclusive(".."){
            if k != "" {
                results.push(k.to_string());
            }
        }

    }
    Ok(results)
}


fn find_match(json: &JsonMap, pattern: &str, relative: Option<String>) -> Result<(String, Value), Error> {
    let mut key: String = String::new();
    
    // Find the pattern
    for (k, v) in json.iter() {
        if k.contains(pattern){
            key.push_str(k);
            println!("Found content: {} {}", k, v.to_string());
            break
        }
    }

    // Find the relative key
    let mut parts: Vec<String> = key.split(".").map(|s| s.to_owned()).collect();
    for k in split_keys(relative)? {
        if k.eq("..") {
            parts.pop();
        } else if !k.eq(".") {
            parts.push(k);
        }
    }

    // Get the new key
    key = parts.join(".");
    let value = json.get(&key).unwrap_or(&Value::Null);

    Ok((key, value.to_owned()))
}


#[derive(Parser)]
struct Arguments {
    pattern: String,
    relative: Option<String>,
    path: Option<std::path::PathBuf>,
}


fn main() {
    let args = Arguments::parse();

    let data = read_json(args.path).unwrap();
    
    let (key, value) = find_match(&data, &args.pattern, args.relative).unwrap();

    println!("Relative content: {}, {}", key, value);
}
