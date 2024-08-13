fn main() -> std::io::Result<()> {
    if std::env::var("CARGO_FEATURE_TKTRACE").is_ok() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let manifest_path = std::path::Path::new(&manifest_dir);
        let record_rs_path = manifest_path.join("src/tktrace");
        let mut conf = prost_build::Config::new();
        conf.out_dir(record_rs_path);
        conf.compile_protos(&["src/tktrace/format.proto"], &["src/tktrace"])?;
    }
    if std::env::var("CARGO_FEATURE_STDLOG").is_ok() {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let manifest_path = std::path::Path::new(&manifest_dir);
        let record_rs_path = manifest_path.join("src/stdlog");
        let mut conf = prost_build::Config::new();
        conf.out_dir(record_rs_path);
        conf.compile_protos(&["src/stdlog/format.proto"], &["src/stdlog"])?;
    }
    Ok(())
}
