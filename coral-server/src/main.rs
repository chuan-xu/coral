use error::CoralRes;

mod cli;
mod error;
mod hand;
mod io;
mod midw;

fn main() -> CoralRes<()> {
    io::run()?;
    Ok(())
}
