// This file is part of the Polymesh distribution (https://github.com/PolymeshAssociation/Polymesh).
// Copyright (c) 2020 Polymesh Association

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, version 3.

// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

pub use frame_benchmarking::{account, benchmarks};
use frame_support::traits::Get;
use frame_system::RawOrigin;
use scale_info::prelude::format;
use sp_core::sr25519::Signature;
use sp_runtime::MultiSignature;
use sp_std::prelude::*;

use pallet_asset::benchmarking::setup_asset_transfer;
use pallet_nft::benchmarking::setup_nft_transfer;
use polymesh_common_utilities::benchs::{make_asset, AccountIdOf, User, UserBuilder};
use polymesh_common_utilities::constants::currency::ONE_UNIT;
use polymesh_common_utilities::constants::ENSURED_MAX_LEN;
use polymesh_common_utilities::TestUtilsFn;
use polymesh_primitives::checked_inc::CheckedInc;
use polymesh_primitives::settlement::ReceiptMetadata;
use polymesh_primitives::{IdentityId, Memo, NFTId, NFTs, PortfolioId, Ticker};

use crate::*;

const MAX_VENUE_DETAILS_LENGTH: u32 = ENSURED_MAX_LEN;
const MAX_SIGNERS_ALLOWED: u32 = 50;
const MAX_VENUE_ALLOWED: u32 = 100;

#[derive(Encode, Decode, Clone, Copy)]
pub struct UserData<T: Config> {
    pub account: T::AccountId,
    pub did: IdentityId,
}

impl<T: Config> From<&User<T>> for UserData<T> {
    fn from(user: &User<T>) -> Self {
        Self {
            account: user.account(),
            did: user.did(),
        }
    }
}

pub struct Parameters {
    pub legs: Vec<Leg>,
    pub portfolios: Portfolios,
}

#[derive(Default)]
pub struct Portfolios {
    pub sdr_portfolios: Vec<PortfolioId>,
    pub sdr_receipt_portfolios: Vec<PortfolioId>,
    pub rcv_portfolios: Vec<PortfolioId>,
    pub rcv_receipt_portfolios: Vec<PortfolioId>,
}

fn creator<T: Config + TestUtilsFn<AccountIdOf<T>>>() -> User<T> {
    UserBuilder::<T>::default().generate_did().build("creator")
}

/// Set venue related storage without any sanity checks.
fn create_venue_<T: Config>(did: IdentityId, signers: Vec<T::AccountId>) -> VenueId {
    let venue = Venue {
        creator: did,
        venue_type: VenueType::Distribution,
    };
    // NB: Venue counter starts with 1.
    let venue_counter = Module::<T>::venue_counter();
    VenueInfo::insert(venue_counter, venue);
    for signer in signers {
        <VenueSigners<T>>::insert(venue_counter, signer, true);
    }
    VenueCounter::put(venue_counter.checked_inc().unwrap());
    venue_counter
}

pub fn create_asset_<T: Config>(owner: &User<T>) -> Ticker {
    make_asset::<T>(owner, Some(&Ticker::generate(8u64)))
}

