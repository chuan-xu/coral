use std::fs::read_dir;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use coral_conf::EnvAssignToml;
use rustls::pki_types::CertificateDer;
use rustls::pki_types::PrivateKeyDer;
use rustls::server::WebPkiClientVerifier;
use rustls::ClientConfig;
use rustls::RootCertStore;
use rustls::ServerConfig;
use rustls_pemfile::certs;
use rustls_pemfile::private_key;
use serde::Deserialize;
use webpki_roots::TLS_SERVER_ROOTS;

use coral_macro::EnvAssign;

use crate::error::CoralRes;
use crate::error::Error;

pub static HTTP2_ALPN: [&str; 2] = ["h2", "http/1.1"];
pub static HTTP3_ALPN: [&str; 4] = ["h3-27", "h3-28", "h3-29", "h3"];

#[derive(Deserialize, EnvAssign, Debug)]
pub struct TlsConf {
    ca: Option<String>,
    cert: String,
    key: String,
    alpn: Option<Vec<String>>,
}

impl TlsConf {
    pub fn check(&self) -> CoralRes<()> {
        if let Some(dir) = self.ca.as_ref() {
            if !std::fs::metadata(dir)?.is_dir() {
                return Err(Error::InvalidCa);
            }
        }
        Ok(())
    }

    fn cert_key(&self) -> CoralRes<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
        let mut cert_file = BufReader::new(File::open(&self.cert)?);
        let mut key_file = BufReader::new(File::open(&self.key)?);
        let cert_chain: Vec<CertificateDer<'static>> =
            certs(&mut cert_file).map(|v| v.unwrap()).collect();
        let key_der = private_key(&mut key_file)?.unwrap();
        Ok((cert_chain, key_der))
    }

    pub fn server_conf(&self) -> CoralRes<ServerConfig> {
        let conf_builder = ServerConfig::builder();
        let (cert_chain, key_der) = self.cert_key()?;
        let mut conf = if self.ca.is_some() {
            let root_store = root_ca(self.ca.as_ref())?;
            let client_verifier = WebPkiClientVerifier::builder(Arc::new(root_store)).build()?;
            conf_builder
                .with_client_cert_verifier(client_verifier)
                .with_single_cert(cert_chain, key_der)?
        } else {
            conf_builder
                .with_no_client_auth()
                .with_single_cert(cert_chain, key_der)?
        };
        if let Some(alpn) = self.alpn.as_ref() {
            conf.alpn_protocols = alpn.iter().map(|v| v.as_bytes().to_vec()).collect();
        }
        Ok(conf)
    }

    pub fn client_conf(&self) -> CoralRes<ClientConfig> {
        let root_store = root_ca(self.ca.as_ref())?;
        let (cert_chain, key_der) = self.cert_key()?;
        let mut conf = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(cert_chain, key_der)?;
        conf.enable_early_data = true;
        if let Some(alpn) = self.alpn.as_ref() {
            conf.alpn_protocols = alpn.iter().map(|v| v.as_bytes().to_vec()).collect();
        }
        Ok(conf)
    }
}

// pub fn new(tls_ca: Option<String>, tls_cert: String, tls_key: String) -> Self {
//     Self {
//         tls_ca,
//         tls_cert,
//         tls_key,
//     }
// }

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
