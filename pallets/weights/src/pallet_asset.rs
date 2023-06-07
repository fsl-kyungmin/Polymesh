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

//! Autogenerated weights for pallet_asset
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-01-25, STEPS: `100`, REPEAT: 5, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 512
//! HOSTNAME: `dev-fsn001`, CPU: `AMD Ryzen 9 5950X 16-Core Processor`

// Executed Command:
// ./target/release/polymesh
// benchmark
// pallet
// -s
// 100
// -r
// 5
// -p=pallet_asset
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

/// Weights for pallet_asset using the Substrate node and recommended hardware.
pub struct SubstrateWeight;
impl pallet_asset::WeightInfo for SubstrateWeight {
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: Asset Tokens (r:1 w:0)
    // Storage: Asset TickerConfig (r:1 w:0)
    // Storage: Asset Tickers (r:1 w:1)
    // Storage: Timestamp Now (r:1 w:0)
    // Storage: ProtocolFee Coefficient (r:1 w:0)
    // Storage: ProtocolFee BaseFees (r:1 w:0)
    // Storage: Identity CurrentPayer (r:1 w:0)
    // Storage: Asset AssetOwnershipRelations (r:0 w:1)
    // Storage: Asset ClassicTickers (r:0 w:1)
    fn register_ticker() -> Weight {
        // Minimum execution time: 53_809 nanoseconds.
        Weight::from_ref_time(55_263_000)
            .saturating_add(DbWeight::get().reads(8))
            .saturating_add(DbWeight::get().writes(3))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: Identity Authorizations (r:1 w:1)
    // Storage: Asset Tokens (r:1 w:0)
    // Storage: Asset Tickers (r:1 w:1)
    // Storage: Identity AuthorizationsGiven (r:0 w:1)
    // Storage: Asset AssetOwnershipRelations (r:0 w:2)
    // Storage: Asset ClassicTickers (r:0 w:1)
    fn accept_ticker_transfer() -> Weight {
        // Minimum execution time: 59_149 nanoseconds.
        Weight::from_ref_time(59_560_000)
            .saturating_add(DbWeight::get().reads(4))
            .saturating_add(DbWeight::get().writes(6))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: Identity Authorizations (r:1 w:1)
    // Storage: Asset Tokens (r:1 w:1)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset Tickers (r:1 w:1)
    // Storage: Identity AuthorizationsGiven (r:0 w:1)
    // Storage: Asset AssetOwnershipRelations (r:0 w:2)
    fn accept_asset_ownership_transfer() -> Weight {
        // Minimum execution time: 72_134 nanoseconds.
        Weight::from_ref_time(73_546_000)
            .saturating_add(DbWeight::get().reads(7))
            .saturating_add(DbWeight::get().writes(6))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: Asset Tokens (r:1 w:1)
    // Storage: Asset TickerConfig (r:1 w:0)
    // Storage: Asset Tickers (r:1 w:1)
    // Storage: Identity DidRecords (r:1 w:1)
    // Storage: Portfolio PortfolioCustodian (r:1 w:0)
    // Storage: ProtocolFee Coefficient (r:1 w:0)
    // Storage: ProtocolFee BaseFees (r:2 w:0)
    // Storage: Identity CurrentPayer (r:1 w:0)
    // Storage: ExternalAgents NumFullAgents (r:1 w:1)
    // Storage: Asset FundingRound (r:0 w:1)
    // Storage: Asset AssetOwnershipRelations (r:0 w:1)
    // Storage: Asset AssetNames (r:0 w:1)
    // Storage: Asset ClassicTickers (r:0 w:1)
    // Storage: Asset DisableInvestorUniqueness (r:0 w:1)
    // Storage: Asset Identifiers (r:0 w:1)
    // Storage: ExternalAgents AgentOf (r:0 w:1)
    // Storage: ExternalAgents GroupOfAgent (r:0 w:1)
    /// The range of component `n` is `[1, 128]`.
    /// The range of component `i` is `[1, 512]`.
    /// The range of component `f` is `[1, 128]`.
    fn create_asset(n: u32, i: u32, f: u32) -> Weight {
        // Minimum execution time: 98_963 nanoseconds.
        Weight::from_ref_time(103_561_555)
            // Manually set weight for `n`
            .saturating_add(Weight::from_ref_time(100_000).saturating_mul(n.into()))
            // Standard Error: 1_327
            .saturating_add(Weight::from_ref_time(66_001).saturating_mul(i.into()))
            // Manually set weight for `f`
            .saturating_add(Weight::from_ref_time(100_000).saturating_mul(f.into()))
            .saturating_add(DbWeight::get().reads(11))
            .saturating_add(DbWeight::get().writes(12))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset Tokens (r:1 w:0)
    // Storage: Asset Frozen (r:1 w:1)
    fn freeze() -> Weight {
        // Minimum execution time: 45_164 nanoseconds.
        Weight::from_ref_time(46_325_000)
            .saturating_add(DbWeight::get().reads(6))
            .saturating_add(DbWeight::get().writes(1))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset Tokens (r:1 w:0)
    // Storage: Asset Frozen (r:1 w:1)
    fn unfreeze() -> Weight {
        // Minimum execution time: 46_366 nanoseconds.
        Weight::from_ref_time(47_007_000)
            .saturating_add(DbWeight::get().reads(6))
            .saturating_add(DbWeight::get().writes(1))
    }
    // Storage: Asset Tokens (r:1 w:0)
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset AssetNames (r:0 w:1)
    /// The range of component `n` is `[1, 128]`.
    fn rename_asset(n: u32) -> Weight {
        // Minimum execution time: 43_360 nanoseconds.
        Weight::from_ref_time(44_313_406)
            // Standard Error: 924
            .saturating_add(Weight::from_ref_time(14_448).saturating_mul(n.into()))
            .saturating_add(DbWeight::get().reads(5))
            .saturating_add(DbWeight::get().writes(1))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Portfolio PortfolioCustodian (r:1 w:0)
    // Storage: Asset Tokens (r:1 w:1)
    // Storage: Asset BalanceOf (r:1 w:1)
    // Storage: Portfolio PortfolioAssetBalances (r:1 w:1)
    // Storage: ProtocolFee Coefficient (r:1 w:0)
    // Storage: ProtocolFee BaseFees (r:1 w:0)
    // Storage: Checkpoint Schedules (r:1 w:0)
    // Storage: Checkpoint CheckpointIdSequence (r:1 w:0)
    // Storage: Asset DisableInvestorUniqueness (r:1 w:0)
    // Storage: Asset ScopeIdOf (r:1 w:0)
    // Storage: Statistics ActiveAssetStats (r:1 w:0)
    // Storage: Asset FundingRound (r:1 w:0)
    // Storage: Asset IssuedInFundingRound (r:1 w:1)
    fn issue() -> Weight {
        // Minimum execution time: 93_723 nanoseconds.
        Weight::from_ref_time(95_467_000)
            .saturating_add(DbWeight::get().reads(17))
            .saturating_add(DbWeight::get().writes(4))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Portfolio PortfolioCustodian (r:1 w:0)
    // Storage: Asset Tokens (r:1 w:1)
    // Storage: Portfolio PortfolioAssetBalances (r:1 w:1)
    // Storage: Portfolio PortfolioLockedAssets (r:1 w:0)
    // Storage: Asset BalanceOf (r:1 w:1)
    // Storage: Checkpoint Schedules (r:1 w:0)
    // Storage: Checkpoint CheckpointIdSequence (r:1 w:0)
    // Storage: Asset DisableInvestorUniqueness (r:1 w:0)
    // Storage: Asset ScopeIdOf (r:1 w:0)
    // Storage: Asset AggregateBalance (r:1 w:1)
    // Storage: Statistics ActiveAssetStats (r:1 w:0)
    // Storage: Asset BalanceOfAtScope (r:0 w:1)
    fn redeem() -> Weight {
        // Minimum execution time: 90_156 nanoseconds.
        Weight::from_ref_time(90_698_000)
            .saturating_add(DbWeight::get().reads(15))
            .saturating_add(DbWeight::get().writes(5))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset Tokens (r:1 w:1)
    fn make_divisible() -> Weight {
        // Minimum execution time: 43_921 nanoseconds.
        Weight::from_ref_time(43_961_000)
            .saturating_add(DbWeight::get().reads(5))
            .saturating_add(DbWeight::get().writes(1))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset AssetDocumentsIdSequence (r:1 w:1)
    // Storage: ProtocolFee Coefficient (r:1 w:0)
    // Storage: ProtocolFee BaseFees (r:1 w:0)
    // Storage: Asset AssetDocuments (r:0 w:1)
    /// The range of component `d` is `[1, 64]`.
    fn add_documents(d: u32) -> Weight {
        // Minimum execution time: 56_915 nanoseconds.
        Weight::from_ref_time(48_762_600)
            // Standard Error: 18_514
            .saturating_add(Weight::from_ref_time(10_231_354).saturating_mul(d.into()))
            .saturating_add(DbWeight::get().reads(7))
            .saturating_add(DbWeight::get().writes(1))
            .saturating_add(DbWeight::get().writes((1_u64).saturating_mul(d.into())))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset AssetDocuments (r:0 w:1)
    /// The range of component `d` is `[1, 64]`.
    fn remove_documents(d: u32) -> Weight {
        // Minimum execution time: 31_147 nanoseconds.
        Weight::from_ref_time(36_167_840)
            // Standard Error: 14_496
            .saturating_add(Weight::from_ref_time(5_179_212).saturating_mul(d.into()))
            .saturating_add(DbWeight::get().reads(4))
            .saturating_add(DbWeight::get().writes((1_u64).saturating_mul(d.into())))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset FundingRound (r:0 w:1)
    /// The range of component `f` is `[1, 128]`.
    fn set_funding_round(f: u32) -> Weight {
        // Minimum execution time: 38_311 nanoseconds.
        Weight::from_ref_time(41_787_332)
            // Standard Error: 7_975
            .saturating_add(Weight::from_ref_time(1_416).saturating_mul(f.into()))
            .saturating_add(DbWeight::get().reads(4))
            .saturating_add(DbWeight::get().writes(1))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset Identifiers (r:0 w:1)
    /// The range of component `i` is `[1, 512]`.
    fn update_identifiers(i: u32) -> Weight {
        // Minimum execution time: 39_563 nanoseconds.
        Weight::from_ref_time(41_840_366)
            // Standard Error: 684
            .saturating_add(Weight::from_ref_time(52_799).saturating_mul(i.into()))
            .saturating_add(DbWeight::get().reads(4))
            .saturating_add(DbWeight::get().writes(1))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Portfolio PortfolioCustodian (r:1 w:0)
    // Storage: Asset Tokens (r:1 w:0)
    // Storage: Asset BalanceOf (r:2 w:2)
    // Storage: Checkpoint Schedules (r:1 w:0)
    // Storage: Checkpoint CheckpointIdSequence (r:1 w:0)
    // Storage: Portfolio PortfolioAssetBalances (r:2 w:2)
    // Storage: Portfolio PortfolioAssetCount (r:1 w:1)
    // Storage: Asset DisableInvestorUniqueness (r:1 w:0)
    // Storage: Asset ScopeIdOf (r:2 w:0)
    // Storage: Asset AggregateBalance (r:2 w:2)
    // Storage: Statistics ActiveAssetStats (r:1 w:0)
    // Storage: Asset BalanceOfAtScope (r:0 w:2)
    fn controller_transfer() -> Weight {
        // Minimum execution time: 111_636 nanoseconds.
        Weight::from_ref_time(112_027_000)
            .saturating_add(DbWeight::get().reads(19))
            .saturating_add(DbWeight::get().writes(9))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: Asset CustomTypesInverse (r:1 w:1)
    // Storage: Asset CustomTypeIdSequence (r:1 w:1)
    // Storage: Asset CustomTypes (r:0 w:1)
    /// The range of component `n` is `[1, 2048]`.
    fn register_custom_asset_type(n: u32) -> Weight {
        // Minimum execution time: 35_405 nanoseconds.
        Weight::from_ref_time(36_930_772)
            // Standard Error: 76
            .saturating_add(Weight::from_ref_time(5_091).saturating_mul(n.into()))
            .saturating_add(DbWeight::get().reads(3))
            .saturating_add(DbWeight::get().writes(3))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset AssetMetadataGlobalKeyToName (r:1 w:0)
    // Storage: Asset AssetMetadataValueDetails (r:1 w:1)
    // Storage: Asset AssetMetadataValues (r:0 w:1)
    fn set_asset_metadata() -> Weight {
        // Minimum execution time: 57_917 nanoseconds.
        Weight::from_ref_time(57_987_000)
            .saturating_add(DbWeight::get().reads(6))
            .saturating_add(DbWeight::get().writes(2))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset AssetMetadataGlobalKeyToName (r:1 w:0)
    // Storage: Asset AssetMetadataValueDetails (r:1 w:1)
    fn set_asset_metadata_details() -> Weight {
        // Minimum execution time: 47_959 nanoseconds.
        Weight::from_ref_time(48_129_000)
            .saturating_add(DbWeight::get().reads(6))
            .saturating_add(DbWeight::get().writes(1))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset AssetMetadataLocalNameToKey (r:1 w:1)
    // Storage: Asset AssetMetadataNextLocalKey (r:1 w:1)
    // Storage: Asset AssetMetadataValueDetails (r:1 w:1)
    // Storage: Asset AssetMetadataValues (r:0 w:1)
    // Storage: Asset AssetMetadataLocalKeyToName (r:0 w:1)
    // Storage: Asset AssetMetadataLocalSpecs (r:0 w:1)
    fn register_and_set_local_asset_metadata() -> Weight {
        // Minimum execution time: 89_555 nanoseconds.
        Weight::from_ref_time(90_587_000)
            .saturating_add(DbWeight::get().reads(7))
            .saturating_add(DbWeight::get().writes(6))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset AssetMetadataLocalNameToKey (r:1 w:1)
    // Storage: Asset AssetMetadataNextLocalKey (r:1 w:1)
    // Storage: Asset AssetMetadataLocalKeyToName (r:0 w:1)
    // Storage: Asset AssetMetadataLocalSpecs (r:0 w:1)
    fn register_asset_metadata_local_type() -> Weight {
        // Minimum execution time: 69_048 nanoseconds.
        Weight::from_ref_time(69_118_000)
            .saturating_add(DbWeight::get().reads(6))
            .saturating_add(DbWeight::get().writes(4))
    }
    // Storage: Asset AssetMetadataGlobalNameToKey (r:1 w:1)
    // Storage: Asset AssetMetadataNextGlobalKey (r:1 w:1)
    // Storage: Asset AssetMetadataGlobalKeyToName (r:0 w:1)
    // Storage: Asset AssetMetadataGlobalSpecs (r:0 w:1)
    fn register_asset_metadata_global_type() -> Weight {
        // Minimum execution time: 44_132 nanoseconds.
        Weight::from_ref_time(45_144_000)
            .saturating_add(DbWeight::get().reads(2))
            .saturating_add(DbWeight::get().writes(4))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Portfolio PortfolioCustodian (r:1 w:0)
    // Storage: Asset Tokens (r:1 w:1)
    // Storage: Portfolio Portfolios (r:1 w:0)
    // Storage: Portfolio PortfolioAssetBalances (r:1 w:1)
    // Storage: Portfolio PortfolioLockedAssets (r:1 w:0)
    // Storage: Portfolio PortfolioAssetCount (r:1 w:1)
    // Storage: Asset BalanceOf (r:1 w:1)
    // Storage: Checkpoint Schedules (r:1 w:0)
    // Storage: Checkpoint CheckpointIdSequence (r:1 w:0)
    // Storage: Asset DisableInvestorUniqueness (r:1 w:0)
    // Storage: Asset ScopeIdOf (r:1 w:0)
    // Storage: Asset AggregateBalance (r:1 w:1)
    // Storage: Statistics ActiveAssetStats (r:1 w:0)
    // Storage: Asset BalanceOfAtScope (r:0 w:1)
    fn redeem_from_portfolio() -> Weight {
        // Minimum execution time: 100_245 nanoseconds.
        Weight::from_ref_time(101_537_000)
            .saturating_add(DbWeight::get().reads(17))
            .saturating_add(DbWeight::get().writes(6))
    }
    // Storage: Asset Tokens (r:1 w:1)
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    fn update_asset_type() -> Weight {
        // Minimum execution time: 46_175 nanoseconds.
        Weight::from_ref_time(47_277_000)
            .saturating_add(DbWeight::get().reads(5))
            .saturating_add(DbWeight::get().writes(1))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset AssetMetadataLocalKeyToName (r:1 w:1)
    // Storage: Asset AssetMetadataValueDetails (r:1 w:1)
    // Storage: NFT CollectionTicker (r:1 w:0)
    // Storage: NFT CollectionKeys (r:1 w:0)
    // Storage: Asset AssetMetadataValues (r:0 w:1)
    // Storage: Asset AssetMetadataLocalNameToKey (r:0 w:1)
    // Storage: Asset AssetMetadataLocalSpecs (r:0 w:1)
    fn remove_local_metadata_key() -> Weight {
        // Minimum execution time: 92_515 nanoseconds.
        Weight::from_ref_time(94_600_000)
            .saturating_add(DbWeight::get().reads(8))
            .saturating_add(DbWeight::get().writes(5))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: ExternalAgents GroupOfAgent (r:1 w:0)
    // Storage: Permissions CurrentPalletName (r:1 w:0)
    // Storage: Permissions CurrentDispatchableName (r:1 w:0)
    // Storage: Asset AssetMetadataLocalKeyToName (r:1 w:0)
    // Storage: Asset AssetMetadataValueDetails (r:1 w:1)
    // Storage: Asset AssetMetadataValues (r:0 w:1)
    fn remove_metadata_value() -> Weight {
        // Minimum execution time: 71_999 nanoseconds.
        Weight::from_ref_time(72_720_000)
            .saturating_add(DbWeight::get().reads(6))
            .saturating_add(DbWeight::get().writes(2))
    }
    // Storage: Asset Frozen (r:1 w:0)
    // Storage: Asset DisableInvestorUniqueness (r:1 w:0)
    // Storage: Portfolio Portfolios (r:2 w:0)
    // Storage: Asset Tokens (r:1 w:0)
    // Storage: Portfolio PortfolioAssetBalances (r:2 w:2)
    // Storage: Portfolio PortfolioLockedAssets (r:1 w:0)
    // Storage: Asset AggregateBalance (r:2 w:2)
    // Storage: Statistics AssetTransferCompliances (r:1 w:0)
    // Storage: ComplianceManager AssetCompliances (r:1 w:0)
    // Storage: Asset BalanceOf (r:2 w:2)
    // Storage: Checkpoint Schedules (r:1 w:0)
    // Storage: Checkpoint CheckpointIdSequence (r:1 w:0)
    // Storage: Portfolio PortfolioAssetCount (r:1 w:1)
    // Storage: Statistics ActiveAssetStats (r:1 w:0)
    // Storage: Asset BalanceOfAtScope (r:0 w:2)
    fn base_transfer() -> Weight {
        // Minimum execution time: 177_578 nanoseconds.
        Weight::from_ref_time(186_484_000)
            .saturating_add(DbWeight::get().reads(18))
            .saturating_add(DbWeight::get().writes(9))
    }
    // Storage: Asset TickersExemptFromAffirmation (r:0 w:1)
    fn exempt_ticker_affirmation() -> Weight {
        // Minimum execution time: 12_064 nanoseconds.
        Weight::from_ref_time(12_364_000).saturating_add(DbWeight::get().writes(1))
    }
    // Storage: Asset TickersExemptFromAffirmation (r:0 w:1)
    fn remove_ticker_affirmation_exemption() -> Weight {
        // Minimum execution time: 12_590 nanoseconds.
        Weight::from_ref_time(12_833_000).saturating_add(DbWeight::get().writes(1))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: Asset PreApprovedTicker (r:0 w:1)
    fn pre_approve_ticker() -> Weight {
        // Minimum execution time: 27_396 nanoseconds.
        Weight::from_ref_time(27_794_000)
            .saturating_add(DbWeight::get().reads(1))
            .saturating_add(DbWeight::get().writes(1))
    }
    // Storage: Identity KeyRecords (r:1 w:0)
    // Storage: Asset PreApprovedTicker (r:0 w:1)
    fn remove_ticker_pre_approval() -> Weight {
        // Minimum execution time: 27_779 nanoseconds.
        Weight::from_ref_time(27_958_000)
            .saturating_add(DbWeight::get().reads(1))
            .saturating_add(DbWeight::get().writes(1))
    }
}
