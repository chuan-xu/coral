use error::CoralRes;

mod cli;
mod error;
mod hand;
mod io;

fn main() -> CoralRes<()> {
    io::run()?;
    Ok(())
}
