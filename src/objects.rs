use std::fmt::{self, Display, Write};

use num_bigint::BigInt;

/// ## Object type flag in the binary "marshal" format
///
/// This enum represents the type of objects as determined by the first byte of
/// their representation in the binary "marshal" format.
///
/// *Note*: Some types are not handled in this implementation, since they were
/// replaced with other types and are not written by recent versions of Python:
///
/// - `'T'` (`TYPE_INT64`)
/// - `'f'` (`TYPE_FLOAT`)
/// - `'x'` (`TYPE_COMPLEX`)
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ObjectType {
    /// Type of a null pointer
    Null,
    /// Type of the `None` singleton object
    None,
    /// Type of `False`
    False,
    /// Type of `True`
    True,
    /// Type of the `StopIteration` singleton object
    StopIteration,
    /// Type of the `...` (ellipsis) singleton object
    Ellipsis,
    /// Type of 32-bit integers
    Int,
    #[doc(hidden)]
    Int64,
    #[doc(hidden)]
    Float,
    /// Type of 64-bit floating-point numbers
    BinaryFloat,
    #[doc(hidden)]
    Complex,
    /// Type of 64-bit floating-point complex numbers
    BinaryComplex,
    /// Type of dynamically sized integers
    Long,
    /// Type of strings
    String,
    /// Type of interned strings
    Interned,
    /// Type of object references
    Ref,
    /// Type of tuples
    Tuple,
    /// Type of lists
    List,
    /// Type of dicts
    Dict,
    /// Type of code objects
    Code,
    /// Type of unicode strings
    Unicode,
    /// Type of unknown objects
    Unknown,
    /// Type of sets
    Set,
    /// Type of frozensets
    FrozenSet,
    /// Type of ASCII strings
    Ascii,
    /// Type of interned ASCII strings
    AsciiInterned,
    /// Type of small tuples
    SmallTuple,
    /// Type of short ASCII strings
    ShortAscii,
    /// Type of short interned ASCII strings
    ShortAsciiInterned,
}

impl Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryFrom<u8> for ObjectType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        use ObjectType as T;

        Ok(match value {
            b'0' => T::Null,
            b'N' => T::None,
            b'F' => T::False,
            b'T' => T::True,
            b'S' => T::StopIteration,
            b'.' => T::Ellipsis,
            b'i' => T::Int,
            b'I' => T::Int64,
            b'f' => T::Float,
            b'g' => T::BinaryFloat,
            b'x' => T::Complex,
            b'y' => T::BinaryComplex,
            b'l' => T::Long,
            b's' => T::String,
            b't' => T::Interned,
            b'r' => T::Ref,
            b'(' => T::Tuple,
            b'[' => T::List,
            b'{' => T::Dict,
            b'c' => T::Code,
            b'u' => T::Unicode,
            b'?' => T::Unknown,
            b'<' => T::Set,
            b'>' => T::FrozenSet,
            b'a' => T::Ascii,
            b'A' => T::AsciiInterned,
            b')' => T::SmallTuple,
            b'z' => T::ShortAscii,
            b'Z' => T::ShortAsciiInterned,
            _ => return Err(()),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StringType {
    String,
    Interned,
    Unicode,
    Ascii,
    AsciiInterned,
}

impl Display for StringType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StringType::String => write!(f, "STRING"),
            StringType::Interned => write!(f, "INTERNED"),
            StringType::Unicode => write!(f, "UNICODE"),
            StringType::Ascii => write!(f, "ASCII"),
            StringType::AsciiInterned => write!(f, "ASCII_INTERNED"),
        }
    }
}

