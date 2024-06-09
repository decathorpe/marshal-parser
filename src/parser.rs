use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Cursor, Read};

use num_bigint::BigInt;

use crate::magic::{pyc_header_length, python_version_from_magic};
use crate::objects::{CodeObject, Object, ObjectType, StringType};

/// Custom error type for distinguishing different failure modes
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Invalid start of object and / or unknown type flag
    #[error("Unknown type {byte:?} at offset {offset}.")]
    UnknownType {
        #[allow(missing_docs)]
        byte: char,
        #[allow(missing_docs)]
        offset: usize,
    },
    /// Invalid file (premature end of file) or I/O error
    #[error("{inner}")]
    Io {
        #[from]
        #[allow(missing_docs)]
        inner: io::Error,
    },
    /// Unsupported Python version (unhandled object type)
    #[error("Handling for type {0:?} is not implemented.")]
    UnhandledType(ObjectType),
    /// Invalid file and / or unsupported Python version (unknown magic number)
    #[error("Cannot determine Python version from file header.")]
    UnknownVersion,
    /// Parsing error resulted in no known objects with this ID
    #[error("Missing object for reference with ID: {index}")]
    UnknownReference {
        #[allow(missing_docs)]
        index: usize,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct ReferencedObject {
    pub(crate) offset: usize,
    pub(crate) index: u32,
    pub(crate) usages: u32,
    pub(crate) typ: ObjectType,
}

/// Parsed contents of a `pyc` file or "marshal dump"
///
/// This data structure contains additional information about which objects are
/// referenced by reference objects. This data can be used to clean up unused
/// reference flags, which are, in general, not reproducible.
#[derive(Debug)]
pub struct MarshalObject {
    pub(crate) object: Object,
    pub(crate) references: HashMap<u32, Vec<usize>>,
    pub(crate) referenced: Vec<ReferencedObject>,
}

impl MarshalObject {
    /// Parse `pyc` file contents (header + marshal dump) from data
    pub fn parse_pyc(data: &[u8]) -> Result<Self, Error> {
        let mut reader = BufReader::new(Cursor::new(data));

        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;

        let Some((major, minor)) = python_version_from_magic(&buf) else {
            return Err(Error::UnknownVersion);
        };

        let header_length = pyc_header_length((major, minor));
        reader.seek_relative((header_length - 4) as i64)?;

        let parser = Parser::new((major, minor), header_length);
        let (object, references, referenced) = parser.read_marshal(&mut reader)?;

        Ok(MarshalObject {
            object,
            references,
            referenced,
        })
    }

    /// Parse marshal dump contents from data
    ///
    /// Since plain "marshal dumps" do not contain a `pyc` file header, the
    /// version of Python that was used to create the data must be specified.
    pub fn parse_dump(data: &[u8], (major, minor): (u16, u16)) -> Result<Self, Error> {
        let mut reader = BufReader::new(Cursor::new(data));
        let parser = Parser::new((major, minor), 0);
        let (object, references, referenced) = parser.read_marshal(&mut reader)?;

        Ok(MarshalObject {
            object,
            references,
            referenced,
        })
    }

    /// Clear unused reference flags from objects
    ///
    /// This method can be used to make `pyc` files more reproducible.
    ///
    /// Reference flags are removed from objects that are never referenced, and
    /// remaining references are adjusted for the shuffled index numbers.
    ///
    /// If no changes are made, data is returned without modifications in a
    /// [`Cow::Borrowed`], otherwise a [`Cow::Owned`] with new file contents is
    /// returned.
    pub fn clear_unused_ref_flags(self, data: &[u8]) -> Cow<[u8]> {
        // this method consumes self because it invalidates the unmarshaled object

        let unreferenced: Vec<_> = self.referenced.iter().filter(|x| x.usages == 0).collect();

        if unreferenced.is_empty() {
            log::info!("No unused references found.");
            return Cow::Borrowed(data);
        }
        let mut data = data.to_vec();

        let mut dropped_indices = Vec::new();
        for unref in &unreferenced {
            log::info!(
                "Clearing unused reference bit from object at offset {} with index {}",
                unref.offset,
                unref.index
            );
            data[unref.offset] = clear_bit(data[unref.offset], 7);
            dropped_indices.push(unref.index);
        }

        for (index, offsets) in &self.references {
            let diff = dropped_indices.iter().filter(|x| **x < *index).count() as u32;

            for offset in offsets {
                let old_bytes = [data[*offset], data[*offset + 1], data[*offset + 2], data[*offset + 3]];
                let old = u32::from_le_bytes(old_bytes);
                let new = old - diff;

                log::info!(
                    "Rewriting reference object at offset {}: index {} -> {}",
                    offset,
                    old,
                    new
                );
                let new_bytes = new.to_le_bytes();

                data[*offset] = new_bytes[0];
                data[*offset + 1] = new_bytes[1];
                data[*offset + 2] = new_bytes[2];
                data[*offset + 3] = new_bytes[3];
            }
        }

        log::info!("Removed {} unused references.", unreferenced.len());
        Cow::Owned(data)
    }

    /// Print objects with unused reference flags to stdout
    pub fn print_unused_ref_flags(&self) {
        for r in &self.referenced {
            if r.usages == 0 {
                println!(
                    "Unused reference bit: {} object with reference index {} at offset {}",
                    r.typ, r.index, r.offset
                );
            }
        }
    }

    /// Obtain a reference to the inner [`Object`]
    pub fn inner(&self) -> &Object {
        &self.object
    }

    /// Consume this [`MarshalObject`] to obtain the inner [`Object`]
    pub fn into_inner(self) -> Object {
        self.object
    }
}

type References = HashMap<u32, Vec<usize>>;
type Referenced = Vec<ReferencedObject>;

#[derive(Debug)]
pub(crate) struct Parser {
    version: (u16, u16),
    offset: usize,
    references: References,
    referenced: Referenced,
}

impl Parser {
    fn new(version: (u16, u16), offset: usize) -> Self {
        Parser {
            version,
            offset,
            references: HashMap::new(),
            referenced: Vec::new(),
        }
    }

    fn read_marshal<T: BufRead>(mut self, reader: &mut T) -> Result<(Object, References, Referenced), Error> {
        let object = self.read_object(reader)?;

        for (index, usages) in &self.references {
            let index = *index as usize;

            if let Some(r) = self.referenced.get_mut(index) {
                r.usages = usages.len() as u32;
            } else {
                return Err(Error::UnknownReference { index });
            }
        }

        Ok((object, self.references, self.referenced))
    }

    fn read_object<T: BufRead>(&mut self, bytes: &mut T) -> Result<Object, Error> {
        log::debug!("Reading object at offset {}", self.offset);

        let offset = self.offset;
        let mut byte = self.read_u8(bytes)?;

        let mut ref_id = None;

        // check if this object has the reference flag bit set
        if test_bit(byte, 7) {
            let index = self.referenced.len() as u32;
            log::debug!("Object at offset {} assigned reference index {}", self.offset, index);

            byte = clear_bit(byte, 7);
            ref_id = Some(index);
        }

        let Some(typ) = ObjectType::try_from(byte).ok() else {
            return Err(Error::UnknownType {
                byte: byte.into(),
                offset,
            });
        };

        if let Some(index) = ref_id {
            let obj = ReferencedObject {
                offset,
                index,
                usages: 0,
                typ,
            };

            self.referenced.push(obj);
        }

        let result = match typ {
            // singleton objects
            ObjectType::Null => Object::Null,
            ObjectType::None => Object::None,
            ObjectType::False => Object::False,
            ObjectType::True => Object::True,
            ObjectType::StopIteration => Object::StopIteration,
            ObjectType::Ellipsis => Object::Ellipsis,

            // simple objects
            ObjectType::Int => Object::Int(self.read_u32(bytes)?),
            ObjectType::BinaryFloat => Object::BinaryFloat(self.read_f64(bytes)?),
            ObjectType::BinaryComplex => Object::BinaryComplex((self.read_f64(bytes)?, self.read_f64(bytes)?)),

            // string objects
            ObjectType::String => Object::String {
                typ: StringType::String,
                bytes: self.read_string(bytes, false)?,
            },
            ObjectType::Interned => Object::String {
                typ: StringType::Interned,
                bytes: self.read_string(bytes, false)?,
            },
            ObjectType::Unicode => Object::String {
                typ: StringType::Unicode,
                bytes: self.read_string(bytes, false)?,
            },
            ObjectType::Ascii => Object::String {
                typ: StringType::Ascii,
                bytes: self.read_string(bytes, false)?,
            },
            ObjectType::AsciiInterned => Object::String {
                typ: StringType::AsciiInterned,
                bytes: self.read_string(bytes, false)?,
            },
            ObjectType::ShortAscii => Object::String {
                typ: StringType::Ascii,
                bytes: self.read_string(bytes, true)?,
            },
            ObjectType::ShortAsciiInterned => Object::String {
                typ: StringType::AsciiInterned,
                bytes: self.read_string(bytes, true)?,
            },

            // collection objects
            ObjectType::Tuple => Object::Tuple(self.read_collection(bytes, false)?),
            ObjectType::List => Object::List(self.read_collection(bytes, false)?),
            ObjectType::Set => Object::Set(self.read_collection(bytes, false)?),
            ObjectType::FrozenSet => Object::FrozenSet(self.read_collection(bytes, false)?),
            ObjectType::SmallTuple => Object::Tuple(self.read_collection(bytes, true)?),
            ObjectType::Dict => Object::Dict(self.read_dict(bytes)?),

            // special cases
            ObjectType::Long => Object::Long(self.read_long(bytes)?),
            ObjectType::Ref => Object::Ref(self.read_ref(bytes)?),
            ObjectType::Code => Object::Code(Box::new(self.read_code_object(bytes)?)),

            // unhandled types:
            // ObjectType::{Int64,Float,Complex,Unknown}
            x => return Err(Error::UnhandledType(x)),
        };

        Ok(result)
    }

    fn read_bytes<T: BufRead>(&mut self, bytes: &mut T, n: usize) -> Result<Vec<u8>, Error> {
        let mut buf = vec![0u8; n];
        bytes.read_exact(&mut buf)?;
        self.offset += n;
        Ok(buf)
    }

    fn read_bytes_const<T: BufRead, const N: usize>(&mut self, bytes: &mut T) -> Result<[u8; N], Error> {
        let mut buf = [0u8; N];
        bytes.read_exact(&mut buf)?;
        self.offset += N;
        Ok(buf)
    }

    fn read_u8<T: BufRead>(&mut self, bytes: &mut T) -> Result<u8, Error> {
        log::debug!("Reading u8 at offset {}", self.offset);
        Ok(u8::from_le_bytes(self.read_bytes_const(bytes)?))
    }

    fn read_u32<T: BufRead>(&mut self, bytes: &mut T) -> Result<u32, Error> {
        log::debug!("Reading u32 at offset {}", self.offset);
        Ok(u32::from_le_bytes(self.read_bytes_const(bytes)?))
    }

    fn read_i32<T: BufRead>(&mut self, bytes: &mut T) -> Result<i32, Error> {
        log::debug!("Reading i32 at offset {}", self.offset);
        Ok(i32::from_le_bytes(self.read_bytes_const(bytes)?))
    }

    fn read_f64<T: BufRead>(&mut self, bytes: &mut T) -> Result<f64, Error> {
        log::debug!("Reading f64 at offset {}", self.offset);
        Ok(f64::from_le_bytes(self.read_bytes_const(bytes)?))
    }

    fn read_string<T: BufRead>(&mut self, bytes: &mut T, short: bool) -> Result<Vec<u8>, Error> {
        let size = if short {
            log::debug!("Reading short string at offset {}", self.offset);
            self.read_u8(bytes)? as usize
        } else {
            log::debug!("Reading string at offset {}", self.offset);
            self.read_u32(bytes)? as usize
        };

        let bytes = self.read_bytes(bytes, size)?;
        Ok(bytes)
    }

    fn read_collection<T: BufRead>(&mut self, bytes: &mut T, small: bool) -> Result<Vec<Object>, Error> {
        let size = if small {
            log::debug!("Reading small tuple at offset {}", self.offset);
            self.read_u8(bytes)? as usize
        } else {
            log::debug!("Reading collection at offset {}", self.offset);
            self.read_u32(bytes)? as usize
        };

        let mut result = Vec::with_capacity(size);
        for _ in 0..size {
            result.push(self.read_object(bytes)?);
        }

        Ok(result)
    }

    fn read_dict<T: BufRead>(&mut self, bytes: &mut T) -> Result<Vec<(Object, Object)>, Error> {
        log::debug!("Reading collection at offset {}", self.offset);

        let mut result = Vec::new();

        loop {
            let key = self.read_object(bytes)?;
            if key == Object::Null {
                break;
            }

            let value = self.read_object(bytes)?;
            result.push((key, value));
        }

        Ok(result)
    }

    fn read_long<T: BufRead>(&mut self, bytes: &mut T) -> Result<BigInt, Error> {
        log::debug!("Reading long at offset {}", self.offset);

        let size = self.read_i32(bytes)?;

        let mut result = BigInt::ZERO;
        let mut shift = 0;

        for _ in 0..size.abs() {
            let x = {
                let b = self.read_bytes_const::<T, 2>(bytes)?;

                let mut x = b[0] as i16;
                x |= (b[1] as i16) << 8;
                x |= -(x & 0x8000u16 as i16);

                BigInt::from(x)
            };

            result += x << shift;
            shift += 15;
        }

        if size > 0 {
            Ok(result)
        } else {
            Ok(-result)
        }
    }

    fn read_ref<T: BufRead>(&mut self, bytes: &mut T) -> Result<u32, Error> {
        log::debug!("Reading reference at offset {}", self.offset);

        let offset = self.offset;
        let index = self.read_u32(bytes)?;
        log::debug!("Found reference at offset {} with index {}", offset, index);

        self.references
            .entry(index)
            .and_modify(|x| x.push(offset))
            .or_insert(vec![offset]);
        Ok(index)
    }

    fn read_code_object<T: BufRead>(&mut self, bytes: &mut T) -> Result<CodeObject, Error> {
        log::debug!("Reading codeobject at offset {}", self.offset);

        let result = CodeObject {
            argcount: self.read_u32(bytes)?,
            posonlyargcount: if self.version >= (3, 8) {
                Some(self.read_u32(bytes)?)
            } else {
                None
            },
            kwonlyargcount: self.read_u32(bytes)?,
            nlocals: if self.version < (3, 11) {
                Some(self.read_u32(bytes)?)
            } else {
                None
            },
            stacksize: self.read_u32(bytes)?,
            flags: self.read_u32(bytes)?,
            code: self.read_object(bytes)?,
            consts: self.read_object(bytes)?,
            names: self.read_object(bytes)?,
            varnames: if self.version < (3, 11) {
                Some(self.read_object(bytes)?)
            } else {
                None
            },
            freevars: if self.version < (3, 11) {
                Some(self.read_object(bytes)?)
            } else {
                None
            },
            cellvars: if self.version < (3, 11) {
                Some(self.read_object(bytes)?)
            } else {
                None
            },
            localsplusnames: if self.version >= (3, 11) {
                Some(self.read_object(bytes)?)
            } else {
                None
            },
            localspluskinds: if self.version >= (3, 11) {
                Some(self.read_object(bytes)?)
            } else {
                None
            },
            filename: self.read_object(bytes)?,
            name: self.read_object(bytes)?,
            qualname: if self.version >= (3, 11) {
                Some(self.read_object(bytes)?)
            } else {
                None
            },
            firstlineno: self.read_u32(bytes)?,
            linetable: self.read_object(bytes)?,
            exceptiontable: if self.version >= (3, 11) {
                Some(self.read_object(bytes)?)
            } else {
                None
            },
        };

        Ok(result)
    }
}

fn test_bit(b: u8, i: u8) -> bool {
    b & (1 << i) != 0u8
}

fn clear_bit(b: u8, i: u8) -> u8 {
    b & !(1 << i)
}
