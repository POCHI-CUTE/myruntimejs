use deno_core::{include_js_files, snapshot, Extension};
use std::env;
use std::path::PathBuf;

fn main() {
    let esm_files = Vec::from(include_js_files!(my_extension dir "src", "runtime.js"));
    let runjs_extension = Extension {
        name: "runjs",
        esm_files: std::borrow::Cow::Owned(Vec::from(
            include_js_files!(my_extension dir "src", "runtime.js"),
        )),
        ..Default::default()
    };

    let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let snapshot_path = o.join("RUNJS_SNAPSHOT.bin");

    let _ = deno_core::snapshot::create_snapshot(
        deno_core::snapshot::CreateSnapshotOptions {
            cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
            extensions: vec![runjs_extension],
            startup_snapshot: None,
            skip_op_registration: false,
            with_runtime_cb: None,
            extension_transpiler: None,
        },
        None,
    );
}
