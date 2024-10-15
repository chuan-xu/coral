use error::CoralRes;

mod cli;
mod error;
mod hand;
mod io;
use log::error;
fn main() -> CoralRes<()> {
    let conf = cli::Cli::init()?;
    let rt = conf.rt_conf.runtime("coral_server")?;
    let app = hand::app(&conf);
    if let Err(err) = rt.block_on(conf.serve(app)) {
        error!(e = format!("{:?}", err); "block on server");
    }
    Ok(())
}
