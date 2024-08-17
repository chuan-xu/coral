mod cli;
mod error;
mod http;
mod io;
mod tls;
mod util;
mod ws;

use error::CoralRes;

fn main() -> CoralRes<()> {
    io::run()?;
    Ok(())
}
