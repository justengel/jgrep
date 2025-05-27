jgrep
=====

`jgrep` is a command-line utility for searching JSON data, allowing you to grep for keys or values using regular expressions. It supports nested key paths, case-insensitive searches, custom delimiters, and output formatting.

Features
--------

- Search JSON files or stdin for keys or values matching regex patterns.
- Support for nested key paths (e.g., ``a::b::c``).
- Case-insensitive search with ``-i`` or ``--ignore-case``.
- Customizable key path delimiter (default: ``::``).
- Pretty-printed JSON output with optional indentation (``--indent``).
- Count matches with ``-c`` or ``--count``.
- Cross-platform: works on Windows, Linux, and macOS.

Installation
------------

Python
~~~~~~

To install the Python version, navigate to the ``python/`` directory and use ``pip``:

.. code-block:: bash

    cd python
    pip install .

This installs the ``jgrep`` command. See `python/README.rst <python/README.rst>`_ for details.

Usage
-----

Both implementations have identical CLI interfaces. Basic syntax:

.. code-block:: bash

    jgrep [OPTIONS] [FILE] [KEY_PATTERNS...]

Examples
~~~~~~~~

Search for keys matching ``name`` in a JSON file:

.. code-block:: bash

    jgrep input.json name

Search for values matching ``error`` case-insensitively:

.. code-block:: bash

    jgrep -i input.json --value error

Search nested keys (e.g., ``user::address::city``):

.. code-block:: bash

    jgrep input.json user::address::city

Count matches from stdin:

.. code-block:: bash

    cat input.json | jgrep -c --value warning

For more examples, see `examples/ <examples/>`_.

License
-------

`jgrep` is licensed under the `MIT License <LICENSE>`_.

Contact
-------

For questions or feedback, open an issue on GitHub or contact Justin Engel <jtengel08@gmail.com>.