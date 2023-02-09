use reth_primitives::{BlockNumber, TxNumber, H256};

/// Database error type. They are using u32 to represent error code.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
pub enum Error {
    /// Failed to open database.
    #[error("{0:?}")]
    DatabaseLocation(u32),
    /// Failed to create a table in database.
    #[error("Table Creating error code: {0:?}")]
    TableCreation(u32),
    /// Failed to insert a value into a table.
    #[error("Database write error code: {0:?}")]
    Write(u32),
    /// Failed to get a value into a table.
    #[error("Database read error code: {0:?}")]
    Read(u32),
    /// Failed to delete a `(key, value)` pair into a table.
    #[error("Database delete error code: {0:?}")]
    Delete(u32),
    /// Failed to commit transaction changes into the database.
    #[error("Database commit error code: {0:?}")]
    Commit(u32),
    /// Failed to initiate a transaction.
    #[error("Initialization of transaction errored with code: {0:?}")]
    InitTransaction(u32),
    /// Failed to initiate a cursor.
    #[error("Initialization of cursor errored with code: {0:?}")]
    InitCursor(u32),
    /// Failed to decode a key from a table..
    #[error("Error decoding value.")]
    DecodeError,
}

// TODO: dedup with provider::Error?
/// A database integrity error.
#[derive(thiserror::Error, Debug)]
#[allow(missing_docs)]
pub enum DatabaseIntegrityError {
    /// The canonical header for a block is missing from the database.
    #[error("No canonical header for block #{number}")]
    CanonicalHeader {
        /// The block number key
        number: BlockNumber,
    },
    /// A header is missing from the database.
    #[error("No header for block #{number} ({hash:?})")]
    Header {
        /// The block number key
        number: BlockNumber,
        /// The block hash key
        hash: H256,
    },
    /// A ommers are missing.
    #[error("Block ommers not found for block #{number}")]
    Ommers {
        /// The block number key
        number: BlockNumber,
    },
    /// A block body is missing.
    #[error("Block body not found for block #{number}")]
    BlockBody {
        /// The block number key
        number: BlockNumber,
    },
    /// The transaction is missing
    #[error("Transaction #{id} not found")]
    Transaction {
        /// The transaction id
        id: TxNumber,
    },
    #[error("Block transition not found for block #{number}")]
    BlockTransition { number: BlockNumber },
    #[error("Gap in transaction table. Missing tx number #{missing}.")]
    TransactionsGap { missing: TxNumber },
    #[error("Gap in transaction signer table. Missing tx number #{missing}.")]
    TransactionsSignerGap { missing: TxNumber },
    #[error("Got to the end of transaction table")]
    EndOfTransactionTable,
    #[error("Got to the end of the transaction sender table")]
    EndOfTransactionSenderTable,
    /// The total difficulty from the block header is missing.
    #[error("Total difficulty not found for block #{number}")]
    TotalDifficulty {
        /// The block number key
        number: BlockNumber,
    },
}
