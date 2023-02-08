//! Implementation of [`BlockchainTree`]
pub mod block_indices;
pub mod chain;

pub use chain::{BlockJoint, Chain, ChainId};

use reth_db::{database::Database, tables, transaction::DbTxMut};
use reth_interfaces::{consensus::Consensus, executor::Error as ExecError, Error};
use reth_primitives::{BlockHash, BlockNumber, SealedBlock};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use self::block_indices::BlockIndices;

#[cfg_attr(doc, aquamarine::aquamarine)]
/// Tree of chains and it identifications.
///
/// Mermaid flowchart represent all blocks that can appear in blockchain.
/// Green blocks belong to canonical chain and are saved inside database table, they are our main
/// chain. Pending blocks and sidechains are found in memory inside [`BlockchainTree`].
/// Both pending and sidechains have same mechanisms only difference is when they got committed to
/// database. For pending it is just append operation but for sidechains they need to move current
/// canonical blocks to BlockchainTree flush sidechain to the database to become canonical chain.
/// ```mermaid
/// flowchart BT
/// subgraph canonical chain
/// CanonState:::state
/// block0canon:::canon -->block1canon:::canon -->block2canon:::canon -->block3canon:::canon --> block4canon:::canon --> block5canon:::canon
/// end
/// block5canon --> block6pending:::pending
/// block5canon --> block67pending:::pending
/// subgraph sidechain2
/// S2State:::state
/// block3canon --> block4s2:::sidechain --> block5s2:::sidechain
/// end
/// subgraph sidechain1
/// S1State:::state
/// block2canon --> block3s1:::sidechain --> block4s1:::sidechain --> block5s1:::sidechain --> block6s1:::sidechain
/// end
/// classDef state fill:#1882C4
/// classDef canon fill:#8AC926
/// classDef pending fill:#FFCA3A
/// classDef sidechain fill:#FF595E
/// ```
///
///
/// main functions:
/// * insert_block: insert block inside tree. Execute it and save it to database.
/// * finalize_block: Flush chain that joins to finalized block.
/// * make_canonical: Check if we have the hash of block that we want to finalize and commit it to db.
/// Do reorg if needed
pub struct BlockchainTree<DB, CONSENSUS> {
    /// chains and present data
    pub chains: HashMap<ChainId, Chain>,
    /// Static chain id generator
    pub chain_id_generator: u64,
    /// Indices to block and their connection.
    pub block_indices: BlockIndices,
    /// Depth after we can prune blocks from chains and be sure that there will not be pending
    /// blocks.
    pub finalized_block: BlockNumber,
    /// Max chain height. Number of blocks that side chain can have.
    pub max_chain_length: u64,
    /// Needs db to save sidechain, do reorgs and push new block to canonical chain that is inside
    /// db.
    pub db: DB,
    /// Consensus
    pub consensus: CONSENSUS,
}

impl<DB: Database, CONSENSUS: Consensus> BlockchainTree<DB, CONSENSUS> {
    /// DONE
    /// Append block at the end of the chain or create new chain with this block.
    fn join_block_to_chain(&mut self, block: SealedBlock, chain_id: ChainId) -> Result<(), Error> {
        // or return error as insertng is not possible
        let parent_chain =
            self.chains.get_mut(&chain_id).ok_or(ExecError::ChainIdConsistency { chain_id })?;
        let last_block_hash = parent_chain.tip().hash();

        if last_block_hash == block.parent_hash {
            let _ = parent_chain.append_block(block, &self.db, &self.consensus);
        } else {
            let chain = parent_chain.new_chain_joint(block, &self.db, &self.consensus).unwrap();
            self.insert_chain(chain);
        }

        Ok(())
    }

    /// DONE
    /// Insert chain to tree and ties the blocks to it.
    /// Helper function that handles indexing and inserting.
    fn insert_chain(&mut self, chain: Chain) -> ChainId {
        let chain_id = self.chain_id_generator;
        self.chain_id_generator += 1;
        self.block_indices.insert_chain(chain_id, &chain);
        // add chain_id -> chain index
        self.chains.insert(chain_id, chain);
        chain_id
    }

