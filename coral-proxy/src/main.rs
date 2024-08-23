mod cli;
mod error;
mod http;
mod io;
mod util;
mod ws;

use error::CoralRes;

fn main() -> CoralRes<()> {
    io::run()?;
    Ok(())
}

#[cfg(test)]
mod test {
    use coral_runtime::tokio;

    async fn client() {}

    #[test]
    fn http3_client() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(client());
    }
}
