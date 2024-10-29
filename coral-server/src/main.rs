use error::CoralRes;

mod cli;
mod error;
mod hand;
mod io;
mod midware;
mod payload;
mod user;
use log::error;
fn main() -> CoralRes<()> {
    let conf = cli::Cli::init()?;
    let rt = conf.rt_conf.runtime("coral_server")?;
    let router = hand::router(&conf);
    let app = conf.app(router)?;
    if let Err(err) = rt.block_on(app.run()) {
        error!(e = format!("{:?}", err); "block on server");
    }
    Ok(())
}
