//! Bundles sounds and icons as a GResource, and compiles the GSettings schema next to the
//! build artifacts for running from the source tree.

use std::process::Command;

fn main() {
    glib_build_tools::compile_resources(
        &["../data"],
        "../data/parlotte.gresource.xml",
        "parlotte.gresource",
    );

    let out = std::env::var("OUT_DIR").unwrap();
    println!("cargo:rerun-if-changed=../data/fr.gwenael_leger.Parlotte.gschema.xml");
    let status = Command::new("glib-compile-schemas")
        .args(["--strict", "--targetdir", &out, "../data"])
        .status()
        .expect("glib-compile-schemas is required (libglib2.0-bin / glib2)");
    assert!(status.success(), "invalid GSettings schema");
}
