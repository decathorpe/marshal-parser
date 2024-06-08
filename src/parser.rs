use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, Read, Seek, SeekFrom, Write};

use num_bigint::BigInt;

use crate::magic::{pyc_header_length, python_version_from_magic};
use crate::objects::{CodeObject, Object, ObjectType};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ParserError {
    #[error("Unknown type {byte:?} at offset {offset}.")]
    UnknownType { byte: char, offset: usize },
    #[error("{inner}")]
    Io {
        #[from]
        inner: io::Error,
    },
    #[error("Handling for type {0:?} is not implemented.")]
    UnhandledType(ObjectType),
    #[error("Cannot determine Python version from file header.")]
    UnknownVersion,
    #[error("Found two references with the same ID: {index}")]
    DuplicateReference { index: u32 },
    #[error("Missing object for reference with ID: {index}")]
    UnknownReference { index: usize },
}

#[derive(Clone, Debug)]
pub(crate) struct ReferencedObject {
    pub(crate) offset: usize,
    pub(crate) index: u32,
    pub(crate) usages: u32,
    pub(crate) typ: ObjectType,
}

pub(crate) struct MarshalObject {
    pub(crate) object: Object,
    pub(crate) references: HashMap<u32, Vec<usize>>,
    pub(crate) referenced: Vec<ReferencedObject>,
}

impl MarshalObject {
    pub(crate) fn parse_dump<R>(reader: &mut R, (major, minor): (u16, u16)) -> Result<Self, ParserError>
    where
        R: BufRead,
    {
        let parser = Parser::new((major, minor), 0);
        parser.read_marshal(reader)
    }

    pub(crate) fn parse_pyc<R>(reader: &mut R) -> Result<Self, ParserError>
    where
        R: BufRead + io::Seek,
    {
        let mut buf = [0u8; 4];
        reader.read_exact(&mut buf)?;

        let Some((major, minor)) = python_version_from_magic(&buf) else {
            return Err(ParserError::UnknownVersion);
        };

        let header_length = pyc_header_length((major, minor));
        reader.seek_relative((header_length - 4) as i64)?;

        let parser = Parser::new((major, minor), header_length);
        parser.read_marshal(reader)
    }

