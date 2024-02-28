use clap::Parser;
use std::fs;
use zkevm::{
    circuit::{AGG_DEGREE, DEGREE},
    utils::create_kzg_params_to_file,
};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Specify directory which params have stored in. (default: ./kzg_params)
    #[clap(short, long = "params_dir")]
    params_dir: Option<String>,
    /// Specify domain size. (generate 2 params with DEGREE/AGG_DEGREE if it is ommitted.)
    #[clap(short, long = "n")]
    n: Option<usize>,
}

impl Args {
    fn get_prarams_dir(&self) -> String {
        match &self.params_dir {
            Some(dir) => dir.clone(),
            None => {
                let dir = String::from("kzg_params");
                fs::create_dir_all(&dir).unwrap();
                dir
            }
        }
    }
}

fn main() {
    dotenv::dotenv().ok();
    env_logger::init();

    let args = Args::parse();
    let params_dir = args.get_prarams_dir();
    match args.n {
        Some(n) => {
            if n > 30 {
                panic!("too big domain size, you should enter `n` less than 30");
            }
            let _ = create_kzg_params_to_file(&params_dir, n);
        }
        None => {
            let _ = create_kzg_params_to_file(&params_dir, *DEGREE);
            let _ = create_kzg_params_to_file(&params_dir, *AGG_DEGREE);
        }
    }
}
