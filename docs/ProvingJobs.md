# Proving Jobs Architecture

## Overview

This document describes the proving jobs architecture for both Realm and Coordinator processors, including the tree structure of different proof types and their public inputs layout.

## Public Inputs Layout Standard

All circuits follow a consistent public inputs layout:
- **[0..4]**: commitment
- **[4..8]**: worker_public_key
- **[8..12]**: circuit-specific hash (usually the hash of the main data structure)

## Realm Proving Jobs

### User Operations Tree

```mermaid
graph TB
    subgraph "User Operations Leaves"
        UO1[UserOp 1<br/>Circuit: ProcessUserOp]
        UO2[UserOp 2<br/>Circuit: ProcessUserOp]
        UO3[UserOp 3<br/>Circuit: ProcessUserOp]
        UON[UserOp N<br/>Circuit: ProcessUserOp]
    end

    subgraph "Aggregation Layer"
        AGG1[Aggregate UserOps<br/>Circuit: AggregateUserOps]
        AGG2[Aggregate UserOps<br/>Circuit: AggregateUserOps]
    end

    subgraph "Root"
        ROOT[Realm State Transition<br/>Circuit: RealmStateTransition]
    end

    UO1 --> AGG1
    UO2 --> AGG1
    UO3 --> AGG2
    UON --> AGG2
    AGG1 --> ROOT
    AGG2 --> ROOT
```

### Realm Circuit Details

| Circuit | Type | Public Inputs | Commitment Calculation |
|---------|------|---------------|------------------------|
| ProcessUserOp | Leaf | [0..4]: commitment<br/>[4..8]: worker_public_key<br/>[8..12]: user_op_hash | commitment = worker_public_key |
| AggregateUserOps | Intermediate | [0..4]: commitment<br/>[4..8]: worker_public_key<br/>[8..12]: agg_hash | commitment = hash(left.commitment, right.commitment) |
| RealmStateTransition | Root | [0..4]: commitment<br/>[4..8]: worker_public_key<br/>[8..12]: state_transition_hash | commitment = hash of all child commitments |

## Coordinator Proving Jobs

### Three Main Trees + Final Aggregation

```mermaid
graph TB
    subgraph "GUTA Tree"
        subgraph "GUTA Leaves"
            GUTA1[Realm GUTA 1]
            GUTA2[Realm GUTA 2]
            GUTAN[Realm GUTA N]
        end

        subgraph "GUTA Aggregation"
            GUTA_AGG1[GUTATwoGUTA]
            GUTA_AGG2[GUTATwoGUTA]
            GUTA_CAP[GUTAVerifyToCap<br/>Optional]
        end

        GUTA1 --> GUTA_AGG1
        GUTA2 --> GUTA_AGG1
        GUTAN --> GUTA_AGG2
        GUTA_AGG1 --> GUTA_CAP
        GUTA_AGG2 --> GUTA_CAP
    end

    subgraph "Register Users Tree"
        subgraph "Register Users Leaves"
            RU1[Batch 1<br/>Circuit: BatchAppendUserRegistrationTree]
            RU2[Batch 2<br/>Circuit: BatchAppendUserRegistrationTree]
            RUN[Batch N<br/>Circuit: BatchAppendUserRegistrationTree]
        end

        subgraph "Register Users Aggregation"
            RU_AGG1[Circuit: AggStateTransition]
            RU_AGG2[Circuit: AggStateTransition]
            RU_ROOT[Root Aggregation<br/>Circuit: AggStateTransition]
        end

        RU1 --> RU_AGG1
        RU2 --> RU_AGG1
        RUN --> RU_AGG2
        RU_AGG1 --> RU_ROOT
        RU_AGG2 --> RU_ROOT
    end

    subgraph "Deploy Contracts Tree"
        subgraph "Deploy Contracts Leaves"
            DC1[Batch 1<br/>Circuit: BatchDeployContracts]
            DC2[Batch 2<br/>Circuit: BatchDeployContracts]
            DCN[Batch N<br/>Circuit: BatchDeployContracts]
        end

        subgraph "Deploy Contracts Aggregation"
            DC_AGG1[Circuit: AggStateTransition]
            DC_AGG2[Circuit: AggStateTransition]
            DC_ROOT[Root Aggregation<br/>Circuit: AggStateTransition]
        end

        DC1 --> DC_AGG1
        DC2 --> DC_AGG1
        DCN --> DC_AGG2
        DC_AGG1 --> DC_ROOT
        DC_AGG2 --> DC_ROOT
    end

    subgraph "Final Aggregation"
        STATE_PART_1[State Part 1<br/>Circuit: AggUserRegistrationDeployContractsGUTA]
        CHECKPOINT[Checkpoint State Transition<br/>Circuit: CheckpointStateTransition]
    end

    GUTA_CAP --> STATE_PART_1
    RU_ROOT --> STATE_PART_1
    DC_ROOT --> STATE_PART_1
    STATE_PART_1 --> CHECKPOINT
```

