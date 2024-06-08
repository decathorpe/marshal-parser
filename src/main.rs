use clap::Parser;

mod magic;
mod objects;
mod parser;

use log::{Level, LevelFilter, Metadata, Record};

struct Logger {}

impl Logger {
    fn init() {
        static LOGGER: Logger = Logger {};

        log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(LevelFilter::Info))
            .unwrap()
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!("{}", record.args());
        }
    }

    fn flush(&self) {}
}

/// Marshalparser and fixer for .pyc files
#[derive(Debug, Parser)]
struct Args {
    /// Print human-readable parser output
    #[arg(long, short)]
    print: bool,
    /// Print unused references
    #[arg(long, short)]
    unused: bool,
    /// Fix references
    #[arg(long, short)]
    fix: bool,
    /// Overwrite existing .pyc file
    #[arg(long, short)]
    overwrite: bool,
    files: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    Logger::init();

    let args = Args::parse();

    for file in &args.files {
        let (object, _parser, _objs, _file) = parser::parse_pyc_file(file)?;
        //println!("{:#?}", _objs);

        if args.print {
            // print human-readable parsed state
            println!("{:#?}", object);
            // TODO: make this better
        }

        if args.unused {
            // find and print unused references
            for r in _objs {
                if r.usages == 0 {
                    println!(
                        "Unused reference bit: {} object with reference index {} at offset {}",
                        r.typ, r.index, r.offset
                    );
                }
            }
        }

        if args.fix {
            todo!();

            // find and clear unused ref flags
            // reshuffle indices

            // if args.overwrite: overwrite file contents
            // else: open file and write new contents
        }
    }

    Ok(())
}
