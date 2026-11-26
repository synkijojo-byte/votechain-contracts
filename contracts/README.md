# Contracts

Soroban smart contracts for the governance and voting system.

## Overview

This directory contains the on-chain contracts that power proposal creation, voting, and vote tallying. The main entry point is `src/lib.rs`.

## Building & Testing

```bash
cargo build --target wasm32-unknown-unknown --release
cargo test
```

## Contract Reference

### `get_votes`

Reads the recorded votes for a batch of voters on a given proposal.

```rust
get_votes(env, proposal_id, voters: Vec<Address>) -> Result<Vec<VoteRecord>, ContractError>
```

**Batch cap:** A single call accepts at most `MAX_BATCH_VOTERS` voters (currently **50**).

- A batch of **50 or fewer** voters succeeds and returns one `VoteRecord` per voter, in the same order as the input.
- A batch of **51 or more** voters fails with `ContractError::BatchTooLarge`.

**Per-call cost:** Each `get_votes` call performs one storage read per voter, so the cost scales linearly with the batch size. Callers should chunk large voter sets into batches of at most `MAX_BATCH_VOTERS` (50) to stay within the cap and keep individual calls within the ledger's resource limits.

**Boundary tests:** The test suite covers both edges of the cap — a 50-voter batch returns the correct records, and a 51-voter batch returns `BatchTooLarge`.
