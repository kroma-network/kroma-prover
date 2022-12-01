use eth_types::Word;
use ethers_core::types::{Bytes, H256, U256, U64};
use std::{
    convert::TryFrom,
    io::{Error, ErrorKind, Read},
};

#[derive(Debug, Default, Clone)]
pub struct AccountData {
    pub nonce: u64,
    pub balance: U256,
    pub code_hash: H256,
}

pub trait CanRead: Sized {
    fn try_parse(rd: impl Read) -> Result<Self, Error>;
}

impl CanRead for AccountData {
    fn try_parse(mut rd: impl Read) -> Result<Self, Error> {
        let mut uint_buf = [0; 4];
        rd.read_exact(&mut uint_buf)?;
        // check it is 0x04040000
        if uint_buf != [4, 4, 0, 0] {
            return Err(Error::new(ErrorKind::Other, "unexpected flags"));
        }

        let mut byte32_buf = [0; 32];
        rd.read_exact(&mut byte32_buf)?; //nonce
        let nonce = U64::from_big_endian(&byte32_buf[24..]);
        rd.read_exact(&mut byte32_buf)?; //balance
        let balance = U256::from_big_endian(&byte32_buf);
        rd.read_exact(&mut byte32_buf)?; //codehash
        let code_hash = H256::from(&byte32_buf);
        //rd.read_exact(&mut hash_buf)?; //storage root, not need yet

        Ok(AccountData {
            nonce: nonce.as_u64(),
            balance,
            code_hash,
        })
    }
}

#[derive(Debug, Default, Clone)]
pub struct StorageData(Word);

impl AsRef<Word> for StorageData {
    fn as_ref(&self) -> &Word {
        &self.0
    }
}

impl CanRead for StorageData {
    fn try_parse(mut rd: impl Read) -> Result<Self, Error> {
        let mut uint_buf = [0; 4];
        rd.read_exact(&mut uint_buf)?;
        // check it is 0x01010000
        if uint_buf != [1, 1, 0, 0] {
            return Err(Error::new(ErrorKind::Other, "unexpected flags"));
        }
        let mut byte32_buf = [0; 32];
        rd.read_exact(&mut byte32_buf)?;
        Ok(StorageData(Word::from(byte32_buf)))
    }
}

#[derive(Debug, Default, Clone)]
pub struct TrieProof<T> {
    pub data: T,
    pub key: Option<H256>,
}

fn deserialize_trie_leaf<R: Read, T: CanRead>(mut rd: R) -> Result<(H256, T), Error> {
    let mut byte32_buf = [0; 32];
    rd.read_exact(&mut byte32_buf)?;
    let key = H256::from(byte32_buf);
    Ok((key, T::try_parse(rd)?))
}

pub type AccountProof = TrieProof<AccountData>;
pub type StorageProof = TrieProof<StorageData>;

impl<T: CanRead + Default> TryFrom<&[Bytes]> for TrieProof<T> {
    type Error = Error;

