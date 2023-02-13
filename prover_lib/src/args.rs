use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Get params and write into file.
    #[clap(short, long = "params")]
    pub params_path: Option<String>,
    /// Get seed and write into file.
    #[clap(long = "seed")]
    pub seed_path: Option<String>,
    /// Get verify circuit verifying key.
    #[clap(long = "vkey")]
    pub vkey_path: Option<String>,
    /// Get BlockTrace from file or dir.
    #[clap(short, long = "trace")]
    pub race_path: Option<String>,
    /// Option means if generates evm proof.
    /// Boolean means if output evm proof.
    #[clap(long = "evm")]
    pub evm_proof: Option<bool>,
    /// Option means if generates state proof.
    /// Boolean means if output state proof.
    #[clap(long = "state")]
    pub state_proof: Option<bool>,
    /// Option means if generates agg proof.
    /// Boolean means if output agg proof.
    #[clap(long = "agg")]
    pub agg_proof: Option<bool>,
}
