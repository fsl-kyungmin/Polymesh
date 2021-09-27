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

//! Autogenerated weights for pallet_scheduler
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-09-22, STEPS: [100, ], REPEAT: 5, LOW RANGE: [], HIGH RANGE: []
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 512

// Executed Command:
// ./target/release/polymesh
// benchmark
// -s
// 100
// -r
// 5
// -p=pallet_scheduler
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
// --raw

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{
    traits::Get,
    weights::{constants::RocksDbWeight as DbWeight, Weight},
};

/// Weights for pallet_scheduler using the Substrate node and recommended hardware.
pub struct WeightInfo;
impl pallet_scheduler::WeightInfo for WeightInfo {
    fn schedule(s: u32) -> Weight {
        (36_189_000 as Weight)
            // Standard Error: 20_000
            .saturating_add((132_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(DbWeight::get().reads(1 as Weight))
            .saturating_add(DbWeight::get().writes(1 as Weight))
    }
    fn cancel(s: u32) -> Weight {
        (30_387_000 as Weight)
            // Standard Error: 61_000
            .saturating_add((4_517_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(DbWeight::get().reads(1 as Weight))
            .saturating_add(DbWeight::get().writes(2 as Weight))
    }
    fn schedule_named(s: u32) -> Weight {
        (42_515_000 as Weight)
            // Standard Error: 21_000
            .saturating_add((157_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(DbWeight::get().reads(2 as Weight))
            .saturating_add(DbWeight::get().writes(2 as Weight))
    }
    fn cancel_named(s: u32) -> Weight {
        (28_063_000 as Weight)
            // Standard Error: 64_000
            .saturating_add((4_476_000 as Weight).saturating_mul(s as Weight))
            .saturating_add(DbWeight::get().reads(2 as Weight))
            .saturating_add(DbWeight::get().writes(2 as Weight))
    }
}