    fn try_from(src: &[Bytes]) -> Result<Self, Self::Error> {
        for data in src {
            let mut rd = data.as_ref();
            let mut prefix = [0; 1];
            rd.read_exact(&mut prefix)?;
            match prefix[0] {
                1 => {
                    let (key, data) = deserialize_trie_leaf(rd)?;
                    return Ok(Self {
                        key: Some(key),
                        data,
                    });
                }
                2 => {
                    // empty node
                    return Ok(Default::default());
                }
                _ => (),
            }
        }

        Err(Error::new(ErrorKind::UnexpectedEof, "no leaf key found"))
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use types::eth::StorageTrace;
    #[test]
    fn deserialize_example1() {
        let example = r#"
        {
            "rootBefore": "0x138e9434a520607da9e07013cdb82c3f832191d3bc2cd1b271b2ed9fa7a6a554",
            "rootAfter": "0x24244ef5c8a7829b8c80d7c2dc7f32ef9b087541444ab5ea2b4f6f00557361a7",
            "proofs": {
                "0x0000000000000000000000000000000000000000": [
                    "0x00b50fa7ebcfbf879d2c87c30fa8da23205fec4876c05200c0211e27a330e9ca16444758a273fc0cfb23366a7a377630f0427fe495c9f78efbed9dc47a1e3f9e0e",
                    "0x009f673aa1dbc844b4d6ec60b57e617ede40d603158e71c02b94fed4127bfa5c02c913cad65a874081a2f409ab839f7bf87ac50ba44e76c7d99a538806fb379e04",
                    "0x008b4c8f5f6840f4a1366d6700d8f4ea9c4f331afd0232cca6f4c9437832269027166f08f09d830e962a2675968f7e75f95dd4953c750800ae11fc646e73e64722",
                    "0x013c6eff766107f2db0c4bf0ead086d4befa5d8675dcf54c50073efc389830fb060404000000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000cc0a77f6e063b4b62eb7d9ed6f427cf687d8d0071d751850cfe5d136bc60d3ab12b6a6aca3814be9efdc6b17540f6e7bb06457e9102149aba975e2210fd3617a00",
                    "0x5448495320495320534f4d45204d4147494320425954455320464f5220534d54206d3172525867503278704449"
                ],
                "0x4cb1aB63aF5D8931Ce09673EbD8ae2ce16fD6571": [
                    "0x00b50fa7ebcfbf879d2c87c30fa8da23205fec4876c05200c0211e27a330e9ca16444758a273fc0cfb23366a7a377630f0427fe495c9f78efbed9dc47a1e3f9e0e",
                    "0x0076a4691e1917894350a3b6b4d10c44aa94316e09692a37c678764b415de1021a537a6fac1254f68629734e6263e4d41e8d6ba08e0eba4e2317bad3b469159907",
                    "0x00ae72888fea2b0ee021bee3ae2e80f0a50b87a5a7966e98b29aa8770b7f485605a3d8fb602901f3cd49e1260c12d40e98607bd279b30164c708d668126345c829",
                    "0x00fbbed4358df764ff3a263c66df07b445abeaaf4ab50bddc2fd643f3512e02a1b6d8a9b9874fbccdb5b7890a8e46e5c5b987819a86cc4b9a642786181f62dda1f",
                    "0x000000000000000000000000000000000000000000000000000000000000000000e2591e8c149131c6df1ac04f9c0f54ff8db991eb80dff52b4b218cac31c1430b",
                    "0x0078c7b59d789c294f21339f0a872b81a418d6b24273fe959dc00126520917970dc63dba5cdc4a7aa1f0e97f4355d9caac51821d67463fa58cb89ffc53c1fe9f27",
                    "0x000000000000000000000000000000000000000000000000000000000000000000e6c0fdae5b43dd0b7edfd21803e8816e191138071fd2e1f7e42d92c5c19a592b",
                    "0x008ebbbd4b7d5ddd66149c1b9feeeaec862e8e489cb8c50409aaaf00de33310721a72c67edca1db779b38140aaee9baf382c96315f0884909da4a4a7480f3ab82d",
                    "0x017581e431a68d0fa641e14a7d29a6c2b150db6da1d13f59dee6f7f492a0bebd2904040000000000000000000000000000000000000000000000000000000000000000001b0056bc75e2d630fffffffffffffffffffffffffffffffffffff7a8e726dd7f67c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470000000000000000000000000000000000000000000000000000000000000000000",
                    "0x5448495320495320534f4d45204d4147494320425954455320464f5220534d54206d3172525867503278704449"
                ],
                "0xe8D466681784504A8458d4EF34F141adaDA678Fe": [
                    "0x00b50fa7ebcfbf879d2c87c30fa8da23205fec4876c05200c0211e27a330e9ca16444758a273fc0cfb23366a7a377630f0427fe495c9f78efbed9dc47a1e3f9e0e",
                    "0x0076a4691e1917894350a3b6b4d10c44aa94316e09692a37c678764b415de1021a537a6fac1254f68629734e6263e4d41e8d6ba08e0eba4e2317bad3b469159907",
                    "0x00ae72888fea2b0ee021bee3ae2e80f0a50b87a5a7966e98b29aa8770b7f485605a3d8fb602901f3cd49e1260c12d40e98607bd279b30164c708d668126345c829",
                    "0x00fbbed4358df764ff3a263c66df07b445abeaaf4ab50bddc2fd643f3512e02a1b6d8a9b9874fbccdb5b7890a8e46e5c5b987819a86cc4b9a642786181f62dda1f",
                    "0x016d3e389f7dd8c147fe168ec3dfa575f588d5caee7bd4da9fd99c7ecf9cc5df000404000000000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000000178763dea206ad5ecfbf211ddeb69d930d18811bc617cb4bbb0c0e7f0d28a3aa2739429413c40c987071b484a92400ccce811ec9e5202885b87d6ed24f27e3ca00",
                    "0x5448495320495320534f4d45204d4147494320425954455320464f5220534d54206d3172525867503278704449"
                ]
            },
            "storageProofs": {
                "0xe8D466681784504A8458d4EF34F141adaDA678Fe": {
                    "0x5a158573daab1c353835da34297290f5f813859e4bb52de641691b875502523f": [
                        "0x00be85171617341aa3277ff987f889ac613465f94f1ce1f88c1cade46090fbd411fd876ccea893e2a915742ccb261abedf8ccc01079f341586b724c103dd1adb17",
                        "0x015bcfa567f724b471f0711c5b1b5295f4babeca2d70ed85e33abf5f8086e7721a0101000000000000000000000000000000000000000000007f228daac38a51833dbbf83000",
                        "0x5448495320495320534f4d45204d4147494320425954455320464f5220534d54206d3172525867503278704449"
                    ],
                    "0x977b86d8b2c12cb1b0cf5c34210e07337f1ed424f3f38ee3bddb639468b3095f": [
                        "0x00be85171617341aa3277ff987f889ac613465f94f1ce1f88c1cade46090fbd411fd876ccea893e2a915742ccb261abedf8ccc01079f341586b724c103dd1adb17",
                        "0x00f7f26bfedc1c3c30c68d11e0cb3f7d434e6235eb8c88637f9dc7ffa68aa9280bf855e9ec031301edaf7f84f64a9dc5798916c8678a1f907597da2c3bf8b63a0e",
                        "0x0057a298c09fb1f9609b74ff09c68470bf41a983d24995a2e05a1fb9546ce3050df3311fe9cdd331d512ec3a45956f00ff6f2bfde8fffbbadf9b8500c230b7b705",
                        "0x008f2fa3897bc04514e3935443ef67a70c0ebe0dd85364f0202ebb79bd910c5a250000000000000000000000000000000000000000000000000000000000000000",
                        "0x0052fb41bda5330046b2f736cfd86106e5b0aadc56e770f870958d7c15334f96090266f9b3b99373c76aabe6fd12667ea462892cac46ef50e325de902b437a7a20",
                        "0x0134aedb4be7574a842f0fde19fc74dcf1a0369b31b42311b841f80bbc74556d230101000000000000000000000000000000000000000000000000000000000000000007d000",
                        "0x5448495320495320534f4d45204d4147494320425954455320464f5220534d54206d3172525867503278704449"
                    ]
                }
            }
        }
        "#;

        let s_trace: StorageTrace = serde_json::from_str(example).unwrap();
        let proofs = s_trace.proofs.as_ref().unwrap();
        for (_, proof) in proofs.iter() {
            let proof: AccountProof = proof.as_slice().try_into().unwrap();
            println!("proof: {:?}", proof);
        }

        for (_, s_map) in s_trace.storage_proofs.iter() {
            for (k, val) in s_map {
                let val_proof: StorageProof = val.as_slice().try_into().unwrap();
                println!("k: {}, v: {:?}", k, val_proof);
            }
        }
    }

    #[test]
    fn deserialize_example2() {
        let example = r#"
        {
            "rootBefore": "0x21d23ec063cd5e5049ce308c85579851b3fc00fa16288c74e03154a759b060be",
            "rootAfter": "0x01298a670d2df71e85631288e106bbfc1cd75a24293c23b052350058a18745ad",
            "proofs": {
                "0x05fDbDfaE180345C6Cff5316c286727CF1a43327": [
                    "0x001e2f9f788fddbe60528860cd94401896e598ff7ec689129b64b38c6f6e5d7cd00e831da32eb5166892a00be68f2ac1fe7ed65e19e0d5e4fc9220c5525af57b47",
                    "0x002ee00d41c4efcec95739e672b3c2c054103f03bf6b376a51ff27c66fd1600c972e8d75732548f8a07026d2dbb807f7844caac67aa88a9c4be518f26779d3eee0",
                    "0x002ab9b4cbc77f170bac3dbc2c9324063d4baa50f57c15819edaad95a16e8166af13d8fd8ebedae18d8dcf924c604b6771e9bbcb08c0ddb2e22473ef9541c41ba2",
                    "0x012512bcd4ae09e58018baa6cdfddc75ac00133901daabcdb14de10e2d25286cfb04040000000000000000000000000000000000000000000000000000000000000000000800000000000000000000000000000000000000000000000000000000000000002adda4f0d2f9538b1c95d876e5ecb0bb0bcd847efab535259dff10a6f58aa8db2a76acf8180ab9c36b231ae111a8ac21cd34c5133fcd1c403fa893f1f31471c300",
                    "0x5448495320495320534f4d45204d4147494320425954455320464f5220534d54206d3172525867503278704449"
                ],
                "0x4cb1aB63aF5D8931Ce09673EbD8ae2ce16fD6571": [
                    "0x001e2f9f788fddbe60528860cd94401896e598ff7ec689129b64b38c6f6e5d7cd00e831da32eb5166892a00be68f2ac1fe7ed65e19e0d5e4fc9220c5525af57b47",
                    "0x002ee00d41c4efcec95739e672b3c2c054103f03bf6b376a51ff27c66fd1600c972e8d75732548f8a07026d2dbb807f7844caac67aa88a9c4be518f26779d3eee0",
                    "0x002289e22abe96b0811a18c472ed2151193155e06dcb894f1cc0e415a22070e1f60511178241b5d9745ede181272654009625d78f4888c06633049bcbab1108578",
                    "0x000186c8ed5ea22968ac76644a1d293ff710a5a760206aad0af0d55b73e89f31452007487a1885185444169e80b08beb9a0e7875498d711f7e17d08bda1175f20c",
                    "0x000000000000000000000000000000000000000000000000000000000000000000050d40d784982a6c8c4f032301c362969c25ae9c7b1c27f072f4a943f7798d15",
                    "0x002248f47a53d7e981b187961b71a36033d9ef59273228ec127666f43c5d0fd2c503371eca2ecb6391c55284c1347145909bfcf00264117c6964ad841e6e946357",
                    "0x0000000000000000000000000000000000000000000000000000000000000000000cebb486e4008593f2a1d6aa5c7f50e0b2b64359f10a572fd24344ccfd30ffa0",
                    "0x0025a325f40c2da66b692f124296a6d273aa1e3afc6cde947abbb912960c1ba3382fe8b70043185278f8f6c75d7057d75ebc62e82052f480dfc207de3b0e89700e",
                    "0x0129bdbea092f4f7e6de593fd1a16ddb50b1c2a6297d4ae141a60f8da631e481750404000000000000000000000000000000000000000000000000000000000000000000330056bc75e2d630ffffffffffffffffffffffffffffffffffff4b6f9404062776c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470000000000000000000000000000000000000000000000000000000000000000000",
                    "0x5448495320495320534f4d45204d4147494320425954455320464f5220534d54206d3172525867503278704449"
                ],
                "0xAC3DecAa2009EB6b384531B43A57Fa5C10f8ec33": [
                    "0x001e2f9f788fddbe60528860cd94401896e598ff7ec689129b64b38c6f6e5d7cd00e831da32eb5166892a00be68f2ac1fe7ed65e19e0d5e4fc9220c5525af57b47",
                    "0x002ee00d41c4efcec95739e672b3c2c054103f03bf6b376a51ff27c66fd1600c972e8d75732548f8a07026d2dbb807f7844caac67aa88a9c4be518f26779d3eee0",
                    "0x002289e22abe96b0811a18c472ed2151193155e06dcb894f1cc0e415a22070e1f60511178241b5d9745ede181272654009625d78f4888c06633049bcbab1108578",
                    "0x001e70ad06df66e7e2878877adc54b4f4b3ffd6d490ac216dba8caafa16031872817a8517bc379e6f45beda1173adfd50de5cdef7035ee67f60199b82695d0e160",
                    "0x0126d5d7ba8c158e8d4a20ff7dd5315879c206cffc2d41deb795f042ea0550f709040400000000000000000000000000000000000000000000000000000000000000000001000000000000000000000000000000000000000000000000000000000000000029b74e075daad9f17eb39cd893c2dd32f52ecd99084d63964842defd00ebcbe223c319641375b3a4b50a6f9fbcd6954072add60cae4e507863924d76aef900ca00",
                    "0x5448495320495320534f4d45204d4147494320425954455320464f5220534d54206d3172525867503278704449"
                ]
            },
            "storageProofs": {
                "0x05fDbDfaE180345C6Cff5316c286727CF1a43327": {
                    "0x0000000000000000000000000000000000000000000000000000000000000000": [
                        "0x012098f5fb9e239eab3ceac3f27b81e481dc3124d55ffed523a839ee8446b6486401010000000000000000000000000000f34a8c7a8b3230be235cd3550f9a15fe5bee3aba00",
                        "0x5448495320495320534f4d45204d4147494320425954455320464f5220534d54206d3172525867503278704449"
                    ]
                },
                "0xAC3DecAa2009EB6b384531B43A57Fa5C10f8ec33": {
                    "0x0000000000000000000000000000000000000000000000000000000000000000": [
                        "0x02",
                        "0x5448495320495320534f4d45204d4147494320425954455320464f5220534d54206d3172525867503278704449"
                    ]
                }
            }
        }
        "#;

        let s_trace: StorageTrace = serde_json::from_str(example).unwrap();
        let proofs = s_trace.proofs.as_ref().unwrap();
        for (_, proof) in proofs.iter() {
            let proof: AccountProof = proof.as_slice().try_into().unwrap();
            println!("proof: {:?}", proof);
        }

        for (_, s_map) in s_trace.storage_proofs.iter() {
            for (k, val) in s_map {
                let val_proof: StorageProof = val.as_slice().try_into().unwrap();
                println!("k: {}, v: {:?}", k, val_proof);
            }
        }
    }
}
