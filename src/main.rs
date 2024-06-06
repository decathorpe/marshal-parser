use clap::Parser;

mod magic;
mod objects;
mod parser;

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
    let args = Args::parse();
    println!("{:#?}", args);

    for file in &args.files {
        // open and parse file

        let (parser, _file) = parser::parse_file(file)?;

        if args.print {
            // print human-readable parsed state
            println!("{:#?}", parser);
            // TODO: make this better
        }

        if args.unused {
            // find and print unused references
        }

        if args.fix {
            // find and clear unused ref flags
            // consider args.overwrite
        }
    }

    Ok(())
}
