//! Handles substate and list of blocks.
//! have functions to split, branch and append the chain.
use reth_interfaces::{consensus::Consensus, Error};
use reth_primitives::{BlockHash, BlockNumber, Header, SealedBlock};

/// TODO: Chain substate
pub type ChainSubState = bool;

/// Side chain that contain it state and connect to block found in canonical chain.
#[derive(Default, Clone)]
pub struct Chain {
    /// Pending state
    /// NOTE: This will be HashMap<Address,Account> etc.
    pub pending_state: ChainSubState,
    /// Changesets for block and transaction.
    pub changesets: Vec<bool>,
    /// Blocks in this chain
    pub blocks: Vec<SealedBlock>,
}

/// Where does the chain connect to.
/// TODO Should be removed with Chain default. Used for scaffolding.
#[derive(Clone, Copy, Default)]
pub struct BlockJoint {
    /// Block number of block that chains branches from
    pub number: u64,
    /// Block hash of block that chains branches from
    pub hash: BlockHash,
}

/// Chain identificator
pub type ChainId = u64;

impl Chain {
    /// Return joint block number and hash.
    pub fn joint_block(&self) -> BlockJoint {
        let tip = self.first();
        BlockJoint { number: tip.number - 1, hash: tip.parent_hash }
    }

    /// Block joint number
    pub fn joint_block_number(&self) -> BlockNumber {
        self.first().number - 1
    }

    /// Block joint hash
    pub fn joint_block_hash(&self) -> BlockHash {
        self.first().parent_hash
    }

    /// First block in chain.
    pub fn first(&self) -> &SealedBlock {
        self.blocks.first().expect("Chain has at least one block for first")
    }

    /// Return tip of the chain. Chain always have at least one block inside
    pub fn tip(&self) -> &SealedBlock {
        self.last()
    }

    /// Return tip of the chain. Chain always have at least one block inside
    pub fn last(&self) -> &SealedBlock {
        self.blocks.last().expect("Chain has at least one block for last")
    }

    /// Create new chain that joins canonical block
    /// If parent block is the tip mark chan joint as [`BlockJoint::CanonicalLatest`]
    /// if not, use [`BlockJoint::Canonical`]
    pub fn new_canonical_joint<PROVIDER, CONSENSUS: Consensus>(
        _block: &SealedBlock,
        _provider: &PROVIDER,
        _consensus: &CONSENSUS,
    ) -> Result<Self, Error> {
        //
        // TODO remove default to not allow empty block chain
        Ok(Self::default())
    }

    /// Create new chain that branches out from existing side chain.
    pub fn new_chain_joint<PROVIDER, CONSENSUS: Consensus>(
        &self,
        _block: SealedBlock,
        _provider: &PROVIDER,
        _consensus: &CONSENSUS,
    ) -> Result<Self, ()> {
        // itera
        let state = ();

        // Create the state without touching provider, we dont want to do db reads if we dont need
        // to. Unwind the chain state with changesets to get to parent state that is needed
        // for executing block.

        // verify block against the parent

        // execute block and verify statechange.

        // if all is okay, return new chain back. Present chain is not modified.
        Ok(Self::default())
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

    /// Merge two chains into one by appending received chain to the current one.
    /// Take substate from newest one.
    pub fn append_chain<PROVIDER, CONSENSUS: Consensus>(
        &mut self,
        _chain: Chain,
        _provider: &PROVIDER,
        _consensus: &CONSENSUS,
    ) -> Result<(), ()> {
        Ok(())
    }

    /// Execute block against this state.
    fn execute_block<PROVIDER>(&mut self, block: SealedBlock) -> Result<(), ()> {
        Ok(())
    }

    /// Iterate over block to find block with the cache that we want to split on.
    /// Given block cache will be contained in first split. If block with hash
    /// is not found fn would return None.
    /// NOTE: Database state will only be found in second chain.
    pub fn split_at_block_hash(self, block_hash: &BlockHash) -> (Option<Chain>, Option<Chain>) {
        // TODO split
        (None, None)
    }

    /// Split chain at the number, block with given number will be included at first chain.
    /// If any chain is empty (Does not have blocks) None will be returned.
    /// NOTE: Database state will be only found in second chain.
    pub fn split_at_number(self, block_number: BlockNumber) -> (Option<Chain>, Option<Chain>) {
        // TODO split
        (None, None)
    }
}
