#![allow(missing_docs)]

use std::fs::File;
use std::io::Read;

use criterion::*;

use marshal_parser::*;

const MODULE_NAMES: [&str; 5] = ["cli", "metadata", "rpm", "semver", "utils"];

macro_rules! bench_modules {
    ($name:ident, $major:literal, $minor:literal) => {
        fn $name(c: &mut Criterion) {
            let root = env!("CARGO_MANIFEST_DIR");

            let mut bench_group = c.benchmark_group(&format!("py_{}_{}", $major, $minor));

            for module in MODULE_NAMES {
                let path = format!(
                    "{}/tests/data/cargo2rpm/{}.cpython-{}{}.pyc",
                    root, module, $major, $minor
                );

                let mut file = File::options()
                    .read(true)
                    .write(false)
                    .create_new(false)
                    .open(path)
                    .unwrap();

                let mut data = Vec::new();
                file.read_to_end(&mut data).unwrap();

                bench_group.throughput(Throughput::Bytes(data.len() as u64));
                bench_group.bench_with_input(BenchmarkId::from_parameter(module), module, |b, _| {
                    b.iter(|| {
                        let data = black_box(&data);
                        let marshal = MarshalObject::parse_pyc(data).unwrap();
                        let result = marshal.clear_unused_ref_flags(&data).unwrap();
                        black_box(result);
                    });
                });
            }
            bench_group.finish();
        }
    };
}

//bench_modules!(py_3_10, 3, 10);
//bench_modules!(py_3_11, 3, 11);
bench_modules!(py_3_12, 3, 12);
//bench_modules!(py_3_13, 3, 13);

//criterion_group!(benches, py_3_10, py_3_11, py_3_12, py_3_13);
criterion_group!(benches, py_3_12);
criterion_main!(benches);
