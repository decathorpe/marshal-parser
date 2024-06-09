//! # Parser for the "marshal" binary de/serialization format used by CPython
//!
//! This crate implements a parser and some utilities for reading files in the
//! "marshal" de/serialization format used internally in CPython. The exact
//! format is not stable and can change between minor versions of CPython.
//!
//! This crate supports parsing "marshal" dumps and `pyc` files that were
//! written by CPython versions `>= 3.6` and `< 3.14`.
//!
//! There is a high-level and a low-level API, depending on how much access to
//! the underlying data structures is needed. The low-level API also provides
//! more flexibility since it does not require files, but can operate on plain
//! bytes ([`Vec<u8>`]).
//!
//! Reading a `pyc` file from disk:
//!
//! ```no_run
//! use marshal_parser::{MarshalFile, Object};
//!
//! let pyc = MarshalFile::from_pyc_path("mod.cpython-310.pyc").unwrap();
//! let object: Object = pyc.into_inner();
//! ```
//!
//! Reading a "marshal" dump (i.e. a file without `pyc` header):
//!
//! ```no_run
//! use marshal_parser::{MarshalFile, Object};
//!
//! let dump = MarshalFile::from_dump_path("dump.marshal", (3, 11)).unwrap();
//! let object: Object = dump.into_inner();
//! ```

use std::borrow::Cow;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

mod magic;
mod objects;
mod parser;

pub use objects::{CodeObject, Object, ObjectType, StringType};
pub use parser::{Error, MarshalObject};

type Result<T> = std::result::Result<T, Error>;

/// High-level parser for `pyc` and "marshal dump" files
#[derive(Debug)]
pub struct MarshalFile {
    data: Vec<u8>,
    marshal: MarshalObject,
}

impl MarshalFile {
    /// Read and parse a `pyc` file at the specified path
    pub fn from_pyc_path<S>(path: S) -> Result<Self>
    where
        S: AsRef<Path>,
    {
        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .create_new(false)
            .open(path)?;
        let mut reader = BufReader::new(file);

        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        let marshal = MarshalObject::parse_pyc(&data)?;
        Ok(MarshalFile { data, marshal })
    }

    /// Read and parse a "marshal dump" file at the specified path
    pub fn from_dump_path<S>(path: S, (major, minor): (u16, u16)) -> Result<Self>
    where
        S: AsRef<Path>,
    {
        let file = OpenOptions::new()
            .read(true)
            .write(false)
            .create_new(false)
            .open(path)?;
        let mut reader = BufReader::new(file);

        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        let marshal = MarshalObject::parse_dump(&data, (major, minor))?;
        Ok(MarshalFile { data, marshal })
    }

    /// Obtain a reference to the inner [`Object`]
    pub fn inner(&self) -> &Object {
        &self.marshal.object
    }

    /// Consume this [`MarshalFile`] to obtain the inner [`Object`]
    pub fn into_inner(self) -> Object {
        self.marshal.object
    }

    /// Print objects with unused reference flags to stdout
    pub fn print_unused_ref_flags(&self) {
        self.marshal.print_unused_ref_flags();
    }

    /// Rewrite file to remove unused reference flags
    ///
    /// This can be useful to generate `pyc` files that are reproducible across
    /// different CPU architectures.
    ///
    /// If no unused reference flags are found, no file is written, and `false`
    /// is returned. If a file is written, `true` is returned.
    pub fn write_normalized<S>(self, path: S) -> Result<bool>
    where
        S: AsRef<Path>,
    {
        let marshal = self.marshal;
        let result = marshal.clear_unused_ref_flags(&self.data);

        if let Cow::Owned(x) = result {
            let file = File::create_new(path)?;
            let mut writer = BufWriter::new(file);

            writer.write_all(&x)?;

            Ok(true)
        } else {
            Ok(false)
        }
    }
}
