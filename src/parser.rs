use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read};

use num_bigint::BigInt;

use crate::magic::{pyc_header_length, python_version_from_magic};
use crate::objects::{Object, ObjectType};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ParserError {
    #[error("Unknown type {byte:?} at offset {offset}")]
    UnknownType { byte: char, offset: usize },
    #[error("IOError: {inner}")]
    Io {
        #[from]
        inner: io::Error,
    },
    #[error("Parsing error: Unhandled type: {0}")]
    UnhandledType(ObjectType),
    #[error("Parsing error: Unknown reference")]
    UnknownReference,
    #[error("Parsing error: cannot parse codeobjects for unknown Python versions")]
    UnknownVersion,
}

pub(crate) fn parse_file(path: &str) -> Result<(ParserState, File), ParserError> {
    let file = File::open(path)?;

    // reading and seeking from a File directly is inefficient
    //let mut reader = BufReader::new(file);
    let mut reader = BufReader::new(file);

    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;

    let version = python_version_from_magic(&buf);

    let offset = if let Some((major, minor)) = version {
        let header_length = pyc_header_length((major, minor));
        reader.seek_relative((header_length - 4) as i64)?;
        header_length
    } else {
        eprintln!("No pyc file header found, assuming marshal dump.");
        reader.seek_relative(-4)?;
        0
    };

    let mut state = ParserState::new(version, offset);

    state.read_object(&mut reader)?;

    let file = reader.into_inner();

    Ok((state, file))
}

#[derive(Debug)]
struct FlagRef {
    offset: usize,
    typ: ObjectType,
    content: Object,
    usages: u32,
}

#[derive(Debug)]
pub(crate) struct ParserState {
    version: Option<(u16, u16)>,
    offset: usize,
    indent: u32,
    references: Vec<(usize, u32)>,
    flag_refs: HashMap<usize, FlagRef>,
}

impl ParserState {
    fn new(version: Option<(u16, u16)>, offset: usize) -> Self {
        ParserState {
            version,
            offset,
            indent: 0,
            references: Vec::new(),
            flag_refs: HashMap::new(),
        }
    }

    fn read_object(&mut self, bytes: &mut BufReader<File>) -> Result<Object, ParserError> {
        let offset = self.offset;
        let mut byte = self.read_u8(bytes)?;

        let mut ref_id = None;

        if test_bit(byte, 7) {
            byte = clear_bit(byte, 7);
            ref_id = Some(self.flag_refs.len());
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
            ObjectType::Code => Object::Code(self.read_code_object(bytes)?),

            // unhandled types:
            // ObjectType::{Int64,Float,Complex,Unknown}
            x => return Err(ParserError::UnhandledType(x)),
        };

        self.indent -= 2;

        if let Some(id) = ref_id {
            self.flag_refs.insert(
                id,
                FlagRef {
                    offset,
                    typ,
                    content: result.clone(),
                    usages: 0,
                },
            );
        }

        Ok(result)
    }

    fn read_bytes(&mut self, bytes: &mut BufReader<File>, n: usize) -> Result<Vec<u8>, ParserError> {
        let mut buf = vec![0u8; n];
        bytes.read_exact(&mut buf)?;
        self.offset += n;
        Ok(buf)
    }

    fn read_bytes_const<const N: usize>(&mut self, bytes: &mut BufReader<File>) -> Result<[u8; N], ParserError> {
        let mut buf = [0u8; N];
        bytes.read_exact(&mut buf)?;
        self.offset += N;
        Ok(buf)
    }

    fn read_u8(&mut self, bytes: &mut BufReader<File>) -> Result<u8, ParserError> {
        Ok(u8::from_le_bytes(self.read_bytes_const(bytes)?))
    }

    fn read_u32(&mut self, bytes: &mut BufReader<File>) -> Result<u32, ParserError> {
        Ok(u32::from_le_bytes(self.read_bytes_const(bytes)?))
    }

    fn read_i32(&mut self, bytes: &mut BufReader<File>) -> Result<i32, ParserError> {
        Ok(i32::from_le_bytes(self.read_bytes_const(bytes)?))
    }

    fn read_f64(&mut self, bytes: &mut BufReader<File>) -> Result<f64, ParserError> {
        Ok(f64::from_le_bytes(self.read_bytes_const(bytes)?))
    }

