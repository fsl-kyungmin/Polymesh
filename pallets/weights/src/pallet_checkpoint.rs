// This file is part of Substrate.

// Copyright (C) 2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Autogenerated weights for pallet_checkpoint
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-08-24, STEPS: `100`, REPEAT: 5, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 512
//! HOSTNAME: `ubuntu-8gb-hel1-5`, CPU: `AMD EPYC Processor`

// Executed Command:
// target/release/polymesh
// benchmark
// pallet
// -s
// 100
// -r
// 5
// -p=*
// -e=*
// --heap-pages
// 4096
// --db-cache
// 512
// --execution
// wasm
// --wasm-execution
// compiled
// --output
// ./pallets/weights/src/
// --template
// ./.maintain/frame-weight-template.hbs

#![allow(unused_parens)]
#![allow(unused_imports)]

use polymesh_runtime_common::{RocksDbWeight as DbWeight, Weight};

/// Weights for pallet_checkpoint using the Substrate node and recommended hardware.
pub struct SubstrateWeight;
impl pallet_asset::checkpoint::WeightInfo for SubstrateWeight {
    // Storage: Checkpoint SchedulesMaxComplexity (r:0 w:1)
    // Proof Skipped: Checkpoint SchedulesMaxComplexity (max_values: Some(1), max_size: None, mode: Measured)
    fn set_schedules_max_complexity() -> Weight {
        // Minimum execution time: 17_984 nanoseconds.
        Weight::from_ref_time(19_707_000).saturating_add(DbWeight::get().writes(1))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Proof Skipped: Identity KeyRecords (max_values: None, max_size: None, mode: Measured)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Proof Skipped: ExternalAgents GroupOfAgent (max_values: None, max_size: None, mode: Measured)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Proof Skipped: Permissions CurrentPalletName (max_values: Some(1), max_size: None, mode: Measured)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Proof Skipped: Permissions CurrentDispatchableName (max_values: Some(1), max_size: None, mode: Measured)
    // Storage: Timestamp Now (r:1 w:0)
    // Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
    // Storage: Checkpoint CheckpointIdSequence (r:1 w:1)
    // Proof Skipped: Checkpoint CheckpointIdSequence (max_values: None, max_size: None, mode: Measured)
    // Storage: Asset Tokens (r:1 w:0)
    // Proof Skipped: Asset Tokens (max_values: None, max_size: None, mode: Measured)
    // Storage: Checkpoint TotalSupply (r:0 w:1)
    // Proof Skipped: Checkpoint TotalSupply (max_values: None, max_size: None, mode: Measured)
    // Storage: Checkpoint Timestamps (r:0 w:1)
    // Proof Skipped: Checkpoint Timestamps (max_values: None, max_size: None, mode: Measured)
    fn create_checkpoint() -> Weight {
        // Minimum execution time: 82_313 nanoseconds.
        Weight::from_ref_time(87_323_000)
            .saturating_add(DbWeight::get().reads(7))
            .saturating_add(DbWeight::get().writes(3))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Proof Skipped: Identity KeyRecords (max_values: None, max_size: None, mode: Measured)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Proof Skipped: ExternalAgents GroupOfAgent (max_values: None, max_size: None, mode: Measured)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Proof Skipped: Permissions CurrentPalletName (max_values: Some(1), max_size: None, mode: Measured)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Proof Skipped: Permissions CurrentDispatchableName (max_values: Some(1), max_size: None, mode: Measured)
    // Storage: Checkpoint SchedulesMaxComplexity (r:1 w:0)
    // Proof Skipped: Checkpoint SchedulesMaxComplexity (max_values: Some(1), max_size: None, mode: Measured)
    // Storage: Checkpoint CachedNextCheckpoints (r:1 w:1)
    // Proof Skipped: Checkpoint CachedNextCheckpoints (max_values: None, max_size: None, mode: Measured)
    // Storage: Timestamp Now (r:1 w:0)
    // Proof: Timestamp Now (max_values: Some(1), max_size: Some(8), added: 503, mode: MaxEncodedLen)
    // Storage: Checkpoint ScheduleIdSequence (r:1 w:1)
    // Proof Skipped: Checkpoint ScheduleIdSequence (max_values: None, max_size: None, mode: Measured)
    // Storage: ProtocolFee Coefficient (r:1 w:0)
    // Proof Skipped: ProtocolFee Coefficient (max_values: Some(1), max_size: None, mode: Measured)
    // Storage: ProtocolFee BaseFees (r:1 w:0)
    // Proof Skipped: ProtocolFee BaseFees (max_values: None, max_size: None, mode: Measured)
    // Storage: Checkpoint ScheduledCheckpoints (r:0 w:1)
    // Proof Skipped: Checkpoint ScheduledCheckpoints (max_values: None, max_size: None, mode: Measured)
    // Storage: Checkpoint ScheduleRefCount (r:0 w:1)
    // Proof Skipped: Checkpoint ScheduleRefCount (max_values: None, max_size: None, mode: Measured)
    fn create_schedule() -> Weight {
        // Minimum execution time: 152_706 nanoseconds.
        Weight::from_ref_time(177_963_000)
            .saturating_add(DbWeight::get().reads(10))
            .saturating_add(DbWeight::get().writes(4))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Proof Skipped: Identity KeyRecords (max_values: None, max_size: None, mode: Measured)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Proof Skipped: ExternalAgents GroupOfAgent (max_values: None, max_size: None, mode: Measured)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Proof Skipped: Permissions CurrentPalletName (max_values: Some(1), max_size: None, mode: Measured)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Proof Skipped: Permissions CurrentDispatchableName (max_values: Some(1), max_size: None, mode: Measured)
    // Storage: Checkpoint ScheduledCheckpoints (r:1 w:1)
    // Proof Skipped: Checkpoint ScheduledCheckpoints (max_values: None, max_size: None, mode: Measured)
    // Storage: Checkpoint ScheduleRefCount (r:1 w:1)
    // Proof Skipped: Checkpoint ScheduleRefCount (max_values: None, max_size: None, mode: Measured)
    // Storage: Checkpoint CachedNextCheckpoints (r:1 w:1)
    // Proof Skipped: Checkpoint CachedNextCheckpoints (max_values: None, max_size: None, mode: Measured)
    fn remove_schedule() -> Weight {
        // Minimum execution time: 149_840 nanoseconds.
        Weight::from_ref_time(152_175_000)
            .saturating_add(DbWeight::get().reads(7))
            .saturating_add(DbWeight::get().writes(3))
    }
}
