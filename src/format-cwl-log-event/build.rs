#[macro_use]
extern crate quote;
extern crate pest_generator;

use pest_generator::derive_parser;
use std::{env, fs::File, io::prelude::*, path::Path};

fn main() {
    let cargo_manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let pest = Path::new(&cargo_manifest_dir).join("src/grammar.pest");
    let rs = Path::new(&out_dir).join("format_cwl_log_event_parser.rs");
    println!("cargo:rerun-if-changed=src/grammar.pest");

    let derived = {
        let path = pest.to_string_lossy();
        let pest = quote! {
            #[grammar = #path]
            pub struct FormatCwlLogEventParser;
        };
        derive_parser(pest, false)
    };

    let mut file = File::create(rs).unwrap();
    writeln!(file, "pub struct FormatCwlLogEventParser;\n{}", derived).unwrap();
}