fn setup_legs<T>(
    sender: &User<T>,
    receiver: &User<T>,
    f: u32,
    n: u32,
    o: u32,
    pause_compliance: bool,
    pause_restrictions: bool,
) -> Parameters
where
    T: Config + TestUtilsFn<AccountIdOf<T>>,
{
    let mut portfolios = Portfolios::default();

    // Creates offchain legs and new portfolios for each leg
    let offchain_legs: Vec<Leg> = (0..o)
        .map(|i| {
            let ticker = Ticker::from_slice_truncated(format!("OFFTicker{}", i).as_bytes());
            Leg::OffChain {
                sender_identity: sender.did(),
                receiver_identity: receiver.did(),
                ticker: ticker.clone(),
                amount: ONE_UNIT,
            }
        })
        .collect();

    // Creates f assets, creates two portfolios, adds maximum compliance requirements, adds maximum transfer conditions and pauses them
    let fungible_legs: Vec<Leg> = (0..f)
        .map(|i| {
            let ticker = Ticker::from_slice_truncated(format!("Ticker{}", i).as_bytes());
            let sdr_portfolio_name = format!("SdrPortfolioTicker{}", i);
            let rcv_portfolio_name = format!("RcvPortfolioTicker{}", i);
            let (sdr_portfolio, rvc_portfolio) = setup_asset_transfer(
                sender,
                receiver,
                ticker,
                Some(&sdr_portfolio_name),
                Some(&rcv_portfolio_name),
                pause_compliance,
                pause_restrictions,
            );
            portfolios.sdr_portfolios.push(sdr_portfolio.clone());
            portfolios.rcv_portfolios.push(rvc_portfolio.clone());
            Leg::Fungible {
                sender: sdr_portfolio,
                receiver: rvc_portfolio,
                ticker,
                amount: ONE_UNIT,
            }
        })
        .collect();

    // Creates n collections, mints one NFT, creates two portfolios, adds maximum compliance requirements and pauses it
    let nft_legs: Vec<Leg> = (0..n)
        .map(|i| {
            let ticker = Ticker::from_slice_truncated(format!("NFTTicker{}", i).as_bytes());
            let sdr_portfolio_name = format!("SdrPortfolioNFTTicker{}", i);
            let rcv_portfolio_name = format!("RcvPortfolioNFTTicker{}", i);
            let (sdr_portfolio, rcv_portfolio) = setup_nft_transfer(
                sender,
                receiver,
                ticker,
                1,
                Some(&sdr_portfolio_name),
                Some(&rcv_portfolio_name),
                pause_compliance,
            );
            portfolios.sdr_portfolios.push(sdr_portfolio.clone());
            portfolios.rcv_portfolios.push(rcv_portfolio.clone());
            Leg::NonFungible {
                sender: sdr_portfolio,
                receiver: rcv_portfolio,
                nfts: NFTs::new_unverified(ticker, vec![NFTId(1)]),
            }
        })
        .collect();

    Parameters {
        legs: [offchain_legs, fungible_legs, nft_legs].concat(),
        portfolios,
    }
}

/// Creates and affirms an instruction with `f` fungible legs, `n` non-fungible legs and `o` offchain legs.
/// All legs have unique tickers, use custom portfolios, and have the maximum number of compliance requirements,
/// active statistics and transfer restrictions,
fn setup_execute_instruction<T>(
    sender: &User<T>,
    receiver: &User<T>,
    settlement_type: SettlementType<T::BlockNumber>,
    venue_id: VenueId,
    f: u32,
    n: u32,
    o: u32,
    pause_compliance: bool,
    pause_restrictions: bool,
) -> Parameters
where
    T: Config + TestUtilsFn<AccountIdOf<T>>,
{
    // Creates the instruction. All assets, collections, portfolios and rules are created here.
    let parameters = setup_legs::<T>(
        sender,
        receiver,
        f,
        n,
        o,
        pause_compliance,
        pause_restrictions,
    );
    Module::<T>::add_instruction(
        sender.origin.clone().into(),
        venue_id,
        settlement_type,
        None,
        None,
        parameters.legs.clone(),
        Some(Memo::default()),
    )
    .unwrap();
    // Affirms the sender side of the instruction
    let receipt_details: Vec<_> = (0..o)
        .map(|i| {
            setup_receipt_details(
                &sender,
                sender.did(),
                receiver.did(),
                ONE_UNIT,
                InstructionId(1),
                i,
            )
        })
        .collect();
    let sdr_portfolios = [
        parameters.portfolios.sdr_portfolios.clone(),
        parameters.portfolios.sdr_receipt_portfolios.clone(),
    ]
    .concat();
    Module::<T>::affirm_with_receipts(
        sender.origin.clone().into(),
        InstructionId(1),
        receipt_details.clone(),
        sdr_portfolios,
    )
    .unwrap();
    let rcv_portfolios = [
        parameters.portfolios.rcv_portfolios.clone(),
        parameters.portfolios.rcv_receipt_portfolios.clone(),
    ]
    .concat();
    Module::<T>::affirm_with_receipts(
        receiver.origin.clone().into(),
        InstructionId(1),
        Vec::new(),
        rcv_portfolios,
    )
    .unwrap();

    parameters
}

