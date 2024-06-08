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
//! more flexibility since it does not require files, but can operate on any
//! input that implements [`BufRead`] and [`Seek`].
//!
//! ## High-level API
//!
//! Reading a `pyc` file from disk:
//!
//! ```no_run
//! use marshal_parser::{Object, PycFile};
//!
//! let pyc = PycFile::from_path("mod.cpython-310.pyc").unwrap();
//! let object: Object = pyc.into_inner();
//! ```
//!
//! Reading a "marshal" dump (i.e. a file without `pyc` header):
//!
//! ```no_run
//! use marshal_parser::{DumpFile, Object};
//!
//! let dump = DumpFile::from_path("dump.marshal", (3, 11)).unwrap();
//! let object: Object = dump.into_inner();
//! ```
//!
//! ## Low-level API
//!
//! Parsing `pyc` format with a custom reader:
//!
//! ```no_run
//! use marshal_parser::MarshalObject;
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! let file = File::open("mod.cpython-311.pyc").unwrap();
//! let mut reader = BufReader::new(file);
//! let marshal = MarshalObject::parse_pyc(&mut reader).unwrap();
//! ```
//!
//! Parsing a "marshal dump" with a custom reader:
//!
//! ```no_run
//! use marshal_parser::MarshalObject;
//! use std::fs::File;
//! use std::io::BufReader;
//!
//! let file = File::open("dump.marshal").unwrap();
//! let mut reader = BufReader::new(file);
//! let marshal = MarshalObject::parse_dump(&mut reader, (3, 11)).unwrap();
//! ```

#[cfg(doc)]
use std::io::{BufRead, Seek};

use std::fs::{File, OpenOptions};
use std::io::BufReader;
use std::path::Path;

mod magic;
mod objects;
mod parser;

pub use objects::{CodeObject, Object, ObjectType};
pub use parser::{Error, MarshalObject};

type Result<T> = std::result::Result<T, Error>;

/// High-level parser for `pyc` files
#[derive(Debug)]
pub struct PycFile {
    reader: BufReader<File>,
    marshal: MarshalObject,
}

impl PycFile {
    /// Read and parse a file at the specified path
    pub fn from_path<S>(path: S) -> Result<Self>
    where
        S: AsRef<Path>,
    {
        let file = OpenOptions::new().read(true).write(true).create_new(false).open(path)?;
        let mut reader = BufReader::new(file);
        let marshal = MarshalObject::parse_pyc(&mut reader)?;
        Ok(PycFile { reader, marshal })
    }

    /// Obtain a reference to the inner [`Object`]
    pub fn inner(&self) -> &Object {
        &self.marshal.object
    }

    /// Consume this [`PycFile`] to obtain the inner [`Object`]
    pub fn into_inner(self) -> Object {
        self.marshal.object
    }

    /// Rewrite file to remove unused reference flags
    ///
    /// This method calls [`MarshalObject::clear_unused_ref_flags`] internally
    /// and rewrites the contents of the opened file.
    ///
    /// The low-level API provides more options if overwriting existing file
    /// contents is not desired.
    pub fn normalize(self) -> Result<()> {
        let mut file = self.reader.into_inner();
        self.marshal.clear_unused_ref_flags(&mut file)?;
        Ok(())
    }
}

/// High-level parser for files containing a "marshal dump"
#[derive(Debug)]
pub struct DumpFile {
    reader: BufReader<File>,
    marshal: MarshalObject,
}

impl DumpFile {
    /// Read and parse a file at the specified path
    pub fn from_path<S>(path: S, (major, minor): (u16, u16)) -> Result<Self>
    where
        S: AsRef<Path>,
    {
        let file = OpenOptions::new().read(true).write(true).create_new(false).open(path)?;
        let mut reader = BufReader::new(file);
        let marshal = MarshalObject::parse_dump(&mut reader, (major, minor))?;
        Ok(DumpFile { reader, marshal })
    }

    /// Obtain a reference to the inner [`Object`]
    pub fn inner(&self) -> &Object {
        &self.marshal.object
    }

    /// Consume this [`DumpFile`] to obtain the inner [`Object`]
    pub fn into_inner(self) -> Object {
        self.marshal.object
    }

    /// Rewrite file to remove unused reference flags
    ///
    /// This method calls [`MarshalObject::clear_unused_ref_flags`] internally
    /// and rewrites the contents of the opened file.
    ///
    /// The low-level API provides more options if overwriting existing file
    /// contents is not desired.
    pub fn normalize(self) -> Result<()> {
        let mut file = self.reader.into_inner();
        self.marshal.clear_unused_ref_flags(&mut file)?;
        Ok(())
    }
}
