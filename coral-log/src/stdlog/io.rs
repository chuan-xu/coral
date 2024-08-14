use std::marker::PhantomData;

use crossbeam_channel::bounded;
use crossbeam_channel::Sender;
use log::Level;
use log::Log;

use crate::error::CoralRes;

pub struct Coralog<C> {
    level: Level,
    tx: Sender<Vec<u8>>,
    _pd: PhantomData<C>,
}

impl<C> Coralog<C> {
    pub fn new<W: std::io::Write + Send + 'static>(
        level: Level,
        cap: Option<usize>,
        mut writer: W,
    ) -> CoralRes<Self> {
        let cap = match cap {
            Some(c) => c,
            None => 4096,
        };
        let (tx, rx) = bounded::<Vec<u8>>(cap);
        std::thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(chunk) => {
                        if let Err(e) = writer.write(&chunk) {
                            eprintln!("failed to writer log file {:?}", e);
                        } else if let Err(e) = writer.flush() {
                            eprintln!("failed to flush {:?}", e);
                        }
                    }
                    Err(e) => eprint!("failed to recv from channel {:?}", e),
                }
            }
        });
        Ok(Self {
            level,
            tx,
            _pd: PhantomData,
        })
    }
}

pub trait Convert {
    fn to_bytes(&mut self, record: &log::Record) -> CoralRes<Vec<u8>>;
}

#[derive(Default)]
pub struct Stdout;

impl Convert for Stdout {
    fn to_bytes(&mut self, record: &log::Record) -> CoralRes<Vec<u8>> {
        let current = std::thread::current();
        let time = chrono::Local::now().to_rfc3339();
        let res = format!("{}: [{}]: {:?}", time, current.name().unwrap_or(""), record);
        Ok(res.as_bytes().to_vec())
    }
}

impl<C> Log for Coralog<C>
where C: Convert + Default + Send + Sync
{
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.level >= metadata.level()
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let mut c = C::default();
            match c.to_bytes(record) {
                Ok(data) => {
                    if let Err(e) = self.tx.send(data) {
                        eprintln!("failed to send log to write {:?}", e);
                    }
                }
                Err(e) => eprintln!("failed to convert log to bytes {:?}", e),
            }
        }
    }

    fn flush(&self) {}
}
