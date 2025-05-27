#!/usr/bin/env python3
import argparse
import json
import os
import re
import select
import sys
from typing import Any, Dict, List, Optional, Tuple


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="JSON grep utility for key or value search"
    )
    parser.add_argument(
        "file", nargs="?", type=str, help="File to search (or pipe from stdin)"
    )
    parser.add_argument(
        "keys",
        nargs="*",
        help=r"Full match regex patterns for key paths (e.g., 'a::b::c')",
    )
    parser.add_argument(
        "--key",
        action="append",
        default=[],
        help="Additional full match regex pattern for key path",
    )
    parser.add_argument(
        "--value", action="append", default=[], help="Regex pattern for values"
    )
    parser.add_argument(
        "-r",
        "--relative-key",
        action="append",
        default=[],
        help="Find a key relative to the found key (Use `..` with the delimiter to traverse back nested keys)",
    )
    parser.add_argument(
        "--relative-delimiter",
        default="..",
        type=str,
        help="Delimiter used in the relative key to traverse back nested keys.",
    )
    parser.add_argument(
        "-i", "--ignore-case", action="store_true", help="Case-insensitive search"
    )
    parser.add_argument(
        "-e",
        "--extended-keys",
        action="append",
        default=[],
        help="Regex search allowing partial matches to keys",
    )
    parser.add_argument(
        "-x",
        "--extended-values",
        action="append",
        default=[],
        help="Regex search allowing partial matches to values",
    )
    parser.add_argument(
        "--delimiter",
        default="::",
        help="Delimiter for nested key search (default: '::')",
    )
    parser.add_argument(
        "--indent",
        type=int,
        default=None,
        help="Indent level for JSON output (default: None)",
    )
    parser.add_argument(
        "-c", "--count", action="store_true", help="Print count of matches"
    )
    return parser.parse_args()


def has_stdin():
    if os.name == "nt":  # Windows
        import msvcrt

        return msvcrt.kbhit() or not sys.stdin.isatty()
    else:  # Unix-like
        return sys.stdin in select.select([sys.stdin], [], [], 0)[0]


def load_json(file_path: str) -> Any:
    try:
        if file_path == "-":
            return json.load(sys.stdin)
        with open(file_path, "r") as f:
            return json.load(f)
    except json.JSONDecodeError as e:
        print(
            f"Error: Invalid JSON in {file_path if file_path != '-' else 'stdin'}: {e}",
            file=sys.stderr,
        )
        return None
    except FileNotFoundError:
        print(f"Error: File {file_path} not found", file=sys.stderr)
        return None


def parse_key(key_str: str, delimiter: str = "::") -> List[str]:
    """Convert a deliminated key string to a list of path components."""
    return key_str.split(delimiter)


def resolve_path(data, path, delimiter="::") -> Tuple[Any, Optional[List[str]]]:
    """Resolve a path in the data to get the value, or None if invalid."""
    current = data
    for part in path:
        try:
            if part.isdigit():
                try:
                    current = current[int(part)]
                    continue
                except (KeyError, ValueError, IndexError):
                    pass
            current = current[part]
        except (KeyError, ValueError, TypeError, IndexError):
            key = delimiter.join(path)
            print(
                f"Warning: Failed to resolve relative path {key!r} on {part!r}",
                file=sys.stderr,
            )
            return None, None
    return current, path


def apply_relative_path(
    key: str, rel_key: str, delimiter: str = "::", relative_delimiter: str = ".."
) -> List[str]:
    """Apply relative key to a base path to get a new path."""
    current_path = parse_key(key, delimiter)
    rel_parts = parse_key(rel_key, delimiter)

    for part in rel_parts:
        if part == relative_delimiter:
            if current_path:
                current_path.pop()
        else:
            current_path.append(part)

    return current_path


def apply_relative_keys(
    data: Dict[str, Any],
    matches: List[Dict[str, Any]],
    relative_keys: List[str],
    delimiter: str = "::",
    relative_delimiter: str = "..",
):
    """
    Modifies matches by applying a relative key to traverse up and down a dictionary.
    Args:
        data: Dictionary or list to look up values
        matches: List of match dictionaries with 'key' (string) and 'value'
        relative_keys: Strings specifying relative path (e.g., '..::key')
        delimiter: Delimiter used in key strings (default: '::')
        relative_delimiter: Delimiter used to traverse back nested keys (default: '..')
    Returns:
        List of new match dictionaries with updated keys and values
    """
    rel_matches = []
    for match in matches:
        for relative_key in relative_keys:
            # Apply the relative key to get the new path
            new_path = apply_relative_path(
                match["key"], relative_key, delimiter, relative_delimiter
            )

            # Resolve the new path in the data
            value, resolved_path = resolve_path(data, new_path, delimiter)

            if resolved_path is not None:
                rel_matches.append({"key": delimiter.join(new_path), "value": value})

    return rel_matches


