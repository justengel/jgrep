import argparse
import json
import sys
import re
import pandas as pd
from collections.abc import MutableMapping
from typing import Optional, Dict, Any, Tuple

def read_input(path: Optional[str] = None) -> str:
    if path is None:
        return sys.stdin.read()
    else:
        with open(path, "r") as f:
            return f.read()


def read_json(path: Optional[str] = None) -> Dict[str, Any]:
    content = read_input(path)
    return flatten_dict(json.loads(content))


def flatten_dict(d: MutableMapping, parent_key: str = '', sep: str = '.'):
    return pd.json_normalize(d, sep=sep).to_dict(orient='records')[0]


def splitkeep(s, delimiter):
    split = s.split(delimiter)
    return [substr + delimiter for substr in split[:-1]] + [split[-1]]


def split_keys(relative: Optional[str] = None):
    for key in (relative or ".").split("::"):
        for k in splitkeep(key, ".."):
            if k:
                yield k


def find_match(data: Dict[str, Any], pattern: str, relative: Optional[str] = None) -> Tuple[str, Any]:
    # Find the pattern
    key = ""
    value = None
    for k, v in data.items():
        if pattern in k:
            key = k
            print(f"Found content: {k} {v}")
            break
    
    # Find the relative key
    parts = list(key.split("."))
    for k in split_keys(relative):
        if k == "..":
            parts.pop()
        elif k != ".":
            parts.append(k)


    # Get the new key
    key = ".".join(parts)
    value = data.get(key)
    return key, value


if __name__ == "__main__":
    P = argparse.ArgumentParser()
    P.add_argument("pattern")
    P.add_argument("relative", nargs="?", default=".")
    P.add_argument("path", nargs="?", default=None)
    ARGS = P.parse_args()

    data = read_json(ARGS.path)

    k, v = find_match(data, ARGS.pattern, ARGS.relative)
    print(f"Relative content: {k} {v}")