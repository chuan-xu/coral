fn main() -> std::io::Result<()> {
    // -- Uncomment when the proto file is updated --
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = std::path::Path::new(&manifest_dir);
    let protos_path = manifest_path.join("src/");
    let mut conf = prost_build::Config::new();
    conf.type_attribute(".", "#[wasm_bindgen::prelude::wasm_bindgen]");
    conf.type_attribute(".", "#[derive(coral_macro::WasmAttr)]");
    conf.out_dir(&protos_path);
    conf.compile_protos(&["../protos/payload.proto"], &["../protos/"])?;
    let payload_rs_file = protos_path.join("payload.rs");
    let content = std::fs::read_to_string(&payload_rs_file)?;
    let modified_content = content.replace("pub ", "pub(crate) ");
    std::fs::write(&payload_rs_file, modified_content)?;
    Ok(())
}
