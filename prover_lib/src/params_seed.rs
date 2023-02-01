use halo2_proofs::halo2curves::bn256::Bn256;
use halo2_proofs::poly::kzg::commitment::ParamsKZG;
use rand_xorshift::XorShiftRng;

#[derive(Debug, Clone)]
pub struct ParamsSeed {
    pub params: ParamsKZG<Bn256>,
    pub agg_params: ParamsKZG<Bn256>,
    pub seed: [u8; 16],
    pub rng: XorShiftRng,
}
