use super::{Nonce, TokenId};
use crate::node::{pack_fee_amount, pack_token_amount};
use bigdecimal::BigDecimal;
use bigdecimal::ToPrimitive;
use crypto::{digest::Digest, sha2::Sha256};

use super::account::AccountAddress;
use super::Engine;
use crate::params::JUBJUB_PARAMS;
use ff::{PrimeField, PrimeFieldRepr};
use franklin_crypto::alt_babyjubjub::fs::FsRepr;
use franklin_crypto::alt_babyjubjub::JubjubEngine;
use franklin_crypto::alt_babyjubjub::{edwards, AltJubjubBn256};
use franklin_crypto::eddsa::{PublicKey, Signature};
use franklin_crypto::jubjub::FixedGenerators;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use web3::types::Address;

/// Signed by user.

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum TxType {
    Transfer,
    Withdraw,
    Close,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    pub from: AccountAddress,
    pub to: AccountAddress,
    pub token: TokenId,
    pub amount: BigDecimal,
    pub fee: BigDecimal,
    pub nonce: Nonce,
    pub signature: TxSignature,
}

impl Transfer {
    const TX_TYPE: u8 = 5;
    fn get_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&[Self::TX_TYPE]);
        out.extend_from_slice(&self.from.data);
        out.extend_from_slice(&self.to.data);
        out.extend_from_slice(&self.token.to_be_bytes());
        out.extend_from_slice(&pack_token_amount(&self.amount));
        out.extend_from_slice(&pack_fee_amount(&self.fee));
        out.extend_from_slice(&self.nonce.to_be_bytes());
        out
    }

    pub fn verify_signature(&self) -> bool {
        if let Some(pub_key) = self.signature.verify_musig_pedersen(&self.get_bytes()) {
            if AccountAddress::from_pubkey(pub_key) == self.from {
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Withdraw {
    // TODO: derrive account address from signature
    pub account: AccountAddress,
    pub eth_address: Address,
    pub token: TokenId,
    /// None -> withdraw all
    pub amount: BigDecimal,
    pub fee: BigDecimal,
    pub nonce: Nonce,
    pub signature: TxSignature,
}

impl Withdraw {
    const TX_TYPE: u8 = 3;
    fn get_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&[Self::TX_TYPE]);
        out.extend_from_slice(&self.account.data);
        out.extend_from_slice(self.eth_address.as_bytes());
        out.extend_from_slice(&self.token.to_be_bytes());
        out.extend_from_slice(&self.amount.to_u128().unwrap().to_be_bytes());
        out.extend_from_slice(&pack_fee_amount(&self.fee));
        out.extend_from_slice(&self.nonce.to_be_bytes());
        out
    }

    pub fn verify_signature(&self) -> bool {
        if let Some(pub_key) = self.signature.verify_musig_pedersen(&self.get_bytes()) {
            if AccountAddress::from_pubkey(pub_key) == self.account {
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Close {
    pub account: AccountAddress,
    pub nonce: Nonce,
    pub signature: TxSignature,
}

impl Close {
    const TX_TYPE: u8 = 4;

    fn get_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&[Self::TX_TYPE]);
        out.extend_from_slice(&self.account.data);
        out.extend_from_slice(&self.nonce.to_be_bytes());
        out
    }

    pub fn verify_signature(&self) -> bool {
        if let Some(pub_key) = self.signature.verify_musig_pedersen(&self.get_bytes()) {
            if AccountAddress::from_pubkey(pub_key) == self.account {
                true
            } else {
                false
            }
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FranklinTx {
    Transfer(Transfer),
    Withdraw(Withdraw),
    Close(Close),
}

impl FranklinTx {
    pub fn hash(&self) -> Vec<u8> {
        let bytes = match self {
            FranklinTx::Transfer(tx) => tx.get_bytes(),
            FranklinTx::Withdraw(tx) => tx.get_bytes(),
            FranklinTx::Close(tx) => tx.get_bytes(),
        };

        let mut hasher = Sha256::new();
        hasher.input(&bytes);
        let mut out = vec![0u8; 32];
        hasher.result(&mut out);
        out
    }

    pub fn account(&self) -> AccountAddress {
        match self {
            FranklinTx::Transfer(tx) => tx.from.clone(),
            FranklinTx::Withdraw(tx) => tx.account.clone(),
            FranklinTx::Close(tx) => tx.account.clone(),
        }
    }

    pub fn nonce(&self) -> Nonce {
        match self {
            FranklinTx::Transfer(tx) => tx.nonce,
            FranklinTx::Withdraw(tx) => tx.nonce,
            FranklinTx::Close(tx) => tx.nonce,
        }
    }

    pub fn check_signature(&self) -> bool {
        match self {
            FranklinTx::Transfer(tx) => tx.verify_signature(),
            FranklinTx::Withdraw(tx) => tx.verify_signature(),
            FranklinTx::Close(tx) => tx.verify_signature(),
        }
    }

    pub fn get_bytes(&self) -> Vec<u8> {
        match self {
            FranklinTx::Transfer(tx) => tx.get_bytes(),
            FranklinTx::Withdraw(tx) => tx.get_bytes(),
            FranklinTx::Close(tx) => tx.get_bytes(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TxSignature {
    pub pub_key: PackedPublicKey,
    pub sign: PackedSignature,
}

impl TxSignature {
    pub fn verify_musig_pedersen(&self, msg: &[u8]) -> Option<PublicKey<Engine>> {
        let valid = self.pub_key.0.verify_musig_pedersen(
            msg,
            &self.sign.0,
            FixedGenerators::SpendingKeyGenerator,
            &JUBJUB_PARAMS,
        );
        if valid {
            Some(self.pub_key.0.clone())
        } else {
            None
        }
    }

    pub fn verify_musig_sha256(&self, msg: &[u8]) -> Option<PublicKey<Engine>> {
        let valid = self.pub_key.0.verify_musig_sha256(
            msg,
            &self.sign.0,
            FixedGenerators::SpendingKeyGenerator,
            &JUBJUB_PARAMS,
        );
        if valid {
            Some(self.pub_key.0.clone())
        } else {
            None
        }
    }
}

impl std::fmt::Debug for TxSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let hex_pk = hex::encode(&self.pub_key.serialize_packed().unwrap());
        let hex_sign = hex::encode(&self.sign.serialize_packed().unwrap());
        write!(f, "{{ pub_key: {}, sign: {} }}", hex_pk, hex_sign)
    }
}

#[derive(Clone)]
pub struct PackedPublicKey(pub PublicKey<Engine>);

impl PackedPublicKey {
    fn serialize_packed(&self) -> std::io::Result<Vec<u8>> {
        let mut packed_point = [0u8; 32];
        (self.0).0.write(packed_point.as_mut())?;
        Ok(packed_point.to_vec())
    }
}

impl Serialize for PackedPublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;
        let packed_point = self
            .serialize_packed()
            .map_err(|e| Error::custom(e.to_string()))?;

        serializer.serialize_str(&hex::encode(packed_point))
    }
}

impl<'de> Deserialize<'de> for PackedPublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        String::deserialize(deserializer).and_then(|string| {
            let bytes = hex::decode(&string).map_err(|e| Error::custom(e.to_string()))?;
            if bytes.len() != 32 {
                return Err(Error::custom("PublicKey size mismatch"));
            }
            Ok(PackedPublicKey(PublicKey::<Engine>(
                edwards::Point::read(&*bytes, &JUBJUB_PARAMS as &AltJubjubBn256).map_err(|e| {
                    Error::custom(format!("Failed to restore point: {}", e.to_string()))
                })?,
            )))
        })
    }
}

#[derive(Clone)]
pub struct PackedSignature(pub Signature<Engine>);

impl PackedSignature {
    fn serialize_packed(&self) -> std::io::Result<Vec<u8>> {
        let mut packed_signature = [0u8; 64];
        let (r_bar, s_bar) = packed_signature.as_mut().split_at_mut(32);

        (self.0).r.write(r_bar)?;
        (self.0).s.into_repr().write_le(s_bar)?;

        Ok(packed_signature.to_vec())
    }
}

impl Serialize for PackedSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::Error;

        let packed_signature = self
            .serialize_packed()
            .map_err(|e| Error::custom(e.to_string()))?;
        serializer.serialize_str(&hex::encode(&packed_signature))
    }
}

impl<'de> Deserialize<'de> for PackedSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;
        String::deserialize(deserializer).and_then(|string| {
            let bytes = hex::decode(&string).map_err(|e| Error::custom(e.to_string()))?;
            if bytes.len() != 64 {
                return Err(Error::custom("Signature size mismatch"));
            }

            let (r_bar, s_bar) = bytes.split_at(32);

            let r =
                edwards::Point::read(r_bar, &JUBJUB_PARAMS as &AltJubjubBn256).map_err(|e| {
                    Error::custom(format!(
                        "Failed to restore R point from R_bar: {}",
                        e.to_string()
                    ))
                })?;

            let mut s_repr = FsRepr::default();
            s_repr
                .read_le(s_bar)
                .map_err(|e| Error::custom(format!("s read err: {}", e.to_string())))?;

            let s = <Engine as JubjubEngine>::Fs::from_repr(s_repr).map_err(|e| {
                Error::custom(format!(
                    "Failed to restore s scalar from s_bar: {}",
                    e.to_string()
                ))
            })?;

            Ok(PackedSignature(Signature { r, s }))
        })
    }
}
