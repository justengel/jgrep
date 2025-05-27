import json
import os
import subprocess
import sys
from io import StringIO
from unittest import mock

import pytest

# Path to example.json
EXAMPLE_JSON = "example.json"

# Content of example.json
EXAMPLE_JSON_CONTENT = {
    "name": "Jane Smith",
    "age": 25,
    "address": {"street": "123 Main St", "city": "New York", "postalCode": "10001"},
    "phoneNumbers": [
        {"type": "home", "number": "212-555-1234"},
        {"type": "work", "number": "646-555-5678"},
    ],
}


@pytest.fixture(scope="function")
def setup_example_json():
    # Create example.json for file-based tests
    with open(EXAMPLE_JSON, "w") as f:
        json.dump(EXAMPLE_JSON_CONTENT, f)
    yield
    # Clean up example.json
    if os.path.exists(EXAMPLE_JSON):
        os.remove(EXAMPLE_JSON)


class Capturing(list):
    def __init__(self, stdio="out"):
        super(Capturing, self).__init__()
        self.stdattr = f"std{stdio}"
        self._stringio = StringIO()
        self._stdobj = getattr(sys, self.stdattr)

    def __enter__(self, stdio="out"):
        setattr(sys, self.stdattr, self._stringio)
        return self

    def __exit__(self, *args):
        self.extend(self._stringio.getvalue().splitlines())
        setattr(sys, self.stdattr, self._stdobj)


@pytest.fixture
def run_jgrep():
    def _run_jgrep(
        args, input_data=None, output=None, error=None, returncode=None, debug=False
    ):
        if debug:
            import jgrep

            exitcode = -1
            with Capturing("out") as stdout, Capturing("err") as stderr, mock.patch(
                "sys.argv", ["jgrep"] + args
            ):
                try:
                    jgrep.main()
                except SystemExit as e:
                    exitcode = e.code
            stdout = "\n".join(stdout) + "\n"
            stderr = "\n".join(stderr) + "\n"

        else:
            cmd = ["python", "-m", "jgrep"] + args
            process = subprocess.run(
                cmd, input=input_data, text=True, capture_output=True
            )
            stdout = process.stdout
            stderr = process.stderr
            exitcode = process.returncode

        if output:
            msg = None
            if not error:
                msg = stderr.strip()
            assert stdout == output, msg
        if error:
            assert stderr == error
        if returncode:
            assert exitcode == returncode
        return stdout, stderr, exitcode

    return _run_jgrep


def format_output(*args, indent=None, **kwargs):
    content = {}
    for arg in args:
        if isinstance(arg, dict):
            content.update(arg)
        elif isinstance(arg, (list, tuple)) and len(arg) == 2:
            content[arg[0]] = arg[1]
    content.update(kwargs)
    out = "\n".join(
        f"{key}: {json.dumps(data, indent=indent)}" for key, data in content.items()
    )
    return out + "\n"


def test_single_key_file(setup_example_json, run_jgrep):
    expected = format_output(phoneNumbers=EXAMPLE_JSON_CONTENT["phoneNumbers"])
    run_jgrep([EXAMPLE_JSON, "phoneNumbers"], output=expected, returncode=0)

    # match on given key
    expected = format_output(("address::street", "123 Main St"))
    run_jgrep([EXAMPLE_JSON, "street"], output=expected, returncode=0)

    # Do not match on fullmatch regex (use -e if you want this to match)
    expected = ""
    run_jgrep([EXAMPLE_JSON, "stre.*"], output=expected, returncode=0)

    # Do not match on fullmatch regex (use -e if you want this to match)
    expected = format_output(("address::street", "123 Main St"))
    run_jgrep([EXAMPLE_JSON, "-e", "stre.*"], output=expected, returncode=0)


def test_single_key_regex_file(setup_example_json, run_jgrep):
    # Fullmatch from start of string without specifying an end
    expected = format_output(
        ("phoneNumbers", EXAMPLE_JSON_CONTENT["phoneNumbers"]),
        ("phoneNumbers::0", EXAMPLE_JSON_CONTENT["phoneNumbers"][0]),
        ("phoneNumbers::0::type", EXAMPLE_JSON_CONTENT["phoneNumbers"][0]["type"]),
        ("phoneNumbers::0::number", EXAMPLE_JSON_CONTENT["phoneNumbers"][0]["number"]),
        ("phoneNumbers::1", EXAMPLE_JSON_CONTENT["phoneNumbers"][1]),
        ("phoneNumbers::1::type", EXAMPLE_JSON_CONTENT["phoneNumbers"][1]["type"]),
        ("phoneNumbers::1::number", EXAMPLE_JSON_CONTENT["phoneNumbers"][1]["number"]),
    )
    run_jgrep([EXAMPLE_JSON, "phone.*"], output=expected, returncode=0)

    # Fullmatch from start of string with specifying an end
    expected = format_output(phoneNumbers=EXAMPLE_JSON_CONTENT["phoneNumbers"])
    run_jgrep([EXAMPLE_JSON, "phone.*s"], output=expected, returncode=0)

    # Fullmatch do not match start
    expected = ""
    run_jgrep([EXAMPLE_JSON, "stre.*"], output=expected, returncode=0)

    # Fullmatch for any start
    expected = format_output(("address::street", "123 Main St"))
    run_jgrep([EXAMPLE_JSON, ".*stre.*"], output=expected, returncode=0)


