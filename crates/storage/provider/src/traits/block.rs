use std::ops::Range;

use super::BlockHashProvider;
use reth_db::{cursor::DbCursorRO, tables, transaction::DbTx};
use reth_interfaces::Result;
use reth_primitives::{
    rpc::{BlockId, BlockNumber},
    Block, ChainInfo, SealedBlock, H256, U256,
};

/// Api trait for fetching `Block` related data.
pub trait BlockProvider: BlockHashProvider + Send + Sync {
    /// Returns the current info for the chain.
    fn chain_info(&self) -> Result<ChainInfo>;

    /// Returns the block. Returns `None` if block is not found.
    fn block(&self, id: BlockId) -> Result<Option<Block>>;

    /// Converts the `BlockNumber` variants.
    fn convert_block_number(
        &self,
        num: BlockNumber,
    ) -> Result<Option<reth_primitives::BlockNumber>> {
        let num = match num {
            BlockNumber::Latest => self.chain_info()?.best_number,
            BlockNumber::Earliest => 0,
            BlockNumber::Pending => return Ok(None),
            BlockNumber::Number(num) => num.as_u64(),
            BlockNumber::Finalized => return Ok(self.chain_info()?.last_finalized),
            BlockNumber::Safe => return Ok(self.chain_info()?.safe_finalized),
        };
        Ok(Some(num))
    }

    /// Get the hash of the block by matching the given id.
    fn block_hash_for_id(&self, block_id: BlockId) -> Result<Option<H256>> {
        match block_id {
            BlockId::Hash(hash) => Ok(Some(H256(hash.0))),
            BlockId::Number(num) => {
                if matches!(num, BlockNumber::Latest) {
                    return Ok(Some(self.chain_info()?.best_hash))
                }
                self.convert_block_number(num)?
                    .map(|num| self.block_hash(U256::from(num)))
                    .transpose()
                    .map(|maybe_hash| maybe_hash.flatten())
            }
        }
    }

    /// Get the number of the block by matching the given id.
    fn block_number_for_id(
        &self,
        block_id: BlockId,
    ) -> Result<Option<reth_primitives::BlockNumber>> {
        match block_id {
            BlockId::Hash(hash) => self.block_number(H256(hash.0)),
            BlockId::Number(num) => self.convert_block_number(num),
        }
    }

    /// Gets the `Block` for the given hash. Returns `None` if no block with this hash exists.
    fn block_number(&self, hash: H256) -> Result<Option<reth_primitives::BlockNumber>>;
}

/// Utilities for querying larger ranges of blocks
pub trait SealedBlockProvider {
    /// Given a range, it proceeds to return a Vec<SealedBlock> for that range.
    /// Will query all of the: Headers, Bodies, Senders,
    fn sealed_block_range(&self, range: Range<usize>) -> Result<Vec<(SealedBlock, Vec<Address>)>>;
}

use reth_db::models::BlockNumHash;
use reth_interfaces::provider::Error as ProviderError;
use reth_primitives::Address;

#[async_trait::async_trait]
impl<'a, Tx: DbTx<'a>> SealedBlockProvider for Tx {
    fn sealed_block_range(&self, range: Range<usize>) -> Result<Vec<(SealedBlock, Vec<Address>)>> {
        let tx = self;
        let start_block = range.start as u64;
        let end_block = range.end as u64;

        // Get next canonical block hashes to execute.
        let mut canonicals = tx.cursor_read::<tables::CanonicalHeaders>()?;
        // Get header with canonical hashes.
        let mut headers = tx.cursor_read::<tables::Headers>()?;
        // Get bodies with canonical hashes.
        let mut bodies_cursor = tx.cursor_read::<tables::BlockBodies>()?;
        // Get ommers with canonical hashes.
        let mut ommers_cursor = tx.cursor_read::<tables::BlockOmmers>()?;
        // Get transaction of the block that we are executing.
        let mut tx_cursor = tx.cursor_read::<tables::Transactions>()?;
        // Skip sender recovery and load signer from database.
        let mut tx_sender = tx.cursor_read::<tables::TxSenders>()?;

        // get canonical blocks (num,hash)
        let canonical_batch = canonicals
            .walk_range(start_block..end_block + 1)?
            .map(|i| i.map(BlockNumHash))
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Get block headers and bodies from canonical hashes
        let block_batch = canonical_batch
            .iter()
            .map(|key| {
                // NOTE: It probably will be faster to fetch all items from one table with cursor,
                // but to reduce complexity we are using `seek_exact` to skip some
                // edge cases that can happen.
                let (_, header) = headers
                    .seek_exact(*key)?
                    .ok_or(ProviderError::Header { number: key.number(), hash: key.hash() })?;
                let (_, body) =
                    bodies_cursor.seek_exact(*key)?.ok_or(ProviderError::BlockBody {
                        block_number: key.number(),
                        block_hash: key.hash(),
                    })?;
                let (_, stored_ommers) = ommers_cursor.seek_exact(*key)?.unwrap_or_default();

                Ok((header, body, stored_ommers.ommers))
            })
            .collect::<Result<Vec<_>>>()?;

        let mut blocks = Vec::with_capacity(block_batch.len());

        // // Fetch transactions, execute them and generate results
        for (header, body, ommers) in block_batch.into_iter() {
            let mut block = SealedBlock::default();
            block.header = header.clone().seal();
            block.ommers = ommers.iter().cloned().map(|x| x.seal()).collect();

            let block_number = header.number;
            tracing::trace!(?block_number, "getting transactions and senders");
            // iterate over all transactions
            let mut tx_walker = tx_cursor.walk(body.start_tx_id)?;
            let mut transactions = Vec::with_capacity(body.tx_count as usize);
            // get next N transactions.
            for index in body.tx_id_range() {
                let (tx_index, tx) =
                    tx_walker.next().ok_or(ProviderError::EndOfTransactionTable)??;
                if tx_index != index {
                    tracing::error!(
                        block = block_number,
                        expected = index,
                        found = tx_index,
                        ?body,
                        "Transaction gap"
                    );
                    return Err(ProviderError::TransactionsGap { missing: tx_index }.into())
                }
                transactions.push(tx);
            }

            // take signers
            let mut tx_sender_walker = tx_sender.walk(body.start_tx_id)?;
            let mut signers = Vec::with_capacity(body.tx_count as usize);
            for index in body.tx_id_range() {
                let (tx_index, tx) =
                    tx_sender_walker.next().ok_or(ProviderError::EndOfTransactionSenderTable)??;
                if tx_index != index {
                    tracing::error!(
                        block = block_number,
                        expected = index,
                        found = tx_index,
                        ?body,
                        "Signer gap"
                    );
                    return Err(ProviderError::TransactionsSignerGap { missing: tx_index }.into())
                }
                signers.push(tx);
            }

            block.body = transactions;
            blocks.push((block, signers));
        }

        Ok(blocks)
    }
}
