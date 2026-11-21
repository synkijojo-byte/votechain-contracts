//! Token contract with governance proposal support.
//!
//! Proposals snapshot the total supply at creation time so that later mints
//! cannot relax the quorum bound enforced by `update_quorum`.

use soroban_sdk::{contract, contractimpl, contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Proposal {
    pub id: u32,
    pub quorum: i128,
    /// Total supply captured when the proposal was created.
    pub supply_snapshot: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    TotalSupply,
    ProposalCount,
    Proposal(u32),
}

#[contract]
pub struct TokenContract;

#[contractimpl]
impl TokenContract {
    pub fn total_supply(env: Env) -> i128 {
        env.storage()
            .instance()
            .get(&DataKey::TotalSupply)
            .unwrap_or(0)
    }

    pub fn mint(env: Env, amount: i128) {
        let supply = Self::total_supply(env.clone());
        env.storage()
            .instance()
            .set(&DataKey::TotalSupply, &(supply + amount));
    }

    /// Create a proposal, snapshotting the current total supply.
    ///
    /// Rejects a quorum that exceeds the current supply and stores the
    /// creation-time supply so later mints cannot relax the bound.
    pub fn create_proposal(env: Env, quorum: i128) -> u32 {
        let supply_snapshot = Self::total_supply(env.clone());
        if quorum > supply_snapshot {
            panic!("quorum exceeds supply");
        }

        let id: u32 = env
            .storage()
            .instance()
            .get(&DataKey::ProposalCount)
            .unwrap_or(0);

        let proposal = Proposal {
            id,
            quorum,
            supply_snapshot,
        };
        env.storage()
            .instance()
            .set(&DataKey::Proposal(id), &proposal);
        env.storage()
            .instance()
            .set(&DataKey::ProposalCount, &(id + 1));

        id
    }

    pub fn get_proposal(env: Env, id: u32) -> Proposal {
        env.storage()
            .instance()
            .get(&DataKey::Proposal(id))
            .expect("proposal not found")
    }

    /// Update a proposal's quorum, validating against the creation-time
    /// supply snapshot rather than the current total supply.
    pub fn update_quorum(env: Env, id: u32, quorum: i128) {
        let mut proposal = Self::get_proposal(env.clone(), id);
        if quorum > proposal.supply_snapshot {
            panic!("quorum exceeds supply snapshot");
        }
        proposal.quorum = quorum;
        env.storage()
            .instance()
            .set(&DataKey::Proposal(id), &proposal);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn create_proposal_snapshots_supply() {
        let env = Env::default();
        let client = TokenContractClient::new(&env, &env.register_contract(None, TokenContract));

        client.mint(&100);
        let id = client.create_proposal(&50);
        let proposal = client.get_proposal(&id);
        assert_eq!(proposal.supply_snapshot, 100);
        assert_eq!(proposal.quorum, 50);
    }

    #[test]
    #[should_panic(expected = "quorum exceeds supply")]
    fn create_proposal_rejects_quorum_above_supply() {
        let env = Env::default();
        let client = TokenContractClient::new(&env, &env.register_contract(None, TokenContract));

        client.mint(&100);
        client.create_proposal(&101);
    }

    #[test]
    fn mint_after_create_does_not_relax_quorum_bound() {
        let env = Env::default();
        let client = TokenContractClient::new(&env, &env.register_contract(None, TokenContract));

        client.mint(&100);
        let id = client.create_proposal(&50);

        // Minting after creation must not raise the effective bound.
        client.mint(&1000);
        assert_eq!(client.total_supply(), 1100);

        // Raising quorum above the creation-time snapshot must still fail.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.update_quorum(&id, &101);
        }));
        assert!(result.is_err());

        // The stored snapshot is unchanged and the quorum is still bounded.
        let proposal = client.get_proposal(&id);
        assert_eq!(proposal.supply_snapshot, 100);
        assert_eq!(proposal.quorum, 50);

        // A quorum within the snapshot is still accepted.
        client.update_quorum(&id, &100);
        assert_eq!(client.get_proposal(&id).quorum, 100);
    }
}