def search_keys(
    data: Any,
    key_patterns: List[re.Pattern],
    delimiter: str = "::",
    prefix: str = "",
) -> List[Dict[str, Any]]:
    matches: List[Dict[str, Any]] = []
    if isinstance(data, dict):
        for k, v in data.items():
            current_prefix = f"{prefix}{k}"
            for pattern in key_patterns:
                exact_match = False
                if pattern.pattern.startswith("^") and pattern.pattern.endswith("$"):
                    exact_match = k == pattern.pattern[1:-1]
                if exact_match or pattern.search(current_prefix):
                    matches.append({"key": current_prefix, "value": v})
                new_prefix = f"{current_prefix}{delimiter}"
                matches += search_keys(v, key_patterns, delimiter, new_prefix)
    elif isinstance(data, list):
        for i, v in enumerate(data):
            current_prefix = f"{prefix}{i}"
            for pattern in key_patterns:
                if pattern.search(current_prefix):
                    matches.append({"key": current_prefix, "value": v})
                new_prefix = f"{current_prefix}{delimiter}"
                matches += search_keys(v, key_patterns, delimiter, new_prefix)
    return matches


def search_values(
    data: Any,
    value_patterns: List[re.Pattern],
    delimiter: str = "::",
    prefix: str = "",
) -> List[Dict[str, Any]]:
    matches: List[Dict[str, Any]] = []
    if isinstance(data, dict):
        for k, v in data.items():
            new_prefix = f"{prefix}{k}{delimiter}"
            matches += search_values(v, value_patterns, delimiter, new_prefix)
    elif isinstance(data, list):
        for i, v in enumerate(data):
            new_prefix = f"{prefix}{i}{delimiter}"
            matches += search_values(v, value_patterns, delimiter, new_prefix)
    elif isinstance(data, (str, int, float, bool)):
        str_data = str(data)
        for pattern in value_patterns:
            if pattern.search(str_data):
                matches.append({"key": prefix.rstrip(delimiter), "value": data})
    return matches


def process_json(
    file_path: str,
    keys: List[str],
    values: List[str],
    extended_keys: List[str],
    extended_values: List[str],
    delimiter: str = "::",
    relative_keys: Optional[List[str]] = None,
    relative_delimiter: str = "..",
    ignore_case: bool = False,
) -> List[Dict[str, Any]]:
    data = load_json(file_path)
    if data is None:
        return []

    matches: List[Dict[str, Any]] = []
    if keys or extended_keys:
        key_patterns = [
            re.compile(f"^{p}$", re.IGNORECASE if ignore_case else 0) for p in keys
        ] + [re.compile(p, re.IGNORECASE if ignore_case else 0) for p in extended_keys]
        matches += search_keys(data, key_patterns, delimiter)
    if values or extended_values:
        value_patterns = [
            re.compile(f"^{p}$", re.IGNORECASE if ignore_case else 0) for p in values
        ] + [
            re.compile(p, re.IGNORECASE if ignore_case else 0) for p in extended_values
        ]
        matches += search_values(data, value_patterns, delimiter)

    if relative_keys:
        return apply_relative_keys(
            data, matches, relative_keys, delimiter, relative_delimiter
        )
    return matches


def main():
    args = parse_args()

    keys = args.keys + args.key
    values = args.value
    relative_keys = args.relative_key
    extended_keys = args.extended_keys
    extended_values = args.extended_values
    delimiter = args.delimiter
    relative_delimiter = args.relative_delimiter
    ignore_case = args.ignore_case
    indent = args.indent
    get_count = args.count

    if os.path.isfile(args.file):
        # Use the given path
        file = args.file
    elif has_stdin():
        # Use stdin
        if args.file != "-" and args.file is not None:
            # Assume first argument is a key with stdin given
            keys.insert(0, args.file)
        file = "-"
    elif args.file != "-" and args.file is not None:
        # Invalid file path with no stdin, exit with error.
        print(
            f"Error: Invalid file {args.file!r}",
            file=sys.stderr,
        )
        sys.exit(1)
    else:
        # No file provided and no stdin, exit with error.
        print(
            f"Error: No file provided and no stdin available",
            file=sys.stderr,
        )
        sys.exit(1)


    if not (keys + extended_keys) and not (values + extended_values):
        print(
            "Error: At least one key or value pattern must be specified",
            file=sys.stderr,
        )
        sys.exit(1)

    matches = process_json(
        file,
        keys,
        values,
        extended_keys,
        extended_values,
        delimiter,
        relative_keys,
        relative_delimiter,
        ignore_case,
    )

    total_count = len(matches)
    if get_count:
        print(total_count)
    elif total_count > 0:
        for match in matches:
            print(f"{match['key']}: {json.dumps(match['value'], indent=indent)}")

    sys.exit(0 if total_count > 0 else 1)


if __name__ == "__main__":
    main()
