use std::fs::read_dir;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use rustls::pki_types::CertificateDer;
use rustls::server::WebPkiClientVerifier;
use rustls::ClientConfig;
use rustls::RootCertStore;
use rustls::ServerConfig;
use rustls_pemfile::certs;
use rustls_pemfile::private_key;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::cli::CommParam;
use crate::error::CoralRes;

/// 根证书
fn root_ca(ca_dir: Option<&String>) -> CoralRes<RootCertStore> {
    let mut root_store = RootCertStore {
        roots: TLS_SERVER_ROOTS.iter().cloned().collect(),
    };
    if let Some(dir) = ca_dir {
        let certs_path = Path::new(dir).to_path_buf();
        client_cert(certs_path, &mut root_store)?;
    }
    Ok(root_store)
}

/// 添加用于校验client请求的根证书
fn client_cert(ca_path: PathBuf, root_store: &mut RootCertStore) -> std::io::Result<()> {
    if ca_path.is_file() {
        let fd = File::open(ca_path)?;
        let mut buf = BufReader::new(fd);
        let cert = certs(&mut buf).map(|v| v.unwrap());
        root_store.add_parsable_certificates(cert);
    } else if ca_path.is_dir() {
        for entry in read_dir(ca_path)? {
            let entry = entry?;
            client_cert(entry.path(), root_store)?;
        }
    }
    Ok(())
}

/// tls server 配置
pub fn server_conf(param: &CommParam) -> CoralRes<ServerConfig> {
    let root_store = root_ca(param.ca_dir.as_ref())?;
    let client_verifier = WebPkiClientVerifier::builder(Arc::new(root_store)).build()?;
    let mut cert_file = BufReader::new(File::open(&param.certificate)?);
    let mut key_file = BufReader::new(File::open(&param.private_key)?);
    let cert_chain: Vec<CertificateDer<'static>> =
        certs(&mut cert_file).map(|v| v.unwrap()).collect();
    let key_der = private_key(&mut key_file)?.unwrap();
    let mut conf = ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(cert_chain, key_der)?;
    conf.alpn_protocols = vec![b"h3".to_vec(), b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(conf)
}

pub fn client_conf(param: &CommParam) -> CoralRes<ClientConfig> {
    let root_store = root_ca(param.ca_dir.as_ref())?;
    let mut cert_file = BufReader::new(File::open(&param.certificate)?);
    let mut key_file = BufReader::new(File::open(&param.private_key)?);
    let cert_chain: Vec<CertificateDer<'static>> =
        certs(&mut cert_file).map(|v| v.unwrap()).collect();
    let key_der = private_key(&mut key_file)?.unwrap();
    let mut conf = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_client_auth_cert(cert_chain, key_der)?;
    conf.enable_early_data = true;
    conf.alpn_protocols = vec![b"h3".to_vec(), b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(conf)
}
