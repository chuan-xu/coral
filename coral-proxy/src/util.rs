#![allow(unused)]
use std::str::FromStr;
use std::sync::LazyLock;

use axum::http::uri::PathAndQuery;
use hyper::Uri;
use log::error;
use regex::Regex;

use crate::error::CoralRes;
use crate::error::Error;

pub static HTTP_RESET_URI: &'static str = "/reset_http";
pub static WS_RESET_URI: &'static str = "/reset_ws";

// static DOT_DECIMAL_RE: Regex =
// Regex::new(r"^((25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])\.
// ){3}(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9]):([0-9]{1,5})$").unwrap();

static DOT_DECIMAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
r"^((25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])\.){3}(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9]):([0-9]{1,5})$"
    ).unwrap()
});

pub fn is_valid_ipv4_with_port(addr: &str) -> bool {
    DOT_DECIMAL_RE.is_match(addr)
}

pub fn modify_path_uri(uri: &Uri, mod_path: &str) -> CoralRes<Uri> {
    let authority = uri
        .authority()
        .ok_or_else(|| {
            error!("uri.authority is none");
            Error::NoneOption("uri.authority")
        })?
        .as_str();
    if let Some(scheme_str) = uri.scheme_str() {
        let mut scheme = scheme_str.to_string();
        scheme += "://";
        scheme += authority;
        scheme += mod_path;
        let nuri = hyper::Uri::try_from(scheme).map_err(|err| {
            error!(
                e = err.to_string(),
                scheme = scheme_str,
                authority = authority;
                "failed to parse scheme"
            );
            err
        })?;
        Ok(nuri)
    } else {
        Ok(hyper::Uri::from_str(mod_path)?)
    }
}

pub fn get_modify_path_url<'a>(uri: &'a Uri, mod_path: &str) -> CoralRes<(&'a PathAndQuery, Uri)> {
    let path = uri.path_and_query().ok_or_else(|| {
        error!("uri.path_and_query is none");
        Error::NoneOption("uri.path_and_query")
    })?;
    let mod_path = modify_path_uri(uri, mod_path)?;
    Ok((path, mod_path))
}

#[cfg(test)]
mod test {
    use super::is_valid_ipv4_with_port;

    #[test]
    fn test_is_valid_ipv4_with_port() {
        let valid_addrs = vec!["1.1.1.1:9001", "127.0.0.1:9999", "0.0.0.0:9212"];
        let invalid_addrs = vec![
            "asdasd.asd.asd.123",
            "qq.ww.ee.rr",
            "192.168.1.1",
            "127.0.0.1:9001/",
        ];
        for addr in valid_addrs {
            assert!(is_valid_ipv4_with_port(addr));
        }
        for addr in invalid_addrs {
            assert!(!is_valid_ipv4_with_port(addr));
        }
    }
}
