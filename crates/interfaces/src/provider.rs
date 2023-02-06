use reth_primitives::{Address, BlockHash, BlockNumber, TransitionId, TxNumber, H256};

/// KV error type. They are using u32 to represent error code.
#[allow(missing_docs)]
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum Error {
    #[error("Block number {block_number} does not exist in database")]
    BlockNumber { block_number: BlockNumber },
    #[error("Block hash {block_hash:?} does not exist in Headers table")]
    BlockHash { block_hash: BlockHash },
    #[error("Block body not exists #{block_number} ({block_hash:?})")]
    BlockBody { block_number: BlockNumber, block_hash: BlockHash },
    #[error("Block transition id does not exist for block #{block_number}")]
    BlockTransition { block_number: BlockNumber },
    #[error("Block number {block_number} from block hash #{block_hash} does not exist in canonical chain")]
    BlockCanonical { block_number: BlockNumber, block_hash: BlockHash },
    #[error("Block number {block_number} with hash #{received_hash:?} is not canonical block. Canonical block hash is #{expected_hash:?}")]
    NonCanonicalBlock {
        block_number: BlockNumber,
        expected_hash: BlockHash,
        received_hash: BlockHash,
    },
    #[error("Storage ChangeSet address: ({address:?} key: {storage_key:?}) for transition:#{transition_id} does not exist")]
    StorageChangeset { transition_id: TransitionId, address: Address, storage_key: H256 },
    #[error("Account {address:?} ChangeSet for transition #{transition_id} does not exist")]
    AccountChangeset { transition_id: TransitionId, address: Address },

    /// A header is missing from the database.
    #[error("No header for block #{number} ({hash:?})")]
    Header {
        /// The block number key
        number: BlockNumber,
        /// The block hash key
        hash: H256,
    },

    #[error("Gap in transaction table. Missing tx number #{missing}.")]
    TransactionsGap { missing: TxNumber },
    #[error("Gap in transaction signer table. Missing tx number #{missing}.")]
    TransactionsSignerGap { missing: TxNumber },
    #[error("Got to the end of transaction table")]
    EndOfTransactionTable,
    #[error("Got to the end of the transaction sender table")]
    EndOfTransactionSenderTable,
}