    fn read_string(&mut self, bytes: &mut BufReader<File>, short: bool) -> Result<Vec<u8>, ParserError> {
        let size = if short {
            self.read_u8(bytes)? as usize
        } else {
            self.read_u32(bytes)? as usize
        };

        let bytes = self.read_bytes(bytes, size)?;
        Ok(bytes)
    }

    fn read_collection(&mut self, bytes: &mut BufReader<File>, small: bool) -> Result<Vec<Object>, ParserError> {
        let size = if small {
            self.read_u8(bytes)? as usize
        } else {
            self.read_u32(bytes)? as usize
        };

        let mut result = Vec::with_capacity(size);
        for _ in 0..size {
            result.push(self.read_object(bytes)?);
        }

        Ok(result)
    }

    fn read_dict(&mut self, bytes: &mut BufReader<File>) -> Result<Vec<(Object, Object)>, ParserError> {
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

    fn read_long(&mut self, bytes: &mut BufReader<File>) -> Result<BigInt, ParserError> {
        let size = self.read_i32(bytes)?;

        let mut result = BigInt::ZERO;
        let mut shift = 0;

        for _ in 0..size.abs() {
            let x = {
                let b = self.read_bytes_const::<2>(bytes)?;

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

    fn read_ref(&mut self, bytes: &mut BufReader<File>) -> Result<(usize, u32), ParserError> {
        let offset = self.offset;

        let index = self.read_u32(bytes)?;
        let result = (offset, index);
        self.references.push(result);

        if let Some(ref mut flag_ref) = self.flag_refs.get_mut(&(index as usize)) {
            flag_ref.usages += 1;
        } else {
            return Err(ParserError::UnknownReference);
        }

        Ok(result)
    }

    fn read_code_object(&mut self, bytes: &mut BufReader<File>) -> Result<Vec<(&'static str, Object)>, ParserError> {
        let Some(version) = self.version else {
            return Err(ParserError::UnknownVersion);
        };

        let mut result = Vec::new();

        let argcount = self.read_u32(bytes)?;
        result.push(("argcount", Object::Int(argcount)));

        if version >= (3, 8) {
            let posonlyargcount = self.read_u32(bytes)?;
            result.push(("posonlyargcount", Object::Int(posonlyargcount)));
        }

        let kwonlyargcount = self.read_u32(bytes)?;
        result.push(("kwonlyargcount", Object::Int(kwonlyargcount)));

        if version < (3, 11) {
            let nlocals = self.read_u32(bytes)?;
            result.push(("nlocals", Object::Int(nlocals)));
        }

        let stacksize = self.read_u32(bytes)?;
        result.push(("stacksize", Object::Int(stacksize)));

        let flags = self.read_u32(bytes)?;
        result.push(("flags", Object::Int(flags)));

        let code = self.read_object(bytes)?;
        result.push(("code", code));

        let consts = self.read_object(bytes)?;
        result.push(("consts", consts));

        let names = self.read_object(bytes)?;
        result.push(("names", names));

        if version < (3, 11) {
            let varnames = self.read_object(bytes)?;
            result.push(("varnames", varnames));

            let freevars = self.read_object(bytes)?;
            result.push(("freevars", freevars));

            let cellvars = self.read_object(bytes)?;
            result.push(("cellvars", cellvars));
        }

        if version >= (3, 11) {
            let localsplusnames = self.read_object(bytes)?;
            result.push(("localsplusnames", localsplusnames));

            let localspluskinds = self.read_object(bytes)?;
            result.push(("localspluskinds", localspluskinds));
        }

        let filename = self.read_object(bytes)?;
        result.push(("filename", filename));

        let name = self.read_object(bytes)?;
        result.push(("name", name));

        if version >= (3, 11) {
            let qualname = self.read_object(bytes)?;
            result.push(("qualname", qualname));
        }

        let firstlineno = self.read_object(bytes)?;
        result.push(("firstlineno", firstlineno));

        let linetable = self.read_object(bytes)?;
        result.push(("linetable", linetable));

        if version >= (3, 11) {
            let exceptiontable = self.read_object(bytes)?;
            result.push(("exceptiontable", exceptiontable));
        }

        Ok(result)
    }
}

fn test_bit(b: u8, i: u8) -> bool {
    b & (1 << i) != 0u8
}

fn clear_bit(b: u8, i: u8) -> u8 {
    b & !(1 << i)
}

fn toggle_bit(b: u8, i: u8) -> u8 {
    b ^ (1 << i)
}
