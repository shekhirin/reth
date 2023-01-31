//!
//! Mermaid flowchart represent all blocks that can appear in blockchain. 
//! Green blocks belong to canonical chain and are saved inside database table, they are our main chain.
//! Pending blocks and sidechains are found in memory inside [`BlockchainTree`].
//! Both pending and sidechains have same mechanisms only different is when they got comited to database.
//! For pending it is just append operation but for sidechains they need to move current canonical blocks for
//! it to be come sidechain and for current sidechin to become canonical (to be saved to db). 
//! ```mermaid
//! flowchart BT
//! subgraph canonical chain
//! CanonState:::state
//! block0canon:::canon -->block1canon:::canon -->block2canon:::canon -->block3canon:::canon --> block4canon:::canon --> block5canon:::canon
//! end
//! block5canon --> block6pending:::pending
//! block5canon --> block67pending:::pending
//! subgraph sidechain2
//! S2State:::state
//! block3canon --> block4s2:::sidechain --> block5s2:::sidechain
//! end
//! subgraph sidechain1
//! S1State:::state
//! block2canon --> block3s1:::sidechain --> block4s1:::sidechain --> block5s1:::sidechain --> block6s1:::sidechain
//! end
//! classDef state fill:#1882C4
//! classDef canon fill:#8AC926
//! classDef pending fill:#FFCA3A
//! classDef sidechain fill:#FF595E
//! ```
//! 
//! 
//! 

use std::collections::HashMap;
use reth_primitives::{TransitionId, BlockNumber, SealedBlock, H256, BlockHash};


/// Side chain that contain it state and connect to block found in canonical chain.
pub struct Chain {
    /// Pendint state
    /// NOTE: This will be HashMap<Address,Account> etc.
    pub pending_state: bool,
    /// Blocks in this chain
    pub blocks: Vec<SealedBlock>,
    /// Changesets for Account/Storage for this chain
    pub changesets: bool,
    /// Newest transition id
    pub newest_transition_id: TransitionId,
    /// Oldest transition id that represent state in canonical chain.
    /// If we want to fetch some old values we need to use it.
    /// There is a difference if chains points to the tip of canonical chan or not.
    pub oldest_transition_id: TransitionId,
    /// Number of block found in canonical chain that this chain connect to 
    pub canonical_block_joint: BlockNumber,
}

/// Chain identificator
pub type ChainId = u64;

/// Tree of chains and it identifications.
pub struct BlockchainTree {
    /// Sidechans and present data
    pub side_chains: HashMap<ChainId,Chain>,
    /// Block hashes and side chain they belong
    pub blocks: HashMap<H256,ChainId>,
    /// Canonical chain tip.
    pub canonical_chain_tip: (BlockNumber,BlockHash),
    /// Needs db to save sidechain, do reorgs and push new block to canonical chain that is inside db.
    pub db: bool,
    /* Add additional indices if needed as in tx hash index to block */ 
}


impl BlockchainTree {
    /// Insert block inside tree
    pub fn insert_block(&mut self, /* */) {
        // check if block parent can be found in Tree
        // if not found, check if it can be found inside canonical chain aka db.

        // execute block and check if it is valid.
        // store it inside BlockchainTree.

        // Be careful that sidechain can depend on blocks of other sidechain. 
    }

    /// Make block and its parent canonical. Unwind chains to database if necessary.
    pub fn make_canonical(&mut self, /* block hash*/) {
        // check if chain joint point to the tip, if it is the case just push new blocks.
        
        // If canonical joint points to parent block that is not tip
        // Unwind block to that parent and add that `Chain` to BlockchainTree
        // flush new canonical to database and remove its `Chain` from `BlockchainTree`.

        // Be careful when removing sidechains, some of the other sidechains can be dependent on it.  
    }
}