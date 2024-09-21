use coral_net::tls::TlsConf;
use toml::from_str;

fn demo_toml_conf() -> &'static str {
    r#"
        ca_path = "/root/cert/ca"
        ca_files = ["/root/other/ca1.pem", "/root/other/ca2.pem"]
        cert = "/root/cert/cert.pem"
        key = "/root/cert/key.pem"
    "#
}

#[test]
fn test_tls_conf() {
    let conf: TlsConf = from_str(demo_toml_conf()).unwrap();
    println!("{:?}", conf);
}
