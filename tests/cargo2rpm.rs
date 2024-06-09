use std::fs;

mod utils;
use utils::*;

const PYTHON_VERSIONS: [(u16, u16); 4] = [(3, 10), (3, 11), (3, 12), (3, 13)];

const MODULE_NAMES: [&str; 5] = ["cli", "metadata", "rpm", "semver", "utils"];

#[test]
fn test() -> anyhow::Result<()> {
    let root = env!("CARGO_MANIFEST_DIR");

    for module in MODULE_NAMES {
        for (py_major, py_minor) in PYTHON_VERSIONS {
            let old_path = format!("{root}/tests/data/cargo2rpm/{module}.cpython-{py_major}{py_minor}.pyc");
            let new_path = format!("{root}/tests/data/cargo2rpm/{module}.cpython-{py_major}{py_minor}.pyc.fixed");

            // remove output file if it already exists
            let _ = fs::remove_file(&new_path);

            println!("Processing: {old_path}");
            parse_and_rewrite(&old_path, &new_path)?;

            println!("Checking: {new_path}");
            compare_with_python(&old_path, &new_path, (py_major, py_minor))?;

            // clean up output file
            let _ = fs::remove_file(&new_path);
        }
    }

    Ok(())
}