    // consume self because it invalidates the unmarshaled object
    pub(crate) fn clear_unused_ref_flags(self, file: &mut File) -> Result<(), ParserError> {
        let unreferenced: Vec<_> = self.referenced.iter().filter(|x| x.usages == 0).collect();

        if unreferenced.is_empty() {
            return Ok(());
        }

        let mut dropped_indices = Vec::new();

        for unref in unreferenced {
            file.seek(SeekFrom::Start(unref.offset as u64))?;

            let mut buf = [0u8; 1];
            file.read_exact(&mut buf)?;

            log::info!(
                "Clearing unused reference bit from object at offset {} with index {}",
                unref.offset,
                unref.index
            );
            buf[0] = clear_bit(buf[0], 7);

            file.seek(SeekFrom::Start(unref.offset as u64))?;
            file.write_all(&buf)?;

            dropped_indices.push(unref.index);
        }

        for (index, offsets) in &self.references {
            let diff = dropped_indices.iter().filter(|x| **x < *index).count() as u32;

            for offset in offsets {
                file.seek(SeekFrom::Start(*offset as u64))?;

                let mut buf = [0u8; 4];
                file.read_exact(&mut buf)?;

                let old = u32::from_le_bytes(buf);
                let new = old - diff;

                log::info!(
                    "Rewriting reference object at offset {}: index {} -> {}",
                    offset,
                    old,
                    new
                );
                buf = new.to_le_bytes();

                file.seek(SeekFrom::Start(*offset as u64))?;
                file.write_all(&buf)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct Parser {
    version: (u16, u16),
    offset: usize,
    indent: u32,

    references: HashMap<u32, Vec<usize>>,
    referenced: Vec<Option<ReferencedObject>>,
}

impl Parser {
    fn new(version: (u16, u16), offset: usize) -> Self {
        Parser {
            version,
            offset,
            indent: 0,

            references: HashMap::new(),
            referenced: Vec::new(),
        }
    }

    fn read_marshal<T: BufRead>(mut self, reader: &mut T) -> Result<MarshalObject, ParserError> {
        let object = self.read_object(reader)?;

        for (index, reference) in self.referenced.iter().enumerate() {
            if reference.is_none() {
                return Err(ParserError::UnknownReference { index });
            }
        }

        let mut referenced: Vec<ReferencedObject> = self.referenced.clone().into_iter().flatten().collect();

        for (index, usages) in &self.references {
            referenced[*index as usize].usages = usages.len() as u32;
        }

        Ok(MarshalObject {
            object,
            referenced,
            references: self.references,
        })
    }

    fn read_object<T: BufRead>(&mut self, bytes: &mut T) -> Result<Object, ParserError> {
        log::debug!("Reading object at offset {}", self.offset);

        let offset = self.offset;
        let mut byte = self.read_u8(bytes)?;

        let mut ref_id = None;

        // check if this object has the reference flag bit set
        if test_bit(byte, 7) {
            let index = self.referenced.len() as u32;
            self.referenced.push(None);

            log::debug!("Object at offset {} assigned reference index {}", self.offset, index);

            byte = clear_bit(byte, 7);
            ref_id = Some(index);
        }

        let Some(typ) = ObjectType::try_from(byte).ok() else {
            return Err(ParserError::UnknownType {
                byte: byte.into(),
                offset,
            });
        };

        self.indent += 2;

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
            ObjectType::String
            | ObjectType::Interned
            | ObjectType::Unicode
            | ObjectType::Ascii
            | ObjectType::AsciiInterned => Object::String(self.read_string(bytes, false)?),
            ObjectType::ShortAscii | ObjectType::ShortAsciiInterned => Object::String(self.read_string(bytes, true)?),

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
            x => return Err(ParserError::UnhandledType(x)),
        };

        self.indent -= 2;

        if let Some(index) = ref_id {
            log::debug!("Finalizing referenced object at offset {} with id {}", offset, index);

            let obj = ReferencedObject {
                offset,
                index,
                usages: 0,
                typ,
            };

            if self.referenced[index as usize].replace(obj).is_some() {
                return Err(ParserError::DuplicateReference { index });
            }
        }

        Ok(result)
    }

    fn read_bytes<T: BufRead>(&mut self, bytes: &mut T, n: usize) -> Result<Vec<u8>, ParserError> {
        let mut buf = vec![0u8; n];
        bytes.read_exact(&mut buf)?;
        self.offset += n;
        Ok(buf)
    }

    fn read_bytes_const<T: BufRead, const N: usize>(&mut self, bytes: &mut T) -> Result<[u8; N], ParserError> {
        let mut buf = [0u8; N];
        bytes.read_exact(&mut buf)?;
        self.offset += N;
        Ok(buf)
    }

    fn read_u8<T: BufRead>(&mut self, bytes: &mut T) -> Result<u8, ParserError> {
        log::debug!("Reading u8 at offset {}", self.offset);
        Ok(u8::from_le_bytes(self.read_bytes_const(bytes)?))
    }

    fn read_u32<T: BufRead>(&mut self, bytes: &mut T) -> Result<u32, ParserError> {
        log::debug!("Reading u32 at offset {}", self.offset);
        Ok(u32::from_le_bytes(self.read_bytes_const(bytes)?))
    }

    fn read_i32<T: BufRead>(&mut self, bytes: &mut T) -> Result<i32, ParserError> {
        log::debug!("Reading i32 at offset {}", self.offset);
        Ok(i32::from_le_bytes(self.read_bytes_const(bytes)?))
    }

    fn read_f64<T: BufRead>(&mut self, bytes: &mut T) -> Result<f64, ParserError> {
        log::debug!("Reading f64 at offset {}", self.offset);
        Ok(f64::from_le_bytes(self.read_bytes_const(bytes)?))
    }

    fn read_string<T: BufRead>(&mut self, bytes: &mut T, short: bool) -> Result<Vec<u8>, ParserError> {
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

    fn read_collection<T: BufRead>(&mut self, bytes: &mut T, small: bool) -> Result<Vec<Object>, ParserError> {
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

    fn read_dict<T: BufRead>(&mut self, bytes: &mut T) -> Result<Vec<(Object, Object)>, ParserError> {
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

    fn read_long<T: BufRead>(&mut self, bytes: &mut T) -> Result<BigInt, ParserError> {
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

    fn read_ref<T: BufRead>(&mut self, bytes: &mut T) -> Result<u32, ParserError> {
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

    fn read_code_object<T: BufRead>(&mut self, bytes: &mut T) -> Result<CodeObject, ParserError> {
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
