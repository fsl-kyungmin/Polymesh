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

//! Autogenerated weights for pallet_compliance_manager
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-06-28, STEPS: `100`, REPEAT: 5, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 512

// Executed Command:
// ./target/release/polymesh
// benchmark
// pallet
// -s
// 100
// -r
// 5
// -p=pallet_compliance_manager
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

/// Weights for pallet_compliance_manager using the Substrate node and recommended hardware.
pub struct WeightInfo;
impl pallet_compliance_manager::WeightInfo for WeightInfo {
    fn condition_costs(a: u32, b: u32, c: u32, d: u32) -> Weight {
        (0 as Weight)
            // Standard Error: 522_000
            .saturating_add((9_579_000 as Weight).saturating_mul(a as Weight))
            // Standard Error: 522_000
            .saturating_add((5_484_000 as Weight).saturating_mul(b as Weight))
            // Standard Error: 522_000
            .saturating_add((5_395_000 as Weight).saturating_mul(c as Weight))
            // Standard Error: 522_000
            .saturating_add((968_000 as Weight).saturating_mul(d as Weight))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: ComplianceManager AssetCompliances (r:1 w:1)
    // Storage: ComplianceManager TrustedClaimIssuer (r:1 w:0)
    // Storage: ProtocolFee Coefficient (r:1 w:0)
    // Storage: ProtocolFee BaseFees (r:1 w:0)
    fn add_compliance_requirement(c: u32) -> Weight {
        (115_178_000 as Weight)
            // Standard Error: 1_087_000
            .saturating_add((387_000 as Weight).saturating_mul(c as Weight))
            .saturating_add(DbWeight::get().reads(8 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: ComplianceManager AssetCompliances (r:1 w:1)
    fn remove_compliance_requirement() -> Weight {
        (85_598_000 as Weight)
            .saturating_add(DbWeight::get().reads(5 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: ComplianceManager AssetCompliances (r:1 w:1)
    fn pause_asset_compliance() -> Weight {
        (108_303_000 as Weight)
            .saturating_add(DbWeight::get().reads(5 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: ComplianceManager AssetCompliances (r:1 w:1)
    fn resume_asset_compliance() -> Weight {
        (74_689_000 as Weight)
            .saturating_add(DbWeight::get().reads(5 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Identity DidRecords (r:1 w:0)
    // Storage: ComplianceManager TrustedClaimIssuer (r:1 w:1)
    // Storage: ComplianceManager AssetCompliances (r:1 w:0)
    fn add_default_trusted_claim_issuer() -> Weight {
        (89_973_000 as Weight)
            .saturating_add(DbWeight::get().reads(7 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: ComplianceManager TrustedClaimIssuer (r:1 w:1)
    fn remove_default_trusted_claim_issuer() -> Weight {
        (73_911_000 as Weight)
            .saturating_add(DbWeight::get().reads(5 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: ComplianceManager AssetCompliances (r:1 w:1)
    // Storage: ComplianceManager TrustedClaimIssuer (r:1 w:0)
    fn change_compliance_requirement(c: u32) -> Weight {
        (100_696_000 as Weight)
            // Standard Error: 1_115_000
            .saturating_add((1_832_000 as Weight).saturating_mul(c as Weight))
            .saturating_add(DbWeight::get().reads(6 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: ComplianceManager TrustedClaimIssuer (r:1 w:0)
    // Storage: ComplianceManager AssetCompliances (r:1 w:1)
    fn replace_asset_compliance(c: u32) -> Weight {
        (103_333_000 as Weight)
            // Standard Error: 2_864_000
            .saturating_add((10_787_000 as Weight).saturating_mul(c as Weight))
            .saturating_add(DbWeight::get().reads(6 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: ComplianceManager AssetCompliances (r:0 w:1)
    fn reset_asset_compliance() -> Weight {
        (69_229_000 as Weight)
            .saturating_add(DbWeight::get().reads(4 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
}
