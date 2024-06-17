#![allow(missing_docs)]

use clap::Parser;
use log::{LevelFilter, Metadata, Record};

use marshal_parser::MarshalFile;

struct Logger {
    filter: LevelFilter,
}

impl Logger {
    fn init(filter: LevelFilter) {
        let logger = Box::new(Logger { filter });

        log::set_logger(Box::leak(logger))
            .map(|()| log::set_max_level(filter))
            .expect("Failed to set up logger.")
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.filter
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

    Logger::init(if args.debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    });

    // parse version into (major, minor)
    let version: Option<(u16, u16)> = if let Some(v) = args.python_version {
        Some(parse_py_version(&v)?)
    } else {
        None
    };

    for path in &args.files {
        let marshal = if let Some((major, minor)) = version {
            MarshalFile::from_dump_path(path, (major, minor))?
        } else {
            MarshalFile::from_pyc_path(path)?
        };

        if args.print {
            println!("{}", marshal.inner());
        }

        if args.unused {
            marshal.print_unused_ref_flags();
        }

        if args.fix {
            let mut path = path.to_string();
            if !args.overwrite {
                path.push_str(".fixed");
            }

            marshal.write_normalized(path)?;
        }
    }

    Ok(())
}
