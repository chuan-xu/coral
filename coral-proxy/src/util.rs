#![allow(unused)]
use regex::Regex;
use std::sync::LazyLock;

// static DOT_DECIMAL_RE: Regex = Regex::new(r"^((25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])\.){3}(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9]):([0-9]{1,5})$").unwrap();

static DOT_DECIMAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
r"^((25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9])\.){3}(25[0-5]|2[0-4][0-9]|1[0-9]{2}|[1-9]?[0-9]):([0-9]{1,5})$"
    ).unwrap()
});

pub fn is_valid_ipv4_with_port(addr: &str) -> bool {
    DOT_DECIMAL_RE.is_match(addr)
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
