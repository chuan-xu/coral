use crossbeam_channel::bounded;
use crossbeam_channel::Sender;
use log::Level;
use log::Log;

use crate::error::CoralRes;

pub struct Coralog {
    level: Level,
    tx: Sender<Vec<u8>>,
}

impl Coralog {
    pub fn new<W: std::io::Write + Send + 'static>(
        level: Level,
        cap: Option<usize>,
        mut writer: W,
    ) -> CoralRes<Self> {
        // let mut fd = std::fs::OpenOptions::new()
        //     .append(true)
        //     .write(true)
        //     .create(true)
        //     .open(file)?;
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
        Ok(Self { level, tx })
    }
}

impl Log for Coralog {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        println!("======================");
        self.level >= metadata.level()
    }

    fn log(&self, record: &log::Record) {
        // if self.enabled()
        self.tx.send(vec![1, 2, 3]).unwrap();
    }

    fn flush(&self) {}
}
