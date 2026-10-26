use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-check-cfg=cfg(slml_web_built)");
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
    let wasm = std::env::var_os("SLML_WASM_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("target/wasm32-unknown-unknown/release/slml_web.wasm"));
    println!("cargo:rerun-if-env-changed=SLML_WASM_PATH");
    println!("cargo:rerun-if-changed={}", wasm.display());
    if wasm.is_file() {
        println!("cargo:rustc-cfg=slml_web_built");
        println!(
            "cargo:rustc-env=SLML_WASM_PATH={}",
            wasm.canonicalize().unwrap().display()
        );
    }
}