## GUTA Circuit Variants

The GUTA (Global User Tree Aggregator) has multiple circuit variants to handle different scenarios:

### GUTA Circuit Types and Usage

```mermaid
graph LR
    subgraph "Leaf Circuits (No Child Proofs)"
        GNC[GUTANoChange<br/>No state changes]
        GSE[GUTASingleEndCap<br/>Single realm update]
        GOR[GUTAOnlyRegisterUsers<br/>Only user registrations]
        GRU[GUTARegisterUsers<br/>With user ops]
    end

    subgraph "Two Children Aggregation"
        GTG[GUTATwoGUTA<br/>Two GUTA proofs]
        GTE[GUTATwoEndCap<br/>Two EndCap proofs]
        GLR[GUTALeftGUTARightEndCap<br/>GUTA + EndCap]
        GLE[GUTALeftEndCapRightGUTA<br/>EndCap + GUTA]
    end

    subgraph "Special Purpose"
        GVC[GUTAVerifyToCap<br/>Verify to tree cap]
    end
```

### GUTA Circuit Details

| Circuit | Purpose | Children | Commitment Calculation |
|---------|---------|----------|------------------------|
| **Leaf Circuits** |
| GUTANoChange | No state changes in checkpoint | None | commitment = worker_public_key |
| GUTASingleEndCap | Single realm had updates | None | commitment = worker_public_key |
| GUTAOnlyRegisterUsers | Only user registrations, no ops | None | commitment = worker_public_key |
| GUTARegisterUsers | User registrations with ops | None | commitment = worker_public_key |
| **Aggregation Circuits** |
| GUTATwoGUTA | Aggregate two GUTA proofs | 2 GUTA | commitment = hash(a.commitment, b.commitment) |
| GUTATwoEndCap | Aggregate two EndCap proofs | 2 EndCap | commitment = hash(worker_public_key, worker_public_key)* |
| GUTALeftGUTARightEndCap | GUTA on left, EndCap on right | 1 GUTA + 1 EndCap | commitment = hash(a.commitment, worker_public_key) |
| GUTALeftEndCapRightGUTA | EndCap on left, GUTA on right | 1 EndCap + 1 GUTA | commitment = hash(worker_public_key, b.commitment)* |
| **Special Circuits** |
| GUTAVerifyToCap | Verify GUTA to tree cap | 1 GUTA | commitment = worker_public_key |

*Note: These circuits currently have incorrect commitment calculations that need fixing.

## State Part 1 (AggUserRegistrationDeployContractsGUTA)

This circuit aggregates the three main trees:

### Inputs
- Register Users proof (from aggregation root)
- Deploy Contracts proof (from aggregation root)
- GUTA proof (from aggregation root or GUTAVerifyToCap)

### Public Inputs Layout
- **[0..4]**: state_transition_hash
- **[4..8]**: register_users_root = hash(register_users_proof.commitment, register_users_proof.worker_public_key)
- **[8..12]**: deploy_contracts_root = hash(deploy_contracts_proof.commitment, deploy_contracts_proof.worker_public_key)
- **[12..16]**: gutas_root = hash(guta_proof.commitment, guta_proof.worker_public_key)

### PM Rewards Commitment
The PM (Prover/Miner) Rewards Commitment is calculated from these three roots:
```rust
PMRewardCommitment {
    register_users_root,
    deploy_contracts_root,
    gutas_root,
}
```

## Checkpoint State Transition

The final circuit that creates the checkpoint proof:

### Inputs
- State Part 1 proof
- Previous checkpoint proof
- Checkpoint tree merkle proof
- Various metadata (block time, random seed, etc.)

### Public Inputs
- Checkpoint hash
- New checkpoint tree root
- State transition proof

## Job Dependencies and Task Graph

```mermaid
graph LR
    subgraph "Parallel Execution"
        RU[Register Users Jobs]
        DC[Deploy Contracts Jobs]
        GUTA[GUTA Jobs]
    end

    subgraph "Sequential Dependencies"
        SP1[State Part 1]
        CST[Checkpoint State Transition]
        NOTIFY[Notify Block Complete]
    end

    RU --> SP1
    DC --> SP1
    GUTA --> SP1
    SP1 --> CST
    CST --> NOTIFY
```

## Key Design Principles

1. **Consistent Public Inputs**: All circuits follow the same [commitment, worker_public_key, data_hash] layout
2. **Tree Aggregation**: Each category (GUTA, Register Users, Deploy Contracts) forms its own tree
3. **Parallel Processing**: The three trees can be processed in parallel
4. **Commitment Chain**: Commitments flow up from leaves to root, enabling reward distribution
5. **Flexibility**: GUTA circuits handle various scenarios (no changes, single realm, multiple realms)