def test_single_key_extended_regex_file(setup_example_json, run_jgrep):
    # Fullmatch extended from start of string without specifying an end
    expected = format_output(
        ("phoneNumbers", EXAMPLE_JSON_CONTENT["phoneNumbers"]),
        ("phoneNumbers::0", EXAMPLE_JSON_CONTENT["phoneNumbers"][0]),
        ("phoneNumbers::0::type", EXAMPLE_JSON_CONTENT["phoneNumbers"][0]["type"]),
        ("phoneNumbers::0::number", EXAMPLE_JSON_CONTENT["phoneNumbers"][0]["number"]),
        ("phoneNumbers::1", EXAMPLE_JSON_CONTENT["phoneNumbers"][1]),
        ("phoneNumbers::1::type", EXAMPLE_JSON_CONTENT["phoneNumbers"][1]["type"]),
        ("phoneNumbers::1::number", EXAMPLE_JSON_CONTENT["phoneNumbers"][1]["number"]),
    )
    run_jgrep(
        [EXAMPLE_JSON, "-e", "phone.*"], output=expected, returncode=0, debug=True
    )

    # Fullmatch extended from start of string with specifying an end
    expected = format_output(
        ("phoneNumbers", EXAMPLE_JSON_CONTENT["phoneNumbers"]),
        ("phoneNumbers::0", EXAMPLE_JSON_CONTENT["phoneNumbers"][0]),
        ("phoneNumbers::0::type", EXAMPLE_JSON_CONTENT["phoneNumbers"][0]["type"]),
        ("phoneNumbers::0::number", EXAMPLE_JSON_CONTENT["phoneNumbers"][0]["number"]),
        ("phoneNumbers::1", EXAMPLE_JSON_CONTENT["phoneNumbers"][1]),
        ("phoneNumbers::1::type", EXAMPLE_JSON_CONTENT["phoneNumbers"][1]["type"]),
        ("phoneNumbers::1::number", EXAMPLE_JSON_CONTENT["phoneNumbers"][1]["number"]),
    )
    run_jgrep(
        [EXAMPLE_JSON, "-e", "phone.*s"], output=expected, returncode=0, debug=True
    )

    # Fullmatch extended do not match start
    expected = format_output(("address::street", "123 Main St"))
    run_jgrep([EXAMPLE_JSON, "-e", "stre.*"], output=expected, returncode=0, debug=True)

    # Fullmatch extended for any start
    expected = format_output(("address::street", "123 Main St"))
    run_jgrep(
        [EXAMPLE_JSON, "-e", ".*stre.*"], output=expected, returncode=0, debug=True
    )


def test_single_key_piped(run_jgrep):
    expected = format_output(phoneNumbers=EXAMPLE_JSON_CONTENT["phoneNumbers"])
    input_data = json.dumps(EXAMPLE_JSON_CONTENT)
    run_jgrep(["phoneNumbers"], input_data=input_data, output=expected, returncode=0)
    run_jgrep(
        ["-", "phoneNumbers"], input_data=input_data, output=expected, returncode=0
    )


def test_multiple_keys_file(setup_example_json, run_jgrep):
    expected = format_output(
        name=EXAMPLE_JSON_CONTENT["name"],
        age=EXAMPLE_JSON_CONTENT["age"],
        phoneNumbers=EXAMPLE_JSON_CONTENT["phoneNumbers"],
    )
    run_jgrep(
        [EXAMPLE_JSON, "name", "age", "--key", "phoneNumbers"],
        output=expected,
        returncode=0,
    )


def test_value_search_file(setup_example_json, run_jgrep):
    expected = format_output(("phoneNumbers::0::number", "212-555-1234"))
    run_jgrep([EXAMPLE_JSON, "--value", "212-.*"], output=expected, returncode=0)


def test_case_insensitive_key(setup_example_json, run_jgrep):
    expected = format_output(phoneNumbers=EXAMPLE_JSON_CONTENT["phoneNumbers"])
    run_jgrep([EXAMPLE_JSON, "PHONENUMBERS", "-i"], output=expected, returncode=0)


def test_indent_option(setup_example_json, run_jgrep):
    expected = format_output(
        phoneNumbers=EXAMPLE_JSON_CONTENT["phoneNumbers"], indent=2
    )
    run_jgrep(
        [EXAMPLE_JSON, "phoneNumbers", "--indent", "2"], output=expected, returncode=0
    )


def test_delimiter_nested_key(setup_example_json, run_jgrep):
    expected = format_output(
        ("address::street", EXAMPLE_JSON_CONTENT["address"]["street"])
    )
    run_jgrep([EXAMPLE_JSON, "address::street"], output=expected, returncode=0)


def test_invalid_json(run_jgrep):
    with open("invalid.json", "w") as f:
        f.write("{invalid}")
    try:
        stdout, stderr, rc = run_jgrep(["invalid.json", "name"])
        assert stdout == ""
        assert "Error: Invalid JSON" in stderr
        assert rc == 1
    finally:
        os.remove("invalid.json")


def test_no_keys_or_values(setup_example_json, run_jgrep):
    stdout, stderr, rc = run_jgrep([EXAMPLE_JSON])
    assert stdout == ""
    assert "Error: At least one key or value pattern must be specified" in stderr
    assert rc == 1
