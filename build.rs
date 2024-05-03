use deno_core::{extension, snapshot};
use std::env;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    extension!(runjs, js = ["src/runtime.js",]);

    let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let snapshot_path = o.join("RUNJS_SNAPSHOT.bin");

    let output = snapshot::create_snapshot(
        snapshot::CreateSnapshotOptions {
            cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
            extensions: vec![runjs::init_ops_and_esm()],
            startup_snapshot: None,
            skip_op_registration: false,
            with_runtime_cb: None,
            extension_transpiler: None,
        },
        None,
    )
    .unwrap();

    let mut file = std::fs::File::create(snapshot_path).unwrap();
    if cfg!(debug_assertions) {
        file.write_all(&output.output).unwrap();
    } else {
        let mut vec = Vec::with_capacity(output.output.len());
        vec.extend((output.output.len() as u32).to_le_bytes());
        vec.extend_from_slice(
            &zstd::bulk::compress(&output.output, 22).expect("snapshot compression failed"),
        );
        file.write_all(&vec).unwrap();
    }

    for path in output.files_loaded_during_snapshot {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}
