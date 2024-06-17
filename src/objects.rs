use std::fmt::{self, Display, Write};

use num_bigint::BigInt;

#[path = "print.rs"]
mod print;

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
#[derive(Clone, Copy, Debug)]
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

/// String type flag for string-like objects
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, PartialEq)]
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
/// "marshal" format, including the optional reference index.
#[derive(Clone, Debug, PartialEq)]
pub struct Object {
    index: Option<u32>,
    inner: ObjectValue,
}

#[allow(non_snake_case)]
impl Object {
    /// Obtain a reference to the inner [`ObjectValue`]
    pub fn inner(&self) -> &ObjectValue {
        &self.inner
    }

    /// Consume this [`Object`] to obtain the inner [`ObjectValue`]
    pub fn into_inner(self) -> ObjectValue {
        self.inner
    }

    /// Obtain the optional reference index of this [`Object`]
    pub fn index(&self) -> Option<u32> {
        self.index
    }

    pub(crate) fn is_null(&self) -> bool {
        self.inner == ObjectValue::Null
    }

    pub(crate) fn NULL(index: Option<u32>) -> Object {
        Object {
            index,
            inner: ObjectValue::Null,
        }
    }

    pub(crate) fn None(index: Option<u32>) -> Object {
        Object {
            index,
            inner: ObjectValue::None,
        }
    }

    pub(crate) fn False(index: Option<u32>) -> Object {
        Object {
            index,
            inner: ObjectValue::False,
        }
    }

    pub(crate) fn True(index: Option<u32>) -> Object {
        Object {
            index,
            inner: ObjectValue::True,
        }
    }

    pub(crate) fn StopIteration(index: Option<u32>) -> Object {
        Object {
            index,
            inner: ObjectValue::StopIteration,
        }
    }

    pub(crate) fn Ellipsis(index: Option<u32>) -> Object {
        Object {
            index,
            inner: ObjectValue::Ellipsis,
        }
    }

    pub(crate) fn Int(index: Option<u32>, value: u32) -> Object {
        Object {
            index,
            inner: ObjectValue::Int(value),
        }
    }

    pub(crate) fn BinaryFloat(index: Option<u32>, value: f64) -> Object {
        Object {
            index,
            inner: ObjectValue::BinaryFloat(value),
        }
    }

    pub(crate) fn BinaryComplex(index: Option<u32>, real: f64, imag: f64) -> Object {
        Object {
            index,
            inner: ObjectValue::BinaryComplex((real, imag)),
        }
    }

    pub(crate) fn String(index: Option<u32>, typ: StringType, bytes: Vec<u8>) -> Object {
        Object {
            index,
            inner: ObjectValue::String { typ, bytes },
        }
    }

    pub(crate) fn Tuple(index: Option<u32>, elements: Vec<Object>) -> Object {
        Object {
            index,
            inner: ObjectValue::Tuple(elements),
        }
    }

    pub(crate) fn List(index: Option<u32>, elements: Vec<Object>) -> Object {
        Object {
            index,
            inner: ObjectValue::List(elements),
        }
    }

    pub(crate) fn Set(index: Option<u32>, elements: Vec<Object>) -> Object {
        Object {
            index,
            inner: ObjectValue::Set(elements),
        }
    }

    pub(crate) fn FrozenSet(index: Option<u32>, elements: Vec<Object>) -> Object {
        Object {
            index,
            inner: ObjectValue::FrozenSet(elements),
        }
    }

    pub(crate) fn Dict(index: Option<u32>, elements: Vec<(Object, Object)>) -> Object {
        Object {
            index,
            inner: ObjectValue::Dict(elements),
        }
    }

    pub(crate) fn Long(index: Option<u32>, value: BigInt) -> Object {
        Object {
            index,
            inner: ObjectValue::Long(value),
        }
    }

    pub(crate) fn Ref(index: Option<u32>, value: u32) -> Object {
        Object {
            index,
            inner: ObjectValue::Ref(value),
        }
    }

    pub(crate) fn Code(index: Option<u32>, value: Box<CodeObject>) -> Object {
        Object {
            index,
            inner: ObjectValue::Code(value),
        }
    }
}

impl Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.pretty_print(f, 0, "", self.index)
    }
}

impl Object {
    pub(crate) fn pretty_print<W>(&self, writer: &mut W, indent: usize, prefix: &str) -> fmt::Result
    where
        W: Write,
    {
        self.inner.pretty_print(writer, indent, prefix, self.index)
    }
}

/// ## Object values in the binary "marshal" format
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ObjectValue {
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

impl Display for ObjectValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.pretty_print(f, 0, "", None)
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