/// ## Python objects as represented in the binary "marshal" format
///
/// This enum represents Python objects as they are represented in the binary
/// "marshal" format.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Object {
    /// null object
    Null,
    /// `None` singleton object
    None,
    /// `False` object
    False,
    /// `True` object
    True,
    /// `StopIteration` singleton
    StopIteration,
    /// `...` (ellipsis) singleton
    Ellipsis,

    /// 32-bit integer
    Int(u32),
    /// 64-bit floating-point number
    BinaryFloat(f64),
    /// 64-bit floating-point complex number
    BinaryComplex((f64, f64)),
    /// string
    #[allow(missing_docs)]
    String { typ: StringType, bytes: Vec<u8> },

    /// tuple object (collection of objects)
    Tuple(Vec<Object>),
    /// list object (collection of objects)
    List(Vec<Object>),
    /// set object (collection of objects)
    Set(Vec<Object>),
    /// frozenset object (collection of objects)
    FrozenSet(Vec<Object>),
    /// dict object (collection of objects in key / value pairs)
    Dict(Vec<(Object, Object)>),

    /// dynamically-sized integer
    Long(BigInt),
    /// reference object
    Ref(u32),
    /// code object
    Code(Box<CodeObject>),
}

impl Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.pretty_print(f, 0, "")
    }
}

impl Object {
    pub(crate) fn pretty_print<W>(&self, writer: &mut W, indent: usize, prefix: &str) -> fmt::Result
    where
        W: Write,
    {
        let indent_str = " ".repeat(indent) + prefix;

        match self {
            Object::Null => writeln!(writer, "{}NULL", indent_str),
            Object::None => writeln!(writer, "{}None", indent_str),
            Object::False => writeln!(writer, "{}False", indent_str),
            Object::True => writeln!(writer, "{}True", indent_str),
            Object::StopIteration => writeln!(writer, "{}StopIteration", indent_str),
            Object::Ellipsis => writeln!(writer, "{}...", indent_str),
            Object::Int(x) => writeln!(writer, "{}int: {}", indent_str, x),
            Object::BinaryFloat(x) => writeln!(writer, "{}float: {}", indent_str, x),
            Object::BinaryComplex(x) => writeln!(writer, "{}complex: ({}, {})", indent_str, x.0, x.1),
            Object::String { typ, bytes } => {
                if matches!(typ, StringType::Ascii | StringType::AsciiInterned) {
                    let s: String = String::from_utf8_lossy(bytes).escape_debug().collect();
                    writeln!(
                        writer,
                        "{}string (type {}, length {}): \"{}\"",
                        indent_str,
                        typ,
                        s.len(),
                        s
                    )
                } else {
                    writeln!(
                        writer,
                        "{}string (type {}, length {}): {:x?}",
                        indent_str,
                        typ,
                        bytes.len(),
                        bytes
                    )
                }
            },
            Object::Tuple(x) => {
                writeln!(writer, "{}tuple (length {}):", indent_str, x.len())?;
                for obj in x {
                    obj.pretty_print(writer, indent + 2, "- ")?;
                }
                Ok(())
            },
            Object::List(x) => {
                writeln!(writer, "{}list (length {}):", indent_str, x.len())?;
                for obj in x {
                    obj.pretty_print(writer, indent + 2, "- ")?;
                }
                Ok(())
            },
            Object::Set(x) => {
                writeln!(writer, "{}set (length {}):", indent_str, x.len())?;
                for obj in x {
                    obj.pretty_print(writer, indent + 2, "- ")?;
                }
                Ok(())
            },
            Object::FrozenSet(x) => {
                writeln!(writer, "{}frozenset (length {}):", indent_str, x.len())?;
                for obj in x {
                    obj.pretty_print(writer, indent + 2, "- ")?;
                }
                Ok(())
            },
            Object::Dict(x) => {
                writeln!(writer, "{}dict (length {}):", indent_str, x.len())?;
                for (key, value) in x {
                    key.pretty_print(writer, indent + 2, "- key: ")?;
                    value.pretty_print(writer, indent + 2, "- value: ")?;
                }
                Ok(())
            },
            Object::Long(x) => writeln!(writer, "{}long: {}", indent_str, x),
            Object::Ref(x) => writeln!(writer, "{}ref: {}", indent_str, x),
            Object::Code(x) => {
                writeln!(writer, "{}code:", indent_str)?;
                x.pretty_print(writer, indent + 2, "- ")
            },
        }
    }
}

