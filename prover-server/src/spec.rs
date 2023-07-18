use enum_iterator::all;
use enum_iterator::Sequence;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::fmt::Display;
use zkevm::circuit::{AGG_DEGREE, CHAIN_ID, DEGREE, MAX_CALLDATA, MAX_TXS};

#[derive(Debug, Sequence, Serialize, Deserialize)]
pub enum ProofType {
    None,
    Evm,
    State,
    Super,
    Agg,
}

impl Display for ProofType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProofType::Evm => write!(f, "evm"),
            ProofType::State => write!(f, "state"),
            ProofType::Super => write!(f, "super"),
            ProofType::Agg => write!(f, "agg"),
            ProofType::None => write!(f, "none"),
        }
    }
}

impl ProofType {
    /// select the enum by a value.
    pub fn from_value(val: i32) -> Self {
        match val {
            1 => ProofType::Evm,
            2 => ProofType::State,
            3 => ProofType::Super,
            4 => ProofType::Agg,
            _ => ProofType::None,
        }
    }

    /// extract value related to the enum.
    pub fn to_value(&self) -> i32 {
        match self {
            ProofType::Evm => 1,
            ProofType::State => 2,
            ProofType::Super => 3,
            ProofType::Agg => 4,
            ProofType::None => 0,
        }
    }

    /// returns enum-value mapping as a String.
    pub fn desc() -> String {
        let mut mapping = HashMap::new();
        let proof_type_vec = all::<ProofType>().collect::<Vec<_>>();
        for i in proof_type_vec {
            let key = i.to_string();
            let value = i.to_value();
            mapping.insert(key, value);
        }
        serde_json::to_string(&mapping).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ZkSpec {
    pub proof_type_desc: String,
    pub degree: u32,
    pub agg_degree: u32,
    pub chain_id: u32,
    pub max_txs: u32,
    pub max_call_data: u32,
}

impl Default for ZkSpec {
    fn default() -> Self {
        Self {
            proof_type_desc: ProofType::desc(),
            degree: *DEGREE as u32,
            agg_degree: *AGG_DEGREE as u32,
            chain_id: *CHAIN_ID as u32,
            max_txs: MAX_TXS as u32,
            max_call_data: MAX_CALLDATA as u32,
        }
    }
}

impl ZkSpec {
    pub fn new(chain_id: u32) -> Self {
        Self {
            proof_type_desc: ProofType::desc(),
            degree: *DEGREE as u32,
            agg_degree: *AGG_DEGREE as u32,
            chain_id,
            max_txs: MAX_TXS as u32,
            max_call_data: MAX_CALLDATA as u32,
        }
    }
}
