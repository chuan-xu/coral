fn main() -> std::io::Result<()> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = std::path::Path::new(&manifest_dir);
    let record_rs_path = manifest_path.join("src/logs");
    let mut conf = prost_build::Config::new();
    conf.out_dir(record_rs_path);
    conf.compile_protos(&["src/logs/logs.proto"], &["src/logs"])?;
    Ok(())
}
