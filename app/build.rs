//! Bundles sounds and icons as a GResource, and compiles the GSettings schema next to the
//! build artifacts for running from the source tree.

use std::process::Command;

fn main() {
    glib_build_tools::compile_resources(
        &["../data"],
        "../data/diktu.gresource.xml",
        "diktu.gresource",
    );

    let out = std::env::var("OUT_DIR").unwrap();
    let schema = "../data/fr.gwenael_leger.Diktu.gschema.xml";
    println!("cargo:rerun-if-changed={schema}");
    let mut source = "../data".to_owned();
    if std::env::var_os("CARGO_FEATURE_DEVEL").is_some() {
        // Same keys under the development app ID, so its settings stay apart.
        source = format!("{out}/schema");
        std::fs::create_dir_all(&source).unwrap();
        let xml = std::fs::read_to_string(schema)
            .unwrap()
            .replace(
                "\"fr.gwenael_leger.Diktu\"",
                "\"fr.gwenael_leger.Diktu.Devel\"",
            )
            .replace("/fr/gwenael_leger/Diktu/", "/fr/gwenael_leger/Diktu/Devel/");
        std::fs::write(
            format!("{source}/fr.gwenael_leger.Diktu.Devel.gschema.xml"),
            xml,
        )
        .unwrap();
    }
    let status = Command::new("glib-compile-schemas")
        .args(["--strict", "--targetdir", &out, &source])
        .status()
        .expect("glib-compile-schemas is required (libglib2.0-bin / glib2)");
    assert!(status.success(), "invalid GSettings schema");
}
