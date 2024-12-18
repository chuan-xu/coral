fn main() -> std::io::Result<()> {
    // -- Uncomment when the proto file is updated --
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = std::path::Path::new(&manifest_dir);
    let protos_path = manifest_path.join("src/");
    let mut conf = prost_build::Config::new();
    // conf.type_attribute(".", "#[wasm_bindgen::prelude::wasm_bindgen]");
    // conf.type_attribute(".", "#[derive(coral_macro::WasmAttr)]");
    conf.out_dir(&protos_path);
    conf.compile_protos(
        &["../coral-wasm/protos/payload.proto"],
        &["../coral-wasm/protos/"],
    )?;
    Ok(())
}
