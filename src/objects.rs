use std::fmt::{self, Display};

use num_bigint::BigInt;

#[derive(Debug)]
pub(crate) enum ObjectType {
    Null,
    None,
    False,
    True,
    StopIteration,
    Ellipsis,
    Int,
    Int64,
    Float,
    BinaryFloat,
    Complex,
    BinaryComplex,
    Long,
    String,
    Interned,
    Ref,
    Tuple,
    List,
    Dict,
    Code,
    Unicode,
    Unknown,
    Set,
    FrozenSet,
    Ascii,
    AsciiInterned,
    SmallTuple,
    ShortAscii,
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
pub(crate) enum Object {
    Null,
    None,
    False,
    True,
    StopIteration,
    Ellipsis,

    Int(u32),
    BinaryFloat(f64),
    BinaryComplex((f64, f64)),
    String(Vec<u8>),

    Tuple(Vec<Object>),
    List(Vec<Object>),
    Set(Vec<Object>),
    FrozenSet(Vec<Object>),
    Dict(Vec<(Object, Object)>),

    Long(BigInt),
    Ref((usize, u32)),
    Code(Vec<(&'static str, Object)>),
}
