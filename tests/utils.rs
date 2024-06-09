use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::process::Command;

use marshal_parser::*;

pub fn parse_and_rewrite(old_path: &str, new_path: &str) -> anyhow::Result<()> {
    let file = File::options()
        .read(true)
        .write(false)
        .create_new(false)
        .open(old_path)?;

    let mut reader = BufReader::new(file);
    let mut data = Vec::new();
    reader.read_to_end(&mut data)?;

    let marshal = MarshalObject::parse_pyc(&data)?;
    let result = marshal.clear_unused_ref_flags(&data);

    let new = File::create_new(new_path)?;
    let mut writer = BufWriter::new(new);

    writer.write_all(&result)?;

    Ok(())
}

pub fn pyc_header_length(version: (u16, u16)) -> usize {
    if version >= (3, 7) {
        16
    } else if version >= (3, 3) {
        12
    } else {
        8
    }
}

pub fn compare_with_python(old: &str, new: &str, (py_major, py_minor): (u16, u16)) -> anyhow::Result<()> {
    let result = Command::new(format!("python{py_major}.{py_minor}"))
        .arg("tests/compare.py")
        .arg(old)
        .arg(new)
        .arg(pyc_header_length((py_major, py_minor)).to_string())
        .output()?;

    if result.status.code() == Some(0) {
        Ok(())
    } else {
        anyhow::bail!("Deserialized marshal dumps differ")
    }
}
