mod receipt;
mod request;
mod typed;

pub use receipt::TransactionReceipt;
pub use request::TransactionRequest;
pub use typed::*;

use reth_primitives::{
    rpc::transaction::eip2930::AccessListItem, rpc_utils::get_contract_address, Address,
    BlockNumber, Bytes, Transaction as RethTransaction, TransactionKind,
    TransactionSignedEcRecovered, TxType, H256, U128, U256, U64,
};
use serde::{Deserialize, Serialize};

/// Transaction object
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transaction {
    /// Hash
    pub hash: H256,
    /// Nonce
    pub nonce: U256,
    /// Block hash
    pub block_hash: Option<H256>,
    /// Block number
    pub block_number: Option<U256>,
    /// Transaction Index
    pub transaction_index: Option<U256>,
    /// Sender
    pub from: Address,
    /// Recipient
    pub to: Option<Address>,
    /// Transferred value
    pub value: U256,
    /// Gas Price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<U128>,
    /// Max BaseFeePerGas the user is willing to pay.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_fee_per_gas: Option<U128>,
    /// The miner's tip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_priority_fee_per_gas: Option<U128>,
    /// Gas
    pub gas: U256,
    /// Data
    pub input: Bytes,
    /// Creates contract
    pub creates: Option<Address>,
    /// The network id of the transaction, if any.
    pub chain_id: Option<U64>,
    /// The standardised V field of the signature.
    pub v: U256,
    /// The R field of the signature.
    pub r: U256,
    /// The S field of the signature.
    pub s: U256,
    /// Pre-pay to warm storage access.
    #[cfg_attr(feature = "std", serde(skip_serializing_if = "Option::is_none"))]
    pub access_list: Option<Vec<AccessListItem>>,
    /// EIP-2718 type
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub transaction_type: Option<U256>,
}

impl Transaction {
    /// Create a new rpc transction result, using the given block hash, number, and tx index fields
    /// to populate the corresponing fields in the rpc result.
    pub(crate) fn from_recovered_with_block_context(
        tx: TransactionSignedEcRecovered,
        block_hash: H256,
        block_number: BlockNumber,
        tx_index: U256,
    ) -> Self {
        let mut tx = Self::from_recovered(tx);
        tx.block_hash = Some(block_hash);
        tx.block_number = Some(U256::from(block_number));
        tx.transaction_index = Some(tx_index);
        tx
    }

    /// Create a new rpc transaction result from a signed and recovered transaction, setting
    /// environment related fields to `None`.
    ///
    /// Sets the sender public key to `None` as well.
    pub(crate) fn from_recovered(tx: TransactionSignedEcRecovered) -> Self {
        let signer = tx.signer();
        let signed_tx = tx.into_signed();

        let to = match signed_tx.kind() {
            TransactionKind::Create => None,
            TransactionKind::Call(to) => Some(*to),
        };

        let (gas_price, max_fee_per_gas) = match signed_tx.tx_type() {
            TxType::Legacy => (Some(U128::from(signed_tx.max_fee_per_gas())), None),
            TxType::EIP2930 => (None, Some(U128::from(signed_tx.max_fee_per_gas()))),
            TxType::EIP1559 => (None, Some(U128::from(signed_tx.max_fee_per_gas()))),
        };

        let creates = match signed_tx.kind() {
            TransactionKind::Create => {
                Some(get_contract_address(signer.0, U256::from(signed_tx.nonce())).0.into())
            }
            TransactionKind::Call(_) => None,
        };

        let chain_id = signed_tx.chain_id().map(|id| U64::from(*id));
        let access_list = match &signed_tx.transaction {
            RethTransaction::Legacy(_) => None,
            RethTransaction::Eip2930(tx) => Some(
                tx.access_list
                    .0
                    .iter()
                    .map(|item| AccessListItem {
                        address: item.address.0.into(),
                        storage_keys: item.storage_keys.iter().map(|key| key.0.into()).collect(),
                    })
                    .collect(),
            ),
            RethTransaction::Eip1559(tx) => Some(
                tx.access_list
                    .0
                    .iter()
                    .map(|item| AccessListItem {
                        address: item.address.0.into(),
                        storage_keys: item.storage_keys.iter().map(|key| key.0.into()).collect(),
                    })
                    .collect(),
            ),
        };

        Self {
            hash: signed_tx.hash,
            nonce: U256::from(signed_tx.nonce()),
            block_hash: None,
            block_number: None,
            transaction_index: None,
            from: signer,
            to,
            value: U256::from(U128::from(*signed_tx.value())),
            gas_price,
            max_fee_per_gas,
            max_priority_fee_per_gas: signed_tx.max_priority_fee_per_gas().map(U128::from),
            gas: U256::from(signed_tx.gas_limit()),
            input: signed_tx.input().clone(),
            creates,
            chain_id,
            v: U256::from(signed_tx.signature.v(chain_id.map(|id| id.as_u64()))),
            r: signed_tx.signature.r,
            s: signed_tx.signature.s,
            access_list,
            transaction_type: Some(U256::from(signed_tx.tx_type() as u8)),
        }
    }
}
