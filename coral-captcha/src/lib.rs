//! generate captcha

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

fn t() {
    let socket = std::net::UdpSocket::bind("").unwrap();
    // socket.s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
