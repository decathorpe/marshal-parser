use std::fs;

use glob::glob;

mod utils;
use utils::*;

macro_rules! test_python {
    ($name:ident, $major:literal, $minor:literal) => {
        #[test]
        fn $name() -> anyhow::Result<()> {
            let root = env!("CARGO_MANIFEST_DIR");

            let paths = glob(&format!("{root}/tests/data/python{}.{}/*.pyc", $major, $minor))?;

            for path in paths {
                let old_path = path?.to_string_lossy().into_owned();
                let mut new_path = old_path.clone();
                new_path.push_str(".fixed");

                // remove output file if it already exists
                let _ = fs::remove_file(&new_path);

                println!("Processing: {old_path}");
                parse_and_rewrite(&old_path, &new_path)?;

                println!("Checking: {new_path}");
                compare_with_python(&old_path, &new_path, ($major, $minor))?;

                // clean up output file
                let _ = fs::remove_file(&new_path);
            }

            Ok(())
        }
    };
}

test_python!(py_3_8, 3, 8);
test_python!(py_3_9, 3, 9);
test_python!(py_3_10, 3, 10);
test_python!(py_3_11, 3, 11);
test_python!(py_3_12, 3, 12);
test_python!(py_3_13, 3, 13);
