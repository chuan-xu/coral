use rustls::{
    pki_types::CertificateDer,
    server::{danger::ClientCertVerifier, WebPkiClientVerifier},
    RootCertStore, ServerConfig,
};
use rustls_pemfile::{certs, private_key};
use std::{
    fs::{read_dir, File},
    io::BufReader,
    path::{Path, PathBuf},
    sync::Arc,
};
use webpki_roots::TLS_SERVER_ROOTS;

use crate::{cli::Cli, error::CoralRes};

/// 根证书
fn root_ca(ca_dir: Option<&String>) -> CoralRes<Arc<dyn ClientCertVerifier>> {
    let mut root_store = RootCertStore {
        roots: TLS_SERVER_ROOTS.iter().cloned().collect(),
    };
    if let Some(dir) = ca_dir {
        let certs_path = Path::new(dir).to_path_buf();
        client_cert(certs_path, &mut root_store)?;
    }
    Ok(WebPkiClientVerifier::builder(Arc::new(root_store)).build()?)
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
pub fn server_conf(cli: &Cli) -> CoralRes<Arc<ServerConfig>> {
    let client_verifier = root_ca(cli.ca_dir.as_ref())?;
    let mut cert_file = BufReader::new(File::open(&cli.certificate)?);
    let mut key_file = BufReader::new(File::open(&cli.private_key)?);
    let cert_chain: Vec<CertificateDer<'static>> =
        certs(&mut cert_file).map(|v| v.unwrap()).collect();
    let key_der = private_key(&mut key_file)?.unwrap();
    let mut conf = ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(cert_chain, key_der)
        .unwrap();
    conf.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(conf))
}
