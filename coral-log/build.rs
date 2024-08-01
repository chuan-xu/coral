fn main() -> std::io::Result<()> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = std::path::Path::new(&manifest_dir);
    let record_rs_paht = manifest_path.join("src");
    let mut conf = prost_build::Config::new();
    conf.out_dir(record_rs_paht);
    conf.compile_protos(&["src/format.proto"], &["src/"])?;
    Ok(())
}
