# Parser for Python's "marshal" serialization format

This is a Rust port of the [marshalparser] project, which is written in Python.

It provides both a command-line interface and a library interface for parsing
data in Python's internal "marshal" serialization format, functionality for
pretty-printing the resulting data structures, and some basic data manipulation,
for example, removing unused reference flags in order to make `pyc` files more
reproducible.

The default feature set is intentionally minimal. Dependencies that are only
required for building the command-line interface can be enabled with the `cli`
flag. Pretty-printing of byte strings can be enabled with the `fancy` feature.

This project supports parsing "marshal" data produced by CPython versions
between 3.8 and 3.13.

[marshalparser]: https://github.com/fedora-python/marshalparser
