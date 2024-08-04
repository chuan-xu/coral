mod cli;
mod error;
mod hand;
mod io;
mod tls;
mod util;

use error::CoralRes;

fn main() -> CoralRes<()> {
    io::run()?;
    Ok(())
}
