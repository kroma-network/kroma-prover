use log::info;
use std::time::Instant;

pub struct Measurer {
    now: Instant,
}

impl Measurer {
    pub fn new() -> Measurer {
        return Measurer {
            now: Instant::now(),
        };
    }

    pub fn start(&mut self) {
        self.now = Instant::now();
    }

    pub fn end(&mut self, message: &str) {
        info!("{}, elapsed: {:?}", message, self.now.elapsed());
    }
}
