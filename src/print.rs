use super::*;

impl ObjectValue {
    pub(crate) fn pretty_print<W>(&self, writer: &mut W, indent: usize, prefix: &str, index: Option<u32>) -> fmt::Result
    where
        W: Write,
    {
        let indent_str = " ".repeat(indent) + prefix;
        let index_str = index.map(|i| format!(" (ref: {i})")).unwrap_or_default();

        match self {
            ObjectValue::Null => writeln!(writer, "{}object{}: NULL", indent_str, index_str),
            ObjectValue::None => writeln!(writer, "{}object{}: None", indent_str, index_str),
            ObjectValue::False => writeln!(writer, "{}object{}: False", indent_str, index_str),
            ObjectValue::True => writeln!(writer, "{}object{}: True", indent_str, index_str),
            ObjectValue::StopIteration => writeln!(writer, "{}object{}: StopIteration", indent_str, index_str),
            ObjectValue::Ellipsis => writeln!(writer, "{}object{}: ...", indent_str, index_str),
            ObjectValue::Int(x) => writeln!(writer, "{}object{}: int: {}", indent_str, index_str, x),
            ObjectValue::BinaryFloat(x) => writeln!(writer, "{}object{}: float: {}", indent_str, index_str, x),
            ObjectValue::BinaryComplex(x) => {
                writeln!(writer, "{}object{}: complex: ({}, {})", indent_str, index_str, x.0, x.1)
            },
            ObjectValue::String { typ, bytes } => pretty_print_string(writer, indent, prefix, *typ, bytes, &index_str),
            ObjectValue::Tuple(x) => {
                writeln!(writer, "{}object{}: tuple (length {}):", indent_str, index_str, x.len())?;
                for obj in x {
                    obj.pretty_print(writer, indent + 2, "- ")?;
                }
                Ok(())
            },
            ObjectValue::List(x) => {
                writeln!(writer, "{}object{}: list (length {}):", indent_str, index_str, x.len())?;
                for obj in x {
                    obj.pretty_print(writer, indent + 2, "- ")?;
                }
                Ok(())
            },
            ObjectValue::Set(x) => {
                writeln!(writer, "{}object{}: set (length {}):", indent_str, index_str, x.len())?;
                for obj in x {
                    obj.pretty_print(writer, indent + 2, "- ")?;
                }
                Ok(())
            },
            ObjectValue::FrozenSet(x) => {
                writeln!(
                    writer,
                    "{}object{}: frozenset (length {}):",
                    indent_str,
                    index_str,
                    x.len()
                )?;
                for obj in x {
                    obj.pretty_print(writer, indent + 2, "- ")?;
                }
                Ok(())
            },
            ObjectValue::Dict(x) => {
                writeln!(writer, "{}object{}: dict (length {}):", indent_str, index_str, x.len())?;
                for (key, value) in x {
                    key.pretty_print(writer, indent + 2, "- key: ")?;
                    value.pretty_print(writer, indent + 2, "- value: ")?;
                }
                Ok(())
            },
            ObjectValue::Long(x) => writeln!(writer, "{}object{}: long: {}", indent_str, index_str, x),
            ObjectValue::Ref(x) => writeln!(writer, "{}object{}: ref: {}", indent_str, index_str, x),
            ObjectValue::Code(x) => {
                writeln!(writer, "{}object{}: code:", indent_str, index_str)?;
                x.pretty_print(writer, indent + 2, "- ")
            },
        }
    }
}

#[cfg(feature = "fancy")]
fn pretty_print_string<W>(
    writer: &mut W,
    indent: usize,
    prefix: &str,
    typ: StringType,
    bytes: &[u8],
    index_str: &str,
) -> fmt::Result
where
    W: Write,
{
    let indent_str = " ".repeat(indent) + prefix;

    if matches!(typ, StringType::Ascii | StringType::AsciiInterned) {
        let s: String = String::from_utf8_lossy(bytes).escape_debug().collect();
        writeln!(
            writer,
            "{}object{}: string (type {}, length {}): \"{}\"",
            indent_str,
            index_str,
            typ,
            s.len(),
            s
        )
    } else if bytes.is_empty() {
        writeln!(
            writer,
            "{}object{}: string (type {}, length {}): []",
            indent_str,
            index_str,
            typ,
            bytes.len(),
        )
    } else {
        let mut indent_str_dump = " ".repeat(indent + 2);
        indent_str_dump.push_str("| ");
        let hex_dump = pretty_hex::config_hex(
            &bytes,
            pretty_hex::HexConfig {
                title: false,
                ascii: true,
                width: 8,
                ..Default::default()
            },
        );

        writeln!(
            writer,
            "{}object{}: string (type {}, length {}):",
            indent_str,
            index_str,
            typ,
            bytes.len(),
        )?;
        writeln!(writer, "{}", textwrap::indent(&hex_dump, &indent_str_dump))
    }
}

#[cfg(not(feature = "fancy"))]
fn pretty_print_string<W>(
    writer: &mut W,
    indent: usize,
    prefix: &str,
    typ: StringType,
    bytes: &[u8],
    index_str: &str,
) -> fmt::Result
where
    W: Write,
{
    let indent_str = " ".repeat(indent) + prefix;

    if matches!(typ, StringType::Ascii | StringType::AsciiInterned) {
        let s: String = String::from_utf8_lossy(bytes).escape_debug().collect();
        writeln!(
            writer,
            "{}object{}: string (type {}, length {}): \"{}\"",
            indent_str,
            index_str,
            typ,
            s.len(),
            s
        )
    } else {
        writeln!(
            writer,
            "{}object{}: string (type {}, length {}): {:x?}",
            indent_str,
            index_str,
            typ,
            bytes.len(),
            bytes
        )
    }
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