    /// DONE
    /// Insert block inside tree
    pub fn insert_block(&mut self, block: SealedBlock) -> Result<(), Error> {
        // check if block number is inside pending block slide
        if block.number <= self.finalized_block {
            return Err(ExecError::PendingBlockIsFinalized {
                block_number: block.number,
                block_hash: block.hash(),
                last_finalized: self.finalized_block,
            }
            .into());
        }

        // we will not even try to insert blocks that are too far in future.
        if block.number > self.finalized_block + self.max_chain_length {
            return Err(ExecError::PendingBlockIsInFuture {
                block_number: block.number,
                block_hash: block.hash(),
                last_finalized: self.finalized_block,
            }
            .into());
        }

        // check if block parent can be found in Tree
        if let Some(parent_chain) = self.block_indices.get_block_chain_id(&block.parent_hash) {
            let _ = self.join_block_to_chain(block.clone(), parent_chain)?;
            self.db.tx_mut()?.put::<tables::PendingBlocks>(block.hash(), block.unseal())?;
            return Ok(())
        }

        // if not found, check if it can be found inside canonical chain.
        if Some(block.parent_hash) == self.block_indices.canonical_hash(&(block.number - 1)) {
            // create new chain that points to that block
            let chain = Chain::new_canonical_joint(&block, &self.db, &self.consensus)?;
            self.insert_chain(chain);
            self.db.tx_mut()?.put::<tables::PendingBlocks>(block.hash(), block.unseal())?;
            return Ok(())
        }
        // NOTE: Block dont have parent, and if we receive this block in `make_canonical` function
        // this could be a trigger to initiate syncing, as we are missing parent.
        Ok(())
    }

    // DONE
    /// Do finalization of blocks. Remove them from tree
    pub fn finalize_block(&mut self, finalized_block: BlockNumber) {
        let mut remove_chains = self.block_indices.finalize_canonical_blocks(&finalized_block);

        while let Some(chain_id) = remove_chains.first() {
            if let Some(chain) = self.chains.remove(chain_id) {
                remove_chains.extend(self.block_indices.remove_chain(&chain));
            }
        }
        self.finalized_block = finalized_block;
    }

    /// DONE
    /// Make block and its parent canonical. Unwind chains to database if necessary.
    pub fn make_canonical(&mut self, block_hash: &BlockHash) -> Result<(), ()> {
        let chain_id = self.block_indices.get_block_chain_id(block_hash).ok_or(())?;
        let chain = self.chains.remove(&chain_id).expect("To be present");
        // we are spliting chain as there is possibility that only part of chain get canonical.
        let (canonical, pending) = chain.split_at_block_hash(block_hash);
        let canonical = canonical.expect("Canonical chain is present");

        if let Some(pending) = pending {
            // joint is now canonical and latest.
            self.chains.insert(chain_id, pending);
        }

        let mut block_joint = canonical.joint_block();
        let mut block_joint_number = canonical.joint_block_number();
        let mut chains_to_promote = vec![canonical];
        // loop while joint blocks are found in Tree.
        while let Some(chain_id) = self.block_indices.get_block_chain_id(&block_joint.hash) {
            let chain = self.chains.remove(&chain_id).expect("To joint to be present");
            block_joint = chain.joint_block();
            let (canonical, rest) = chain.split_at_number(block_joint_number);
            let canonical = canonical.expect("Chain is present");
            // reinsert back the chunk of sidechain that didn't get reorged.
            if let Some(rest_of_sidechain) = rest {
                self.chains.insert(chain_id, rest_of_sidechain);
            }
            block_joint_number = canonical.joint_block_number();
            chains_to_promote.push(canonical);
        }

        let old_tip = self.block_indices.canonical_tip();
        // Merge all chain into one chain.
        let mut new_canon_chain = chains_to_promote.pop().expect("There is at least one block");
        for chain in chains_to_promote.into_iter().rev() {
            new_canon_chain.append_chain(chain, &self.db, &self.consensus)?
        }

        // if joins to the tipx
        if new_canon_chain.joint_block_hash() == old_tip.hash {
            // append to database
            self.commit_canonical(new_canon_chain)?;
        } else {
            // it joints to canonical block that is not the tip.

            let canon_joint = new_canon_chain.joint_block();
            // sanity check
            if self.block_indices.canonical_hash(&canon_joint.number) != Some(canon_joint.hash) {
                unreachable!("All chains should point to canonical chain.");
            }

            // revert `N` blocks from current canonical chain and put them inside BlockchanTree
            // This is main reorgs on tables.
            let old_canon_chain = self.revert_canonical(canon_joint.number)?;
            self.commit_canonical(new_canon_chain)?;

            // insert old canonical chain to BlockchainTree.
            self.insert_chain(old_canon_chain);
        }

        Ok(())
    }

    /// TODO
    /// Commit chain for it to become canonical. Assume we are doing pending operation to db.
    fn commit_canonical(&mut self, _chain: Chain) -> Result<(), ()> {
        // update self.block_indices

        // remove all committed blocks from Tree.
        Ok(())
    }

    /// TODO
    /// Revert canonical blocks from database and insert them to pending table
    /// Revert should be non inclusive, and revert_until should stay in db.
    /// Return the chain that represent reverted canonical blocks.
    fn revert_canonical(&mut self, _revert_until: BlockNumber) -> Result<Chain, ()> {
        // read data that is needed for new sidechain

        // Use pipeline (or part of it) to unwind canonical chain from database.

        // think about atomicity of operations. if we put canonical chain inside tree, what could
        // happen?

        // commit old canonical to pending table.

        // update self.block_indices

        Ok(Chain::default())
    }
}
