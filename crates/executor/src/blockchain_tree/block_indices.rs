//! Implementation of [`BlockIndices`] related to [`super::BlockchainTree`]

use super::chain::{BlockJoint, Chain, ChainId};
use reth_primitives::{BlockHash, BlockNumber};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Internal indices of the block.
#[derive(Default)]
pub struct BlockIndices {
    /// Index needed when discarding the chain, so we can remove connected chains from tree.
    /// NOTE: It contains just a blocks that are forks as a key and not all blocks.
    pub fork_to_child: HashMap<BlockHash, HashSet<BlockHash>>,
    /// Canonical chain. Contains N number (depends on `finalization_depth`) of blocks.
    /// These blocks are found in fork_to_child but not inside `blocks_to_chain` or
    /// `number_to_block` as those are chain specific indices.
    pub canonical_chain: BTreeMap<BlockNumber, BlockHash>,
    /// Block hashes and side chain they belong
    pub blocks_to_chain: HashMap<BlockHash, ChainId>,
    /* Add additional indices if needed as in tx hash index to block */
    /// Utility index, Block number to block hash.
    pub number_to_block: HashMap<BlockNumber, HashSet<BlockHash>>,
}

impl BlockIndices {
    /// Insert block to chain and fork child indices of the new chain
    pub fn insert_chain(&mut self, chain_id: ChainId, chain: &Chain) {
        for block in chain.blocks.iter() {
            // add block -> chain_id index
            self.blocks_to_chain.insert(block.hash(), chain_id);
            // add number -> block
            self.number_to_block.entry(block.number).or_default().insert(block.hash());
        }
        let first = chain.first();
        // add parent block -> block index
        self.fork_to_child.entry(first.parent_hash).or_default().insert(first.hash());
    }

    /// get block chain id
    pub fn get_block_chain_id(&self, block: &BlockHash) -> Option<ChainId> {
        self.blocks_to_chain.get(block).cloned()
    }

    /// DONE
    /// Remove chain from indices and return dependent chains that needs to be removed.
    /// Does the cleaning of the tree and removing blocks from the chain.
    pub fn remove_chain(&mut self, chain: &Chain) -> BTreeSet<ChainId> {
        let mut lose_chains = BTreeSet::new();
        for block in chain.blocks.iter() {
            let block_number = block.number;
            let block_hash = block.hash();

            // rm number -> block
            if let Some(set) = self.number_to_block.get_mut(&block_number) {
                set.remove(&block_hash);
            }
            // rm block -> chain_id
            self.blocks_to_chain.remove(&block_hash);

            // rm fork -> child
            if let Some(fork_blocks) = self.fork_to_child.remove(&block_hash) {
                lose_chains = fork_blocks.into_iter().fold(lose_chains, |mut fold, fork_child| {
                    if let Some(lose_chain) = self.blocks_to_chain.remove(&fork_child) {
                        fold.insert(lose_chain);
                    }
                    fold
                });
            }
        }
        lose_chains
    }

    /// DONE
    /// Used for finalization of block.
    /// Return list of chains that depends on finalized canonical chain.
    pub fn finalize_canonical_blocks(&mut self, block_number: &BlockNumber) -> BTreeSet<ChainId> {
        // +1 is to have first split to include the block_number.
        // `split_off` is returning second half of the btree.
        let mut finalized_blocks = self.canonical_chain.split_off(&(block_number + 1));

        // only save first N blocks.
        core::mem::swap(&mut finalized_blocks, &mut self.canonical_chain);

        let mut lose_chains = BTreeSet::new();

        for (_, block_hash) in finalized_blocks.into_iter() {
            // there is a fork block.
            if let Some(fork_blocks) = self.fork_to_child.remove(&block_hash) {
                lose_chains = fork_blocks.into_iter().fold(lose_chains, |mut fold, fork_child| {
                    if let Some(lose_chain) = self.blocks_to_chain.remove(&fork_child) {
                        fold.insert(lose_chain);
                    }
                    fold
                });
            }
        }

        lose_chains
    }

    /// get canonical hash
    pub fn canonical_hash(&self, block_number: &BlockNumber) -> Option<BlockHash> {
        self.canonical_chain.get(block_number).cloned()
    }

    /// get canonical tip
    pub fn canonical_tip(&self) -> BlockJoint {
        let (&number, &hash) =
            self.canonical_chain.last_key_value().expect("There is always the canonical chain");
        BlockJoint { number, hash }
    }
}
