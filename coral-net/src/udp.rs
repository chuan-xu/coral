use rustls::ServerConfig;

struct UdpServer {
    listen_addr: std::net::SocketAddr,
    tls_cfg: ServerConfig,
}

struct H3 {}

struct H3Stream {}
