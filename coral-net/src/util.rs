use std::str::FromStr;

use crate::error::{CoralRes, Error};
use axum::http::uri::PathAndQuery;
use hyper::Uri;
use log::error;

pub fn reset_uri_path(uri: &Uri, mod_path: &str) -> CoralRes<Uri> {
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
                e = format!("{:?}", err),
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
    let mod_path = reset_uri_path(uri, mod_path)?;
    Ok((path, mod_path))
}