/// Returns the receipt details, signed by `signer`, of a transfer of `amount` for `format!("OFFTicker{}", leg_id).
fn setup_receipt_details<T: Config>(
    signer: &User<T>,
    sender_identity: IdentityId,
    receiver_identity: IdentityId,
    amount: Balance,
    instruction_id: InstructionId,
    leg_id: u32,
) -> ReceiptDetails<T::AccountId, T::OffChainSignature> {
    let ticker = Ticker::from_slice_truncated(format!("OFFTicker{}", leg_id).as_bytes());
    let receipt = Receipt::new(
        leg_id as u64,
        instruction_id,
        LegId(leg_id as u64),
        sender_identity,
        receiver_identity,
        ticker,
        amount,
    );
    let raw_signature: [u8; 64] = signer.sign(&receipt.encode()).unwrap().0;
    let encoded_signature = MultiSignature::from(Signature::from_raw(raw_signature)).encode();
    let signature = T::OffChainSignature::decode(&mut &encoded_signature[..]).unwrap();
    ReceiptDetails::new(
        leg_id as u64,
        instruction_id,
        LegId(leg_id as u64),
        signer.account(),
        signature,
        Some(ReceiptMetadata::default()),
    )
}

benchmarks! {
    where_clause { where T: TestUtilsFn<AccountIdOf<T>>, T: pallet_scheduler::Config }

    create_venue {
        // Variations for the venue_details length.
        let d in 1 .. MAX_VENUE_DETAILS_LENGTH;
        // Variations for the no. of signers allowed.
        let s in 0 .. MAX_SIGNERS_ALLOWED;
        let mut signers = Vec::with_capacity(s as usize);
        let User {origin, did, .. } = UserBuilder::<T>::default().generate_did().build("caller");
        let venue_details = VenueDetails::from(vec![b'D'; d as usize].as_slice());
        let venue_type = VenueType::Distribution;
        // Create signers vector.
        for signer in 0 .. s {
            signers.push(UserBuilder::<T>::default().generate_did().seed(signer).build("signers").account());
        }
    }: _(origin, venue_details, signers, venue_type)
    verify {
        assert_eq!(Module::<T>::venue_counter(), VenueId(2), "Invalid venue counter");
        assert!(UserVenues::contains_key(did.unwrap(), VenueId(1)), "Invalid venue id");
        assert!(Module::<T>::venue_info(VenueId(1)).is_some(), "Incorrect venue info set");
    }

    update_venue_details {
        // Variations for the venue_details length.
        let d in 1 .. MAX_VENUE_DETAILS_LENGTH;
        let details1 = VenueDetails::from(vec![b'D'; d as usize].as_slice());
        let details2 = details1.clone();

        let User { origin, did, .. } = creator::<T>();
        let venue_id = create_venue_::<T>(did.unwrap(), vec![]);
    }: _(origin, venue_id, details1)
    verify {
        assert_eq!(Module::<T>::details(venue_id), details2, "Incorrect venue details");
    }

    update_venue_type {
        let ty = VenueType::Sto;

        let User { account, origin, did, .. } = creator::<T>();
        let venue_id = create_venue_::<T>(did.unwrap(), vec![]);
    }: _(origin, venue_id, ty)
    verify {
        assert_eq!(Module::<T>::venue_info(VenueId(1)).unwrap().venue_type, ty, "Incorrect venue type value");
    }

    update_venue_signers {
        // Variations for the no. of signers allowed.
        let s in 0 .. MAX_SIGNERS_ALLOWED;
        let mut signers = Vec::with_capacity(s as usize);
        let User {account, origin, did, .. } = creator::<T>();
        let venue_id = create_venue_::<T>(did.unwrap(), vec![account.clone()]);
        // Create signers vector.
        for signer in 0 .. s {
            signers.push(UserBuilder::<T>::default().generate_did().seed(signer).build("signers").account());
        }
    }: _(origin, venue_id, signers.clone(), true)
    verify {
        for signer in signers.iter() {
            assert_eq!(Module::<T>::venue_signers(venue_id, signer), true, "Incorrect venue signer");
        }
    }

    set_venue_filtering {
        // Constant time function. It is only for allow venue filtering.
        let user = creator::<T>();
        let ticker = create_asset_::<T>(&user);
    }: _(user.origin, ticker, true)
    verify {
        assert!(Module::<T>::venue_filtering(ticker), "Fail: set_venue_filtering failed");
    }

    allow_venues {
        // Count of venue is variant for this dispatchable.
        let v in 0 .. MAX_VENUE_ALLOWED;
        let user = creator::<T>();
        let ticker = create_asset_::<T>(&user);
        let mut venues = Vec::new();
        for i in 0 .. v {
            venues.push(VenueId(i.into()));
        }
        let s_venues = venues.clone();
    }: _(user.origin, ticker, s_venues)
    verify {
        for v in venues.iter() {
            assert!(Module::<T>::venue_allow_list(ticker, v), "Fail: allow_venue dispatch");
        }
    }

    disallow_venues {
        // Count of venue is variant for this dispatchable.
        let v in 0 .. MAX_VENUE_ALLOWED;
        let user = creator::<T>();
        let ticker = create_asset_::<T>(&user);
        let mut venues = Vec::new();
        for i in 0 .. v {
            venues.push(VenueId(i.into()));
        }
        let s_venues = venues.clone();
    }: _(user.origin, ticker, s_venues)
    verify {
        for v in venues.iter() {
            assert!(!Module::<T>::venue_allow_list(ticker, v), "Fail: allow_venue dispatch");
        }
    }

    affirm_with_receipts {
        // Number of fungible, non fungible and offchain assets
        let f in 1..T::MaxNumberOfFungibleAssets::get();
        let n in 0..T::MaxNumberOfNFTs::get();
        let o in 0..T::MaxNumberOfOffChainAssets::get();

        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let venue_id = create_venue_::<T>(alice.did(), vec![alice.account()]);

        let parameters = setup_legs::<T>(&alice, &bob, f, n, o, false, false);
        Module::<T>::add_instruction(
            alice.origin.clone().into(),
            venue_id,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            parameters.legs,
            Some(Memo::default()),
        ).unwrap();

        let receipt_details = (0..o)
            .map(|i| {
                setup_receipt_details(
                    &alice,
                    alice.did(),
                    bob.did(),
                    ONE_UNIT,
                    InstructionId(1),
                    i
                )
            })
            .collect();
        let portfolios =
            [parameters.portfolios.sdr_portfolios, parameters.portfolios.sdr_receipt_portfolios].concat();
    }: _(alice.origin, InstructionId(1), receipt_details, portfolios)

    execute_manual_instruction {
        // Number of fungible, non-fungible and offchain assets in the instruction
        let f in 1..T::MaxNumberOfFungibleAssets::get();
        let n in 0..T::MaxNumberOfNFTs::get();
        let o in 0..T::MaxNumberOfOffChainAssets::get();

        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let settlement_type = SettlementType::SettleManual(0u32.into());
        let venue_id = create_venue_::<T>(alice.did(), vec![alice.account(), bob.account()]);

        setup_execute_instruction::<T>(&alice, &bob, settlement_type, venue_id, f, n, o, false, false);
    }: _(alice.origin, InstructionId(1), None, f, n, o, Some(Weight::MAX))

    add_instruction{
        // Number of fungible, non-fungible and offchain LEGS in the instruction
        let f in 1..T::MaxNumberOfFungibleAssets::get();
        let n in 0..T::MaxNumberOfNFTs::get();
        let o in 0..T::MaxNumberOfOffChainAssets::get();

        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let memo = Some(Memo::default());
        let settlement_type = SettlementType::SettleOnBlock(100u32.into());
        let venue_id = create_venue_::<T>(alice.did(), vec![alice.account()]);

        let parameters = setup_legs::<T>(&alice, &bob, f, n, o, false, false);
    }: _(alice.origin, venue_id, settlement_type, None, None, parameters.legs, memo)

    add_and_affirm_instruction {
        // Number of fungible, non-fungible and offchain LEGS in the instruction
        let f in 1..T::MaxNumberOfFungibleAssets::get();
        let n in 0..T::MaxNumberOfNFTs::get();
        let o in 0..T::MaxNumberOfOffChainAssets::get();

        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let memo = Some(Memo::default());
        let settlement_type = SettlementType::SettleOnBlock(100u32.into());
        let venue_id = create_venue_::<T>(alice.did(), vec![alice.account()]);

        let parameters = setup_legs::<T>(&alice, &bob, f, n, o, false, false);
    }: _(alice.origin, venue_id, settlement_type, None, None, parameters.legs, parameters.portfolios.sdr_portfolios, memo)

    affirm_instruction {
        // Number of fungible and non-fungible assets in the portfolios
        let f in 1..T::MaxNumberOfFungibleAssets::get() as u32;
        let n in 1..T::MaxNumberOfNFTs::get() as u32;

        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let memo = Some(Memo::default());
        let venue_id = create_venue_::<T>(alice.did(), vec![alice.account()]);

        let parameters = setup_legs::<T>(&alice, &bob, f, n, T::MaxNumberOfOffChainAssets::get(), false, false);
        Module::<T>::add_instruction(
            alice.origin.clone().into(),
            venue_id,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            parameters.legs,
            memo,
        ).unwrap();
    }: _(alice.origin, InstructionId(1), parameters.portfolios.sdr_portfolios)

    withdraw_affirmation {
        // Number of fungible, non-fungible and offchain LEGS in the portfolios
        let f in 1..T::MaxNumberOfFungibleAssets::get();
        let n in 0..T::MaxNumberOfNFTs::get();
        let o in 0..T::MaxNumberOfOffChainAssets::get();

        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let settlement_type = SettlementType::SettleOnBlock(100u32.into());
        let venue_id = create_venue_::<T>(alice.did(), vec![alice.account(), bob.account()]);

        let parameters = setup_execute_instruction::<T>(&alice, &bob, settlement_type, venue_id, f, n, o, false, false);
        let portfolios =
            [parameters.portfolios.sdr_portfolios, parameters.portfolios.sdr_receipt_portfolios].concat();
    }: _(alice.origin, InstructionId(1),  portfolios)

    reject_instruction {
        // Number of fungible, non-fungible and offchain LEGS in the instruction
        let f in 1..T::MaxNumberOfFungibleAssets::get();
        let n in 0..T::MaxNumberOfNFTs::get();
        let o in 0..T::MaxNumberOfOffChainAssets::get();

        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let settlement_type = SettlementType::SettleOnBlock(100u32.into());
        let venue_id = create_venue_::<T>(alice.did(), vec![alice.account(), bob.account()]);

        let parameters = setup_execute_instruction::<T>(&alice, &bob, settlement_type, venue_id, f, n, o, false, false);
        let portfolios =
            [parameters.portfolios.sdr_portfolios.clone(), parameters.portfolios.sdr_receipt_portfolios].concat();
    }: _(alice.origin, InstructionId(1), parameters.portfolios.sdr_portfolios[0])

    execute_instruction_paused {
        // Number of fungible, non-fungible and offchain assets in the instruction
        let f in 1..T::MaxNumberOfFungibleAssets::get() as u32;
        let n in 0..T::MaxNumberOfNFTs::get() as u32;
        let o in 0..T::MaxNumberOfOffChainAssets::get();

        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let venue_id = create_venue_::<T>(alice.did(), vec![alice.account(), bob.account()]);

        setup_execute_instruction::<T>(&alice, &bob, SettlementType::SettleOnAffirmation, venue_id, f, n, o, true, true);
    }: execute_scheduled_instruction(RawOrigin::Root, InstructionId(1), Weight::MAX)

    execute_scheduled_instruction {
        // Number of fungible, non-fungible and offchain assets in the instruction
        let f in 1..T::MaxNumberOfFungibleAssets::get() as u32;
        let n in 0..T::MaxNumberOfNFTs::get() as u32;
        let o in 0..T::MaxNumberOfOffChainAssets::get();

        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let venue_id = create_venue_::<T>(alice.did(), vec![alice.account(), bob.account()]);

        setup_execute_instruction::<T>(&alice, &bob, SettlementType::SettleOnAffirmation, venue_id, f, n, o, false, false);
    }: _(RawOrigin::Root, InstructionId(1), Weight::MAX)

    ensure_root_origin {
        let origin = RawOrigin::Root;
    }: {
        assert!(Module::<T>::ensure_root_origin(origin.into()).is_ok());
    }

    affirm_with_receipts_rcv {
        // Number of fungible, non fungible and offchain assets
        let f in 1..T::MaxNumberOfFungibleAssets::get();
        let n in 0..T::MaxNumberOfNFTs::get();
        let o in 0..T::MaxNumberOfOffChainAssets::get();

        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let venue_id = create_venue_::<T>(alice.did(), vec![alice.account()]);

        let parameters = setup_legs::<T>(&alice, &bob, f, n, o, false, false);
        Module::<T>::add_instruction(
            alice.origin.clone().into(),
            venue_id,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            parameters.legs,
            Some(Memo::default()),
        ).unwrap();

        let receipt_details = (0..o)
            .map(|i| {
                setup_receipt_details(
                    &alice,
                    alice.did(),
                    bob.did(),
                    ONE_UNIT,
                    InstructionId(1),
                    i
                )
            })
            .collect();
        let portfolios =
            [parameters.portfolios.rcv_portfolios, parameters.portfolios.rcv_receipt_portfolios].concat();
    }: affirm_with_receipts(bob.origin, InstructionId(1), receipt_details, portfolios)

    affirm_instruction_rcv {
        // Number of fungible and non-fungible assets in the portfolios
        let f in 1..T::MaxNumberOfFungibleAssets::get() as u32;
        let n in 1..T::MaxNumberOfNFTs::get() as u32;

        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let memo = Some(Memo::default());
        let venue_id = create_venue_::<T>(alice.did(), vec![alice.account()]);

        let parameters = setup_legs::<T>(&alice, &bob, f, n, T::MaxNumberOfOffChainAssets::get(), false, false);
        Module::<T>::add_instruction(
            alice.origin.clone().into(),
            venue_id,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            parameters.legs,
            memo,
        ).unwrap();
    }: affirm_instruction(bob.origin, InstructionId(1), parameters.portfolios.rcv_portfolios)

    withdraw_affirmation_rcv {
        // Number of fungible, non-fungible and offchain LEGS in the portfolios
        let f in 1..T::MaxNumberOfFungibleAssets::get();
        let n in 0..T::MaxNumberOfNFTs::get();
        let o in 0..T::MaxNumberOfOffChainAssets::get();

        let alice = UserBuilder::<T>::default().generate_did().build("Alice");
        let bob = UserBuilder::<T>::default().generate_did().build("Bob");
        let settlement_type = SettlementType::SettleOnBlock(100u32.into());
        let venue_id = create_venue_::<T>(alice.did(), vec![alice.account(), bob.account()]);

        let parameters = setup_execute_instruction::<T>(&alice, &bob, settlement_type, venue_id, f, n, o, false, false);
        let portfolios =
            [parameters.portfolios.rcv_portfolios, parameters.portfolios.rcv_receipt_portfolios].concat();
    }: withdraw_affirmation(bob.origin, InstructionId(1),  portfolios)
}
