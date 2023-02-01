use reth_db::database::Database;
use reth_interfaces::consensus::{self, Consensus};
use reth_primitives::{BlockHash, BlockNumber, Header, SealedBlock, TransitionId, H256};
use reth_provider::{HeaderProvider, ShareableDatabase};
use std::collections::HashMap;

/// Side chain that contain it state and connect to block found in canonical chain.
#[derive(Default, Clone)]
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
    /// Block joint that connect this chain to parent ones. Joint block can be found in
    /// canonical chain or other side chains.
    pub block_joint: BlockJoint,
}

/// Where does the chain connect to.
/// TODO Should be removed with Chain default. Used for scaffolding.
#[derive(Clone, Copy,Default)]
pub struct BlockJoint {
    pub number: u64,
    pub hash: BlockHash,
}


/// Chain identificator
pub type ChainId = u64;

impl Chain {

    /// Return tip of the chain. Chain always have at least one block inside
    pub fn tip(&self) -> &SealedBlock {
        self.blocks.last().expect("Chain has at least one block")
    }
    /// Create new chain that joins canonical block
    /// If parent block is the tip mark chan joint as [`BlockJoint::CanonicalLatest`]
    /// if not, use [`BlockJoint::Canonical`]
    pub fn new_canonical_joint<PROVIDER, CONSENSUS: Consensus>(
        block: SealedBlock,
        parent: &Header,
        provider: &PROVIDER,
        consensus: &CONSENSUS,
    ) -> Result<Self, ()> {
        //
        // TODO remove default to not allow empty block chain
        Ok(Self::default())
    }

    /// Create new chain that branches out from existing side chain.
    pub fn new_chain_joint<PROVIDER, CONSENSUS: Consensus>(
        &self,
        block: SealedBlock,
        provider: &PROVIDER,
        consensus: &CONSENSUS,
    ) -> Result<Self, ()> {
        // itera
        let state = ();

        // Create the state without touching provider, we dont want to do db reads if we dont need to.
        // Unwind the chain state with changesets to get to parent state that is needed for executing block.

        // verify block agains parent

        // execute block and verify statechange.

        // if all is okay, return new chain back. Present chain is not modified.
        Ok(Self::default())
    }

    /// Return chain joint block number
    pub fn joint_block_number(&self) -> BlockNumber {
        self.blocks.first().expect("Chain can be empty").number - 1
    }

    /// Return chain joint block hash
    pub fn joint_block_hash(&self) -> BlockHash {
        self.blocks.first().expect("Chain can be empty").parent_hash
    }

    /// Append block to this chain
    pub fn append_block<PROVIDER, CONSENSUS: Consensus>(
        &mut self,
        block: SealedBlock,
        provider: &PROVIDER,
        consensus: &CONSENSUS,
    ) -> Result<(), ()> {
        let Some(parent) = self.blocks.last() else {return Err(())};

        // this will validate connection between child and parent.
        let _ = consensus.validate_header(&block, parent);

        // TODO execute against the pending state.

        self.blocks.push(block);
        Ok(())
    }

    /// Execute block against this state.
    fn execute_block<PROVIDER>(&mut self, block: SealedBlock) -> Result<(), ()> {
        Ok(())
    }

    /// Iterate over block to find block with the cache that we want to split on.
    /// Given block cache will be contained in first split. If block with hash
    /// is not found fn would return None.
    pub fn split_at_block_hash(self, block_hash: &BlockHash) -> (Option<Chain>,Option<Chain>) {
        (None,None)
    }

    /// Split chain at the number, block with given number will be included at first chain.
    /// If any chain is empty (Does not have blocks) None will be returned.
    pub fn split_at_number(self, block_number: BlockNumber) -> (Option<Chain>, Option<Chain>) {
        // TODO split
        (None, None)
    }
}
