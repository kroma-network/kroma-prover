use log::info;
use std::fmt::Display;
use std::path::Path;
use std::{fs, io::Write, path::PathBuf};
use zkevm::{
    io::write_file,
    prover::{AggCircuitProof, Prover, TargetCircuitProof},
};

use crate::prover_lib::ProofType;

pub static ERROR_MSG_HEADER: &str = "KROMA";

pub fn kroma_msg<S: AsRef<str> + Display>(msg: S) -> String {
    format!("[{ERROR_MSG_HEADER}] {msg}")
}

pub fn kroma_info<S: AsRef<str> + Display>(msg: S) {
    info!("{}", kroma_msg(msg))
}

pub fn write_target_proof(dir: &Path, proof: TargetCircuitProof, proof_type: ProofType) {
    let proof_path = dir.join(proof_type.to_string() + ".proof");
    let mut f = fs::File::create(proof_path).unwrap();
    f.write_all(proof.proof.as_slice()).unwrap();
}

pub fn write_agg_proof(dir: &Path, proof: &AggCircuitProof) {
    let mut proof_path = dir.join("agg.proof");
    let _ = fs::create_dir_all(&proof_path);

    proof.write_to_dir(&mut proof_path);
}

pub fn write_solidity(
    prover: &Prover,
    proof: &AggCircuitProof,
    dir: &PathBuf,
    verifier_name: &str,
) {
    let sol = prover.create_solidity_verifier(proof);
    let _ = fs::create_dir_all(dir);

    let mut dir = dir.clone();
    write_file(&mut dir, verifier_name, &Vec::<u8>::from(sol.as_bytes()));
}
