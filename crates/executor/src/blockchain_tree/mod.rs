//!
pub mod chain;

pub use chain::{BlockJoint, Chain, ChainId};

use reth_db::database::Database;
use reth_interfaces::consensus::Consensus;
use reth_primitives::{BlockHash, BlockNumber, SealedBlock, H256};
use reth_provider::{HeaderProvider, ShareableDatabase};
use std::collections::{BTreeMap, HashMap, HashSet};

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
pub struct BlockchainTree<DB, CONSENSUS> {
    /// chains and present data
    pub chains: HashMap<ChainId, Chain>,

    /// Static chain id generator
    pub chain_id_generator: u64,
    /// Block hashes and side chain they belong
    pub blocks: HashMap<H256, ChainId>,
    /// Index needed when discarding the chain, so we can remove connected chains from tree.
    pub block_child: HashMap<H256, HashSet<H256>>,
    /// Canonical chain. Contains N number (depends on `finalization_depth`) of blocks  
    pub canonical_chain: BTreeMap<BlockNumber, BlockHash>,
    /// Depth after we can prune blocks from chains and be sure that there will not be pending blocks.
    pub finalization_block: BlockNumber,
    /// Max chain height. Number of blocks that side chain can have.
    pub max_chain_length: u64,
    /// Needs db to save sidechain, do reorgs and push new block to canonical chain that is inside db.
    pub db: DB,
    /// Consensus
    pub consensus: CONSENSUS,
    /* Add additional indices if needed as in tx hash index to block */
}

///
/// main functions
/// insert_block
/// finalize_blook
/// make_canonical
/// 

impl<DB: Database, CONSENSUS: Consensus> BlockchainTree<DB, CONSENSUS> {
    /// Append block at the end of the chain or create new chain with this block.
    fn insert_block_in_chain(&mut self, block: SealedBlock, chain_id: ChainId) -> Result<(), ()> {
        // or return error as insertng is not possible
        let parent_chain = self.chains.get_mut(&chain_id).ok_or(())?;
        let last_block_hash = parent_chain.tip().hash();

        if last_block_hash == block.parent_hash {
            let _ = parent_chain.append_block(block, &self.db, &self.consensus);
        } else {
            let chain = parent_chain.new_chain_joint(block, &self.db, &self.consensus).unwrap();
            self.insert_chain(chain);
        }

        Ok(())
    }

    /// Insert chain to tree and ties the blocks to it.
    fn insert_chain(&mut self, chain: Chain) -> ChainId {
        let chain_id = self.chain_id_generator;
        // add block -> chain_id index
        self.blocks.extend(chain.blocks.iter().map(|h| (h.hash(), chain_id)));
        // add chain_id -> chain
        self.chains.insert(chain_id, chain);
        self.chain_id_generator += 1;
        chain_id
    }

    /// Insert block inside tree
    // Done
    pub fn insert_block(&mut self, block: SealedBlock) -> Result<(), ()> {
        // check if block parent can be found in Tree
        if let Some(parent_chain) = self.blocks.get(&block.parent_hash) {
            self.insert_block_in_chain(block, *parent_chain)
        // if not found, check if it can be found inside canonical chain aka db.
        } else if let Some(parent) =
            ShareableDatabase::new(&self.db).header(&block.parent_hash).ok().flatten()
        {
            // create new chain that points to that block
            let chain = Chain::new_canonical_joint(block, &parent, &self.db, &self.consensus)?;
            self.insert_chain(chain);
            Ok(())
        } else {
            // TODO: fetch from p2p or discard if no parent is present.
            // see how to handle recovery after this as can could receive this block
            // in `make_canonical` function
            return Ok(());
        }
        // TODO insert block to DB
    }

    pub fn finalize_block(&mut self, finalized_block_num: BlockNumber) -> Result<(), ()> {
        //let blocks_removed = HashMap::new();

        // iter

        //let filtered_chains = self.chains.

        //self.finalization_block = finalized_block_num
        Ok(())
    }

    /// Make block and its parent canonical. Unwind chains to database if necessary.
    pub fn make_canonical(&mut self, block_hash: &BlockHash) -> Result<(), ()> {
        let chain_id = *self.blocks.get(block_hash).ok_or(())?;
        let chain = self.chains.remove(&chain_id).expect("To be present");
        // we are spliting chain as there is possibility that only part of chain get canonical.
        let (canonical, pending) = chain.split_at_block_hash(block_hash);
        let canonical = canonical.expect("Canonical chain is present");

        if let Some(pending) = pending {
            // joint is now canonical and latest.
            self.chains.insert(chain_id, pending);
        }

        let mut block_joint = canonical.block_joint;
        let mut block_joint_number = canonical.joint_block_number();
        let mut chains_to_promote = vec![(chain_id, canonical)];
        // loop while joint blocks are found in Tree.
        while let Some(chain_id) = self.blocks.get(&block_joint.hash).cloned() {
            let chain = self.chains.remove(&chain_id).expect("To joint to be present");
            block_joint = chain.block_joint;
            let (canonical, rest) = chain.split_at_number(block_joint_number);
            let canonical = canonical.expect("Chain is present");
            // reinsert back the chunk of sidechain that didn't get reorged.
            if let Some(rest_of_sidechain) = rest {
                self.chains.insert(chain_id, rest_of_sidechain);
            }
            block_joint_number = canonical.joint_block_number();
            chains_to_promote.push((chain_id, canonical));
        }

        let (_, &the_tip) =
            self.canonical_chain.last_key_value().expect("There is always the canonical chain");
        let block = self.canonical_chain.get(&block_joint.number).cloned();

        // if joins to the tip
        if block_joint.hash == the_tip {
            // append to database
        } else if block == Some(block_joint.hash) {
            // it joints to canonical but not tip

            // last chain is first that needs to be flushed
            let revert_until = chains_to_promote
                .last()
                .expect("there is at least one block")
                .1
                .joint_block_number();

            // revert `N` blocks from canonical chain and put them inside BlockchanTree
            self.revert_canonical(revert_until)?;

            for (chain_id, new_canonical_chain) in chains_to_promote.into_iter().rev() {
                // iterate over all chains that has canonical joints and point them to this chain.
                self.commit_canonical(new_canonical_chain)?;

                // When flushing out all blocks, removed them from BlockchanTree.
            }
        } else {
            unreachable!("As while loop flushed all chains blocks")
        }

        // If canonical joint points to parent block that is not tip
        // Unwind block to that parent and add that `Chain` to BlockchainTree
        // flush new canonical to database and remove its `Chain` from `BlockchainTree`.

        Ok(())
    }

    /// Commit chain for it to become canonical. Assume we are doing pending operation to db.
    fn commit_canonical(&mut self, chain: Chain) -> Result<(), ()> {
        Ok(())
    }

    /// Revert canonical blocks from database and insert them to pending table
    fn revert_canonical(&mut self, revert_until: BlockNumber) -> Result<(), ()> {
        // read data that is needed for new sidechain

        // Use pipeline (or part of it) to unwind canonical chain from database.

        // think about atomicity of operations. if we put canonical chain inside tree, what could happen?

        // commit old canonical to pending table.

        Ok(())
    }
}
