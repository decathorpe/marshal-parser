#![allow(missing_docs)]

use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};

use clap::Parser;
use log::{Level, LevelFilter, Metadata, Record};

use marshal_parser::MarshalObject;

struct Logger {
    level: Level,
}

impl Logger {
    fn init(level: Level) {
        let logger = Box::new(Logger { level });

        log::set_logger(Box::leak(logger))
            .map(|()| log::set_max_level(LevelFilter::Info))
            .unwrap()
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!("{}", record.args());
        }
    }

    fn flush(&self) {}
}

/// Parser and fixer for pyc files and marshal dumps
#[derive(Debug, Parser)]
struct Args {
    /// Print human-readable parser output
    #[arg(long, short)]
    print: bool,
    /// Print unused reference flags
    #[arg(long, short)]
    unused: bool,
    /// Clear unused reference flags from objects
    #[arg(long, short)]
    fix: bool,
    /// Python version for marshal dumps without pyc header (major.minor)
    #[arg(long, short = 'V')]
    python_version: Option<String>,
    /// Overwrite existing file when clearing unused references
    #[arg(long, short)]
    overwrite: bool,
    /// Print verbose debugging output
    #[arg(long, short)]
    debug: bool,
    /// File path(s)
    files: Vec<String>,
}

fn parse_py_version(v: &str) -> anyhow::Result<(u16, u16)> {
    let mut split = v.splitn(2, '.');
    match (split.next(), split.next()) {
        (Some(major), Some(minor)) => Ok((major.parse()?, minor.parse()?)),
        _ => Err(anyhow::anyhow!("Invalid version string: {}", v)),
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    Logger::init(if args.debug { Level::Debug } else { Level::Info });

    // parse version into (major, minor)
    let version: Option<(u16, u16)> = if let Some(v) = args.python_version {
        Some(parse_py_version(&v)?)
    } else {
        None
    };

    for path in &args.files {
        let file = OpenOptions::new()
            .read(true)
            .write(args.overwrite)
            .create_new(false)
            .open(path)?;

        // buffered IO is more efficient here
        let mut reader = BufReader::new(file);

        let marshal = if let Some((major, minor)) = version {
            MarshalObject::parse_dump(&mut reader, (major, minor))?
        } else {
            MarshalObject::parse_pyc(&mut reader)?
        };

        if args.print {
            // print human-readable parsed state
            println!("{:#x?}", marshal.inner());
            // TODO: make this better somehow?
        }

        if args.unused {
            // find and print unused references
            marshal.print_unused_ref_flags();
        }

        if args.fix {
            let mut file = if args.overwrite {
                // keep same file open
                reader.into_inner()
            } else {
                // copy file contents to new file
                let mut old = reader.into_inner();
                old.seek(SeekFrom::Start(0))?;

                // read old file contents
                let mut buf = Vec::new();
                old.read_to_end(&mut buf)?;

                // copy contents contents to new file
                let mut new = File::create_new(format!("{}.fixed", path))?;
                new.write_all(&buf)?;

                new.seek(SeekFrom::Start(0))?;
                new
            };

            marshal.clear_unused_ref_flags(&mut file)?;
        }
    }

    Ok(())
}
