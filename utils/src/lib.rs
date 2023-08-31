use log::info;
use std::env;
use std::time::Instant;

pub struct Measurer {
    now: Instant,
}

impl Default for Measurer {
    fn default() -> Self {
        Self::new()
    }
}

impl Measurer {
    pub fn new() -> Measurer {
        Measurer {
            now: Instant::now(),
        }
    }

    pub fn start(&mut self) {
        self.now = Instant::now();
    }

    pub fn end(&mut self, message: &str) {
        info!("{}, elapsed: {:?}", message, self.now.elapsed());
    }
}

pub fn check_chain_id() -> String {
    let mut chain_id = String::new();
    if let Ok(chain_id_value) = env::var("CHAIN_ID") {
        if chain_id_value.parse::<u32>().is_err() {
            panic!("CHAIN_ID environment variable is not set properly");
        }
        chain_id = chain_id_value;
    }
    if chain_id.is_empty() {
        panic!("CHAIN_ID environment variable is not set");
    }
    chain_id
}