/// ## Code objects as represented in the binary "marshal" format
///
/// The exact layout of this object in the binary format differs between Python
/// versions. Some fields are present in all Python versions, some fields have
/// been added, some fields have been removed.
#[derive(Clone, Debug, PartialEq)]
#[allow(missing_docs)]
#[non_exhaustive]
pub struct CodeObject {
    pub argcount: u32,
    /// added in Python 3.8+
    pub posonlyargcount: Option<u32>,
    pub kwonlyargcount: u32,
    /// removed in Python 3.11+
    pub nlocals: Option<u32>,
    pub stacksize: u32,
    pub flags: u32,
    pub code: Object,
    pub consts: Object,
    pub names: Object,
    /// removed in Python 3.11+
    pub varnames: Option<Object>,
    /// removed in Python 3.11+
    pub freevars: Option<Object>,
    /// removed in Python 3.11+
    pub cellvars: Option<Object>,
    /// added in Python 3.11+
    pub localsplusnames: Option<Object>,
    /// added in Python 3.11+
    pub localspluskinds: Option<Object>,
    pub filename: Object,
    pub name: Object,
    /// added in Python 3.11+
    pub qualname: Option<Object>,
    pub firstlineno: u32,
    pub linetable: Object,
    /// added in Python 3.11+
    pub exceptiontable: Option<Object>,
}

impl CodeObject {
    pub(crate) fn pretty_print<W>(&self, writer: &mut W, indent: usize, prefix: &str) -> fmt::Result
    where
        W: Write,
    {
        let indent_str = " ".repeat(indent) + prefix;

        writeln!(writer, "{}argcount: {}", indent_str, self.argcount)?;

        if let Some(posonlyargcount) = &self.posonlyargcount {
            writeln!(writer, "{}posonlyargcount: {}", indent_str, posonlyargcount)?;
        }

        writeln!(writer, "{}kwonlyargcount: {}", indent_str, self.kwonlyargcount)?;

        if let Some(nlocals) = &self.nlocals {
            writeln!(writer, "{}nlocals: {}", indent_str, nlocals)?;
        }

        writeln!(writer, "{}stacksize: {}", indent_str, self.stacksize)?;
        writeln!(writer, "{}flags: {}", indent_str, self.flags)?;

        self.code.pretty_print(writer, indent, "- code: ")?;
        self.consts.pretty_print(writer, indent, "- consts: ")?;
        self.names.pretty_print(writer, indent, "- names: ")?;

        if let Some(varnames) = &self.varnames {
            varnames.pretty_print(writer, indent, "- varnames: ")?;
        }

        if let Some(freevars) = &self.freevars {
            freevars.pretty_print(writer, indent, "- freevars: ")?;
        }

        if let Some(cellvars) = &self.cellvars {
            cellvars.pretty_print(writer, indent, "- cellvars:  ")?;
        }

        if let Some(localsplusnames) = &self.localsplusnames {
            localsplusnames.pretty_print(writer, indent, "- localsplusnames: ")?;
        }

        if let Some(localspluskinds) = &self.localspluskinds {
            localspluskinds.pretty_print(writer, indent, "- localspluskinds: ")?;
        }

        self.filename.pretty_print(writer, indent, "- filename: ")?;
        self.name.pretty_print(writer, indent, "- name: ")?;

        if let Some(qualname) = &self.qualname {
            qualname.pretty_print(writer, indent, "- qualname: ")?;
        }

        writeln!(writer, "{}firstlineno: {}", indent_str, self.firstlineno)?;
        self.linetable.pretty_print(writer, indent, "- linetable: ")?;

        if let Some(exceptiontable) = &self.exceptiontable {
            exceptiontable.pretty_print(writer, indent, "- exceptiontable: ")?;
        }
        Ok(())
    }
}
