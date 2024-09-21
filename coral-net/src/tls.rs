use std::fs::read_dir;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use clap::Args;
use rustls::pki_types::CertificateDer;
use rustls::server::WebPkiClientVerifier;
use rustls::ClientConfig;
use rustls::RootCertStore;
use rustls::ServerConfig;
use rustls_pemfile::certs;
use rustls_pemfile::private_key;
use serde::Deserialize;
use webpki_roots::TLS_SERVER_ROOTS;

use crate::error::CoralRes;
use crate::error::Error;

#[derive(Args, Debug)]
pub struct TlsParam {
    #[arg(long, help = "ca directory")]
    pub tls_ca: Option<String>,

    #[arg(long, help = "server/client certificate")]
    pub tls_cert: String,

    #[arg(long, help = "server/client private")]
    pub tls_key: String,
}

#[allow(unused)]
#[derive(Deserialize, Debug)]
pub struct TlsConf {
    ca_path: Option<String>,
    ca_files: Option<Vec<String>>,
    cert: String,
    key: String,
}

impl TlsConf {
    pub fn server_conf(&self) {}
}

impl TlsParam {
    pub fn check(&self) -> CoralRes<()> {
        if let Some(dir) = self.tls_ca.as_ref() {
            if !std::fs::metadata(dir)?.is_dir() {
                return Err(Error::InvalidCa);
            }
        }
        Ok(())
    }

    pub fn new(tls_ca: Option<String>, tls_cert: String, tls_key: String) -> Self {
        Self {
            tls_ca,
            tls_cert,
            tls_key,
        }
    }
}

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

/// tls server config
pub fn server_conf(param: &TlsParam) -> CoralRes<ServerConfig> {
    let root_store = root_ca(param.tls_ca.as_ref())?;
    let client_verifier = WebPkiClientVerifier::builder(Arc::new(root_store)).build()?;
    let mut cert_file = BufReader::new(File::open(&param.tls_cert)?);
    let mut key_file = BufReader::new(File::open(&param.tls_key)?);
    let cert_chain: Vec<CertificateDer<'static>> =
        certs(&mut cert_file).map(|v| v.unwrap()).collect();
    let key_der = private_key(&mut key_file)?.unwrap();
    let mut conf = ServerConfig::builder()
        // .with_client_cert_verifier(client_verifier)
        .with_no_client_auth()
        .with_single_cert(cert_chain, key_der)?;
    conf.alpn_protocols = vec![b"h3".to_vec(), b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(conf)
}

/// tls client config
pub fn client_conf(param: &TlsParam) -> CoralRes<ClientConfig> {
    let root_store = root_ca(param.tls_ca.as_ref())?;
    let mut cert_file = BufReader::new(File::open(&param.tls_cert)?);
    let mut key_file = BufReader::new(File::open(&param.tls_key)?);
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
