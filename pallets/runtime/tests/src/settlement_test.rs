use std::collections::HashMap;
use std::convert::TryFrom;
use std::ops::Deref;

use codec::Encode;
use frame_support::dispatch::DispatchErrorWithPostInfo;
use frame_support::{
    assert_err_ignore_postinfo, assert_noop, assert_ok, assert_storage_noop,
    IterableStorageDoubleMap, StorageDoubleMap, StorageMap,
};
use rand::{prelude::*, thread_rng};
use sp_runtime::{AccountId32, AnySignature};
use sp_std::collections::btree_set::BTreeSet;

use pallet_asset::BalanceOf;
use pallet_nft::NumberOfNFTs;
use pallet_portfolio::{PortfolioLockedNFT, PortfolioNFT};
use pallet_scheduler as scheduler;
use pallet_settlement::{
    AffirmsReceived, InstructionAffirmsPending, InstructionLegs, InstructionMemos,
    NumberOfVenueSigners, OffChainAffirmations, RawEvent, UserAffirmations, UserVenues,
    VenueInstructions,
};
use polymesh_common_utilities::constants::currency::ONE_UNIT;
use polymesh_common_utilities::constants::ERC1400_TRANSFER_SUCCESS;
use polymesh_primitives::asset::{AssetType, NonFungibleType};
use polymesh_primitives::asset_metadata::{
    AssetMetadataKey, AssetMetadataLocalKey, AssetMetadataValue,
};
use polymesh_primitives::checked_inc::CheckedInc;
use polymesh_primitives::settlement::{
    AffirmationCount, AffirmationStatus, AssetCount, Instruction, InstructionId, InstructionStatus,
    Leg, LegId, LegStatus, Receipt, ReceiptDetails, SettlementType, VenueDetails, VenueId,
    VenueType,
};
use polymesh_primitives::{
    AccountId, AuthorizationData, Balance, Claim, Condition, ConditionType, Fund, FundDescription,
    IdentityId, Memo, NFTCollectionKeys, NFTId, NFTMetadataAttribute, NFTs, PortfolioId,
    PortfolioKind, PortfolioName, PortfolioNumber, Signatory, Ticker, WeightMeter,
};
use sp_keyring::AccountKeyring;

use super::asset_test::{allow_all_transfers, max_len_bytes};
use super::nft::{create_nft_collection, mint_nft};
use super::storage::{
    default_portfolio_vec, make_account_without_cdd, user_portfolio_vec, TestStorage, User,
};
use super::{next_block, ExtBuilder};

type Identity = pallet_identity::Module<TestStorage>;
type Balances = pallet_balances::Module<TestStorage>;
type Asset = pallet_asset::Module<TestStorage>;
type Portfolio = pallet_portfolio::Module<TestStorage>;
type PortfolioError = pallet_portfolio::Error<TestStorage>;
type Timestamp = pallet_timestamp::Pallet<TestStorage>;
type ComplianceManager = pallet_compliance_manager::Module<TestStorage>;
type AssetError = pallet_asset::Error<TestStorage>;
type OffChainSignature = AnySignature;
type Origin = <TestStorage as frame_system::Config>::RuntimeOrigin;
type Moment = <TestStorage as pallet_timestamp::Config>::Moment;
type BlockNumber = <TestStorage as frame_system::Config>::BlockNumber;
type Settlement = pallet_settlement::Module<TestStorage>;
type System = frame_system::Pallet<TestStorage>;
type Error = pallet_settlement::Error<TestStorage>;
type Scheduler = pallet_scheduler::Pallet<TestStorage>;
type NFTError = pallet_nft::Error<TestStorage>;

const TICKER: Ticker = Ticker::new_unchecked([b'A', b'C', b'M', b'E', 0, 0, 0, 0, 0, 0, 0, 0]);
const TICKER2: Ticker = Ticker::new_unchecked([b'A', b'C', b'M', b'E', b'2', 0, 0, 0, 0, 0, 0, 0]);

macro_rules! assert_add_claim {
    ($signer:expr, $target:expr, $claim:expr) => {
        assert_ok!(Identity::add_claim($signer, $target, $claim, None,));
    };
}

macro_rules! assert_affirm_instruction {
    ($signer:expr, $instruction_id:expr, $did:expr) => {
        assert_ok!(Settlement::affirm_instruction(
            $signer,
            $instruction_id,
            default_portfolio_vec($did),
        ));
    };
}

struct UserWithBalance {
    user: User,
    init_balances: Vec<(Ticker, Balance)>,
}

impl UserWithBalance {
    fn new(acc: AccountKeyring, tickers: &[Ticker]) -> Self {
        let user = User::new(acc);
        Self {
            init_balances: tickers
                .iter()
                .map(|ticker| (*ticker, Asset::balance_of(ticker, user.did)))
                .collect(),
            user,
        }
    }

    fn refresh_init_balances(&mut self) {
        for (ticker, balance) in &mut self.init_balances {
            *balance = Asset::balance_of(ticker, self.user.did);
        }
    }

    #[track_caller]
    fn init_balance(&self, ticker: &Ticker) -> Balance {
        self.init_balances
            .iter()
            .find(|bs| bs.0 == *ticker)
            .unwrap()
            .1
    }

    #[track_caller]
    fn assert_all_balances_unchanged(&self) {
        for (t, balance) in &self.init_balances {
            assert_balance(t, &self.user, *balance);
        }
    }

    #[track_caller]
    fn assert_balance_unchanged(&self, ticker: &Ticker) {
        assert_balance(ticker, &self.user, self.init_balance(ticker));
    }

    #[track_caller]
    fn assert_balance_increased(&self, ticker: &Ticker, amount: Balance) {
        assert_balance(ticker, &self.user, self.init_balance(ticker) + amount);
    }

    #[track_caller]
    fn assert_balance_decreased(&self, ticker: &Ticker, amount: Balance) {
        assert_balance(ticker, &self.user, self.init_balance(ticker) - amount);
    }

    #[track_caller]
    fn assert_portfolio_bal(&self, num: PortfolioNumber, balance: Balance) {
        assert_eq!(
            Portfolio::user_portfolio_balance(self.user.did, num, &TICKER),
            balance,
        );
    }

    #[track_caller]
    fn assert_default_portfolio_bal(&self, balance: Balance) {
        assert_eq!(
            Portfolio::default_portfolio_balance(self.user.did, &TICKER),
            balance,
        );
    }

    #[track_caller]
    fn assert_default_portfolio_bal_unchanged(&self) {
        self.assert_default_portfolio_bal(self.init_balance(&TICKER));
    }

    #[track_caller]
    fn assert_default_portfolio_bal_decreased(&self, amount: Balance) {
        self.assert_default_portfolio_bal(self.init_balance(&TICKER) - amount);
    }

    #[track_caller]
    fn assert_default_portfolio_bal_increased(&self, amount: Balance) {
        self.assert_default_portfolio_bal(self.init_balance(&TICKER) + amount);
    }
}

impl Deref for UserWithBalance {
    type Target = User;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

fn create_token_and_venue(ticker: Ticker, user: User) -> VenueId {
    create_token(ticker, user);
    create_venue(user)
}

fn create_token(ticker: Ticker, user: User) {
    assert_ok!(Asset::create_asset(
        user.origin(),
        ticker.as_slice().into(),
        ticker,
        true,
        AssetType::default(),
        vec![],
        None,
    ));
    assert_ok!(Asset::issue(
        user.origin(),
        ticker,
        100_000,
        PortfolioKind::Default
    ));
    allow_all_transfers(ticker, user);
}

pub(crate) fn create_venue(user: User) -> VenueId {
    let venue_counter = Settlement::venue_counter();
    assert_ok!(Settlement::create_venue(
        user.origin(),
        VenueDetails::default(),
        vec![user.acc()],
        VenueType::Other
    ));
    venue_counter
}

pub fn set_current_block_number(block: u32) {
    System::set_block_number(block);
}

#[test]
fn venue_details_length_limited() {
    ExtBuilder::default().build().execute_with(|| {
        let actor = User::new(AccountKeyring::Alice);
        let id = Settlement::venue_counter();
        let create = |d| Settlement::create_venue(actor.origin(), d, vec![], VenueType::Exchange);
        let update = |d| Settlement::update_venue_details(actor.origin(), id, d);
        assert_too_long!(create(max_len_bytes(1)));
        assert_ok!(create(max_len_bytes(0)));
        assert_too_long!(update(max_len_bytes(1)));
        assert_ok!(update(max_len_bytes(0)));
    });
}

fn venue_instructions(id: VenueId) -> Vec<InstructionId> {
    VenueInstructions::iter_prefix(id).map(|(i, _)| i).collect()
}

fn user_venues(did: IdentityId) -> Vec<VenueId> {
    let mut venues = UserVenues::iter_prefix(did)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    venues.sort();
    venues
}

#[test]
fn venue_registration() {
    ExtBuilder::default().build().execute_with(|| {
        let alice = User::new(AccountKeyring::Alice);
        let venue_counter = Settlement::venue_counter();
        assert_ok!(Settlement::create_venue(
            alice.origin(),
            VenueDetails::default(),
            vec![
                AccountKeyring::Alice.to_account_id(),
                AccountKeyring::Bob.to_account_id()
            ],
            VenueType::Exchange
        ));
        let venue_info = Settlement::venue_info(venue_counter).unwrap();
        assert_eq!(
            Settlement::venue_counter(),
            venue_counter.checked_inc().unwrap()
        );
        assert_eq!(user_venues(alice.did), [venue_counter]);
        assert_eq!(venue_info.creator, alice.did);
        assert_eq!(venue_instructions(venue_counter).len(), 0);
        assert_eq!(Settlement::details(venue_counter), VenueDetails::default());
        assert_eq!(venue_info.venue_type, VenueType::Exchange);
        assert_eq!(Settlement::venue_signers(venue_counter, alice.acc()), true);
        assert_eq!(
            Settlement::venue_signers(venue_counter, AccountKeyring::Bob.to_account_id()),
            true
        );
        assert_eq!(
            Settlement::venue_signers(venue_counter, AccountKeyring::Charlie.to_account_id()),
            false
        );

        // Creating a second venue
        assert_ok!(Settlement::create_venue(
            alice.origin(),
            VenueDetails::default(),
            vec![alice.acc(), AccountKeyring::Bob.to_account_id()],
            VenueType::Exchange
        ));
        assert_eq!(
            user_venues(alice.did),
            [venue_counter, venue_counter.checked_inc().unwrap()]
        );

        // Editing venue details
        assert_ok!(Settlement::update_venue_details(
            alice.origin(),
            venue_counter,
            [0x01].into(),
        ));
        let venue_info = Settlement::venue_info(venue_counter).unwrap();
        assert_eq!(venue_info.creator, alice.did);
        assert_eq!(venue_instructions(venue_counter).len(), 0);
        assert_eq!(Settlement::details(venue_counter), [0x01].into());
        assert_eq!(venue_info.venue_type, VenueType::Exchange);
    });
}

fn test_with_cdd_provider(test: impl FnOnce(AccountId)) {
    let cdd = AccountKeyring::Eve.to_account_id();
    ExtBuilder::default()
        .cdd_providers(vec![cdd.clone()])
        .build()
        .execute_with(|| test(cdd));
}

#[test]
fn basic_settlement() {
    test_with_cdd_provider(|_eve| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER]);
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        let instruction_id = Settlement::instruction_counter();
        let amount = 100u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            vec![Leg::Fungible {
                sender: PortfolioId::default_portfolio(alice.did),
                receiver: PortfolioId::default_portfolio(bob.did),
                ticker: TICKER,
                amount
            }],
            None,
        ));
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();

        assert_affirm_instruction!(alice.origin(), instruction_id, alice.did);

        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        set_current_block_number(5);
        // Instruction get scheduled to next block.
        assert_affirm_instruction!(bob.origin(), instruction_id, bob.did);

        // Advances the block no. to execute the instruction.
        next_block();
        alice.assert_balance_decreased(&TICKER, amount);
        bob.assert_balance_increased(&TICKER, amount);
    });
}

#[test]
fn create_and_affirm_instruction() {
    test_with_cdd_provider(|_eve| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER]);
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        let instruction_id = Settlement::instruction_counter();
        let amount = 100u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        let add_and_affirm_tx = |affirm_from_portfolio| {
            Settlement::add_and_affirm_instruction(
                alice.origin(),
                venue_counter,
                SettlementType::SettleOnAffirmation,
                None,
                None,
                vec![Leg::Fungible {
                    sender: PortfolioId::default_portfolio(alice.did),
                    receiver: PortfolioId::default_portfolio(bob.did),
                    ticker: TICKER,
                    amount,
                }],
                affirm_from_portfolio,
                None,
            )
        };

        // If affirmation fails, the instruction should be rolled back.
        // i.e. this tx should be a no-op.
        assert_noop!(
            add_and_affirm_tx(user_portfolio_vec(alice.did, 1u64.into())),
            Error::UnexpectedAffirmationStatus
        );

        assert_ok!(add_and_affirm_tx(default_portfolio_vec(alice.did)));

        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();

        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Affirmed);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Pending);
        set_current_block_number(5);

        assert_affirm_instruction!(bob.origin(), instruction_id, bob.did);

        // Advances the block no.
        next_block();
        alice.assert_balance_decreased(&TICKER, amount);
        bob.assert_balance_increased(&TICKER, amount);
    });
}

#[test]
fn overdraft_failure() {
    ExtBuilder::default().build().execute_with(|| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER]);
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        let instruction_id = Settlement::instruction_counter();
        let amount = 100_000_000u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            vec![Leg::Fungible {
                sender: PortfolioId::default_portfolio(alice.did),
                receiver: PortfolioId::default_portfolio(bob.did),
                ticker: TICKER,
                amount
            }],
            None,
        ));
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        assert_noop!(
            Settlement::affirm_instruction(
                alice.origin(),
                instruction_id,
                default_portfolio_vec(alice.did),
            ),
            PortfolioError::InsufficientPortfolioBalance
        );
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
    });
}

#[test]
fn token_swap() {
    test_with_cdd_provider(|_eve| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER, TICKER2]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER, TICKER2]);
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        create_token(TICKER2, bob.user);
        let instruction_id = Settlement::instruction_counter();
        let amount = 100u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        let legs = vec![
            Leg::Fungible {
                sender: PortfolioId::default_portfolio(alice.did),
                receiver: PortfolioId::default_portfolio(bob.did),
                ticker: TICKER,
                amount,
            },
            Leg::Fungible {
                sender: PortfolioId::default_portfolio(bob.did),
                receiver: PortfolioId::default_portfolio(alice.did),
                ticker: TICKER2,
                amount,
            },
        ];

        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs.clone(),
            None,
        ));

        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Pending);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Pending);

        for i in 0..legs.len() {
            assert_eq!(
                InstructionLegs::get(&instruction_id, &LegId(i as u64)),
                legs[i].clone().into()
            );
        }

        let instruction_details = Instruction {
            instruction_id,
            venue_id: venue_counter,
            settlement_type: SettlementType::SettleOnAffirmation,
            created_at: Some(Timestamp::get()),
            trade_date: None,
            value_date: None,
        };
        assert_instruction_status(instruction_id, InstructionStatus::Pending);
        assert_instruction_details(instruction_id, instruction_details);

        assert_affirms_pending(instruction_id, 2);
        assert_eq!(venue_instructions(venue_counter), vec![instruction_id]);

        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();

        assert_affirm_instruction!(alice.origin(), instruction_id, alice.did);
        assert_affirms_pending(instruction_id, 1);

        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Affirmed);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Pending);

        assert_leg_status(instruction_id, LegId(0), LegStatus::ExecutionPending);
        assert_leg_status(instruction_id, LegId(1), LegStatus::PendingTokenLock);

        assert_locked_assets(&TICKER, &alice, amount);

        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();

        assert_ok!(Settlement::withdraw_affirmation(
            alice.origin(),
            instruction_id,
            default_portfolio_vec(alice.did),
        ));

        assert_affirms_pending(instruction_id, 2);
        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Pending);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Pending);

        assert_leg_status(instruction_id, LegId(0), LegStatus::PendingTokenLock);
        assert_leg_status(instruction_id, LegId(1), LegStatus::PendingTokenLock);

        assert_locked_assets(&TICKER, &alice, 0);
        assert_affirm_instruction!(alice.origin(), instruction_id, alice.did);

        assert_affirms_pending(instruction_id, 1);
        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Affirmed);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Pending);

        assert_leg_status(instruction_id, LegId(0), LegStatus::ExecutionPending);
        assert_leg_status(instruction_id, LegId(1), LegStatus::PendingTokenLock);

        assert_locked_assets(&TICKER, &alice, amount);

        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        set_current_block_number(500);

        assert_affirm_instruction!(bob.origin(), instruction_id, bob.did);

        next_block();
        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Unknown);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Unknown);
        assert_locked_assets(&TICKER, &alice, 0);
        alice.assert_balance_decreased(&TICKER, amount);
        alice.assert_balance_increased(&TICKER2, amount);
        bob.assert_balance_increased(&TICKER, amount);
        bob.assert_balance_decreased(&TICKER2, amount);
    });
}

#[test]
fn settle_on_block() {
    test_with_cdd_provider(|_eve| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER, TICKER2]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER, TICKER2]);
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        create_token(TICKER2, bob.user);
        let instruction_id = Settlement::instruction_counter();
        let block_number = System::block_number() + 1;
        let amount = 100u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        let legs = vec![
            Leg::Fungible {
                sender: PortfolioId::default_portfolio(alice.did),
                receiver: PortfolioId::default_portfolio(bob.did),
                ticker: TICKER,
                amount,
            },
            Leg::Fungible {
                sender: PortfolioId::default_portfolio(bob.did),
                receiver: PortfolioId::default_portfolio(alice.did),
                ticker: TICKER2,
                amount,
            },
        ];

        assert_eq!(0, scheduler::Agenda::<TestStorage>::get(block_number).len());
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnBlock(block_number),
            None,
            None,
            legs.clone(),
            None,
        ));
        assert_eq!(1, scheduler::Agenda::<TestStorage>::get(block_number).len());

        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Pending);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Pending);

        for i in 0..legs.len() {
            assert_eq!(
                InstructionLegs::get(&instruction_id, &LegId(i as u64)),
                legs[i].clone().into()
            );
        }

        let instruction_details = Instruction {
            instruction_id,
            venue_id: venue_counter,
            settlement_type: SettlementType::SettleOnBlock(block_number),
            created_at: Some(Timestamp::get()),
            trade_date: None,
            value_date: None,
        };
        assert_instruction_status(instruction_id, InstructionStatus::Pending);
        assert_eq!(
            Settlement::instruction_details(instruction_id),
            instruction_details
        );

        assert_affirms_pending(instruction_id, 2);
        assert_eq!(venue_instructions(venue_counter), vec![instruction_id]);

        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();

        assert_affirm_instruction!(alice.origin(), instruction_id, alice.did);

        assert_affirms_pending(instruction_id, 1);
        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Affirmed);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Pending);
        assert_leg_status(instruction_id, LegId(0), LegStatus::ExecutionPending);
        assert_leg_status(instruction_id, LegId(1), LegStatus::PendingTokenLock);
        assert_locked_assets(&TICKER, &alice, amount);

        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();

        assert_affirm_instruction!(bob.origin(), instruction_id, bob.did);

        assert_affirms_pending(instruction_id, 0);
        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Affirmed);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Affirmed);
        assert_leg_status(instruction_id, LegId(0), LegStatus::ExecutionPending);
        assert_leg_status(instruction_id, LegId(1), LegStatus::ExecutionPending);
        assert_locked_assets(&TICKER, &alice, amount);
        assert_locked_assets(&TICKER2, &bob, amount);

        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();

        // Instruction should've settled
        next_block();
        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Unknown);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Unknown);
        assert_locked_assets(&TICKER, &alice, 0);
        assert_locked_assets(&TICKER, &bob, 0);

        alice.assert_balance_decreased(&TICKER, amount);
        bob.assert_balance_increased(&TICKER, amount);
        alice.assert_balance_increased(&TICKER2, amount);
        bob.assert_balance_decreased(&TICKER2, amount);
    });
}

#[test]
fn failed_execution() {
    ExtBuilder::default().build().execute_with(|| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER, TICKER2]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER, TICKER2]);
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        create_token(TICKER2, bob.user);
        let instruction_id = Settlement::instruction_counter();
        assert_ok!(ComplianceManager::reset_asset_compliance(
            Origin::signed(AccountKeyring::Bob.to_account_id()),
            TICKER2,
        ));
        let block_number = System::block_number() + 1;
        let amount = 100u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        let legs = vec![
            Leg::Fungible {
                sender: PortfolioId::default_portfolio(alice.did),
                receiver: PortfolioId::default_portfolio(bob.did),
                ticker: TICKER,
                amount,
            },
            Leg::Fungible {
                sender: PortfolioId::default_portfolio(bob.did),
                receiver: PortfolioId::default_portfolio(alice.did),
                ticker: TICKER2,
                amount,
            },
        ];

        assert_eq!(0, scheduler::Agenda::<TestStorage>::get(block_number).len());
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnBlock(block_number),
            None,
            None,
            legs.clone(),
            None,
        ));
        assert_eq!(1, scheduler::Agenda::<TestStorage>::get(block_number).len());

        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Pending);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Pending);

        for i in 0..legs.len() {
            assert_eq!(
                InstructionLegs::get(&instruction_id, &LegId(i as u64)),
                legs[i].clone().into()
            );
        }

        let instruction_details = Instruction {
            instruction_id,
            venue_id: venue_counter,
            settlement_type: SettlementType::SettleOnBlock(block_number),
            created_at: Some(Timestamp::get()),
            trade_date: None,
            value_date: None,
        };
        assert_instruction_status(instruction_id, InstructionStatus::Pending);
        assert_eq!(
            Settlement::instruction_details(instruction_id),
            instruction_details
        );
        assert_affirms_pending(instruction_id, 2);
        assert_eq!(venue_instructions(venue_counter), vec![instruction_id]);

        // Ensure balances have not changed.
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();

        assert_affirm_instruction!(alice.origin(), instruction_id, alice.did);

        // Ensure affirms are in correct state.
        assert_affirms_pending(instruction_id, 1);
        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Affirmed);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Pending);

        // Ensure legs are in a correct state.
        assert_leg_status(instruction_id, LegId(0), LegStatus::ExecutionPending);
        assert_leg_status(instruction_id, LegId(1), LegStatus::PendingTokenLock);

        // Check that tokens are locked for settlement execution.
        assert_locked_assets(&TICKER, &alice, amount);

        // Ensure balances have not changed.
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();

        assert_affirm_instruction!(bob.origin(), instruction_id, bob.did);

        // Ensure all affirms were successful.
        assert_affirms_pending(instruction_id, 0);
        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Affirmed);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Affirmed);

        // Ensure legs are in a pending state.
        assert_leg_status(instruction_id, LegId(0), LegStatus::ExecutionPending);
        assert_leg_status(instruction_id, LegId(1), LegStatus::ExecutionPending);

        // Check that tokens are locked for settlement execution.
        assert_locked_assets(&TICKER, &alice, amount);
        assert_locked_assets(&TICKER2, &bob, amount);

        // Ensure balances have not changed.
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();

        assert_instruction_status(instruction_id, InstructionStatus::Pending);

        // Instruction should execute on the next block and settlement should fail,
        // since the tokens are still locked for settlement execution.
        next_block();

        assert_instruction_status(instruction_id, InstructionStatus::Failed);

        // Check that tokens stay locked after settlement execution failure.
        assert_locked_assets(&TICKER, &alice, amount);
        assert_locked_assets(&TICKER2, &bob, amount);

        // Ensure balances have not changed.
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();

        assert_storage_noop!(assert_err_ignore_postinfo!(
            Settlement::execute_manual_instruction(
                alice.origin(),
                instruction_id,
                None,
                2,
                0,
                0,
                None,
            ),
            Error::FailedToReleaseLockOrTransferAssets
        ));
    });
}

#[test]
fn venue_filtering() {
    test_with_cdd_provider(|_eve| {
        let alice = User::new(AccountKeyring::Alice);
        let bob = User::new(AccountKeyring::Bob);
        let venue_counter = create_token_and_venue(TICKER, alice);
        let block_number = System::block_number() + 1;
        let instruction_id = Settlement::instruction_counter();

        let legs = vec![Leg::Fungible {
            sender: PortfolioId::default_portfolio(alice.did),
            receiver: PortfolioId::default_portfolio(bob.did),
            ticker: TICKER,
            amount: 10,
        }];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnBlock(block_number),
            None,
            None,
            legs.clone(),
            None,
        ));
        assert_ok!(Settlement::set_venue_filtering(
            alice.origin(),
            TICKER,
            true
        ));
        assert_noop!(
            Settlement::add_instruction(
                alice.origin(),
                venue_counter,
                SettlementType::SettleOnBlock(block_number),
                None,
                None,
                legs.clone(),
                None,
            ),
            Error::UnauthorizedVenue
        );
        assert_ok!(Settlement::allow_venues(
            alice.origin(),
            TICKER,
            vec![venue_counter]
        ));
        assert_ok!(Settlement::add_and_affirm_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnBlock(block_number + 1),
            None,
            None,
            legs.clone(),
            default_portfolio_vec(alice.did),
            None,
        ));

        assert_affirm_instruction!(alice.origin(), instruction_id, alice.did);
        assert_affirm_instruction!(bob.origin(), instruction_id, bob.did);
        assert_affirm_instruction!(bob.origin(), instruction_id.checked_inc().unwrap(), bob.did);

        next_block();
        assert_eq!(Asset::balance_of(&TICKER, bob.did), 10);
        assert_ok!(Settlement::disallow_venues(
            alice.origin(),
            TICKER,
            vec![venue_counter]
        ));
        next_block();
        // Second instruction fails to settle due to venue being not whitelisted
        assert_balance(&TICKER, &bob, 10)
    });
}

#[test]
fn basic_fuzzing() {
    test_with_cdd_provider(|_eve| {
        let alice = User::new(AccountKeyring::Alice);
        let bob = User::new(AccountKeyring::Bob);
        let charlie = User::new(AccountKeyring::Charlie);
        let dave = User::new(AccountKeyring::Dave);
        let venue_counter = Settlement::venue_counter();
        assert_ok!(Settlement::create_venue(
            Origin::signed(AccountKeyring::Alice.to_account_id()),
            VenueDetails::default(),
            vec![AccountKeyring::Alice.to_account_id()],
            VenueType::Other
        ));
        let mut tickers = Vec::with_capacity(40);
        let mut balances = HashMap::with_capacity(320);
        let users = vec![alice, bob, charlie, dave];

        for ticker_id in 0..10 {
            let mut create = |x: usize, user: User| {
                let tn = format!("TOKEN{}", ticker_id * 4 + x);
                tickers.push(Ticker::from_slice_truncated(tn.as_bytes()));
                create_token(tickers[ticker_id * 4 + x], user);
            };
            create(0, alice);
            create(1, bob);
            create(2, charlie);
            create(3, dave);
        }

        let block_number = System::block_number() + 1;
        let instruction_id = Settlement::instruction_counter();

        // initialize balances
        for ticker_id in 0..10 {
            for user_id in 0..4 {
                balances.insert(
                    (tickers[ticker_id * 4 + user_id], users[user_id].did, "init").encode(),
                    100_000,
                );
                balances.insert(
                    (
                        tickers[ticker_id * 4 + user_id],
                        users[user_id].did,
                        "final",
                    )
                        .encode(),
                    100_000,
                );
                for k in 0..4 {
                    if user_id == k {
                        continue;
                    }
                    balances.insert(
                        (tickers[ticker_id * 4 + user_id], users[k].did, "init").encode(),
                        0,
                    );
                    balances.insert(
                        (tickers[ticker_id * 4 + user_id], users[k].did, "final").encode(),
                        0,
                    );
                }
            }
        }

        let mut legs = Vec::with_capacity(100);
        let mut legs_count: HashMap<IdentityId, u32> = HashMap::with_capacity(100);
        let mut locked_assets = HashMap::with_capacity(100);
        for ticker_id in 0..10 {
            for user_id in 0..4 {
                let mut final_i = 100_000;
                balances.insert(
                    (tickers[ticker_id * 4 + user_id], users[user_id].did, "init").encode(),
                    100_000,
                );
                for k in 0..4 {
                    if user_id == k {
                        continue;
                    }
                    balances.insert(
                        (tickers[ticker_id * 4 + user_id], users[k].did, "init").encode(),
                        0,
                    );
                    if random() {
                        // This leg should happen
                        balances.insert(
                            (tickers[ticker_id * 4 + user_id], users[k].did, "final").encode(),
                            1,
                        );
                        final_i -= 1;
                        *locked_assets
                            .entry((users[user_id].did, tickers[ticker_id * 4 + user_id]))
                            .or_insert(0) += 1;
                        legs.push(Leg::Fungible {
                            sender: PortfolioId::default_portfolio(users[user_id].did),
                            receiver: PortfolioId::default_portfolio(users[k].did),
                            ticker: tickers[ticker_id * 4 + user_id],
                            amount: 1,
                        });
                        *legs_count.entry(users[user_id].did).or_insert(0) += 1;
                        if legs.len() >= 100 {
                            break;
                        }
                    }
                }
                balances.insert(
                    (
                        tickers[ticker_id * 4 + user_id],
                        users[user_id].did,
                        "final",
                    )
                        .encode(),
                    final_i,
                );
                if legs.len() >= 100 {
                    break;
                }
            }
            if legs.len() >= 100 {
                break;
            }
        }
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnBlock(block_number),
            None,
            None,
            legs.clone(),
            None,
        ));

        // Authorize instructions and do a few authorize/deny in between
        for (_, user) in users.clone().iter().enumerate() {
            for _ in 0..2 {
                if random() {
                    assert_affirm_instruction!(user.origin(), instruction_id, user.did);
                    assert_ok!(Settlement::withdraw_affirmation(
                        user.origin(),
                        instruction_id,
                        default_portfolio_vec(user.did),
                    ));
                }
            }
            assert_affirm_instruction!(user.origin(), instruction_id, user.did);
        }

        fn check_locked_assets(
            locked_assets: &HashMap<(IdentityId, Ticker), i32>,
            tickers: &Vec<Ticker>,
            users: &Vec<User>,
        ) {
            for ((did, ticker), balance) in locked_assets {
                assert_eq!(
                    Portfolio::locked_assets(PortfolioId::default_portfolio(*did), ticker),
                    *balance as u128
                );
            }
            for ticker in tickers {
                for user in users {
                    assert_eq!(
                        Portfolio::locked_assets(PortfolioId::default_portfolio(user.did), &ticker),
                        locked_assets
                            .get(&(user.did, *ticker))
                            .cloned()
                            .unwrap_or(0) as u128
                    );
                }
            }
        }

        check_locked_assets(&locked_assets, &tickers, &users);

        let fail: bool = random();
        let mut rng = thread_rng();
        let failed_user = rng.gen_range(0, 4);
        if fail {
            assert_ok!(Settlement::withdraw_affirmation(
                users[failed_user].origin(),
                instruction_id,
                default_portfolio_vec(users[failed_user].did),
            ));
            locked_assets.retain(|(did, _), _| *did != users[failed_user].did);
        }

        next_block();

        if fail {
            assert_eq!(
                Settlement::instruction_status(instruction_id),
                InstructionStatus::Failed
            );
            check_locked_assets(&locked_assets, &tickers, &users);
        }

        for ticker in &tickers {
            for user in &users {
                if fail {
                    assert_eq!(
                        Asset::balance_of(&ticker, user.did),
                        u128::try_from(
                            *balances.get(&(ticker, user.did, "init").encode()).unwrap()
                        )
                        .unwrap()
                    );
                    assert_eq!(
                        Portfolio::locked_assets(PortfolioId::default_portfolio(user.did), &ticker),
                        locked_assets
                            .get(&(user.did, *ticker))
                            .cloned()
                            .unwrap_or(0) as u128
                    );
                } else {
                    assert_eq!(
                        Asset::balance_of(&ticker, user.did),
                        u128::try_from(
                            *balances.get(&(ticker, user.did, "final").encode()).unwrap()
                        )
                        .unwrap()
                    );
                    assert_eq!(
                        Portfolio::locked_assets(PortfolioId::default_portfolio(user.did), &ticker),
                        0
                    );
                }
            }
        }

        if fail {
            assert_ok!(Settlement::reject_instruction(
                users[0].origin(),
                instruction_id,
                PortfolioId::default_portfolio(users[0].did),
            ));
            assert_eq!(
                Settlement::instruction_status(instruction_id),
                InstructionStatus::Rejected(System::block_number())
            );
        }

        for ticker in &tickers {
            for user in &users {
                assert_eq!(
                    Portfolio::locked_assets(PortfolioId::default_portfolio(user.did), ticker),
                    0
                );
            }
        }
    });
}

#[test]
fn claim_multiple_receipts_during_authorization() {
    ExtBuilder::default().build().execute_with(|| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER]);
        let venue_counter = create_venue(alice.user);
        let id = Settlement::instruction_counter();
        alice.refresh_init_balances();
        bob.refresh_init_balances();
        let amount = 100;

        let legs = vec![
            Leg::OffChain {
                sender_identity: alice.did,
                receiver_identity: bob.did,
                ticker: TICKER,
                amount,
            },
            Leg::OffChain {
                sender_identity: alice.did,
                receiver_identity: bob.did,
                ticker: TICKER2,
                amount,
            },
        ];

        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs.clone(),
            None,
        ));

        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        let msg1 = Receipt::new(0, id, LegId(0), alice.did, bob.did, TICKER, amount);
        let msg2 = Receipt::new(0, id, LegId(1), alice.did, bob.did, TICKER2, amount);
        let msg3 = Receipt::new(1, id, LegId(1), alice.did, bob.did, TICKER2, amount);

        assert_noop!(
            Settlement::affirm_with_receipts(
                alice.origin(),
                id,
                vec![
                    ReceiptDetails::new(
                        0,
                        id,
                        LegId(0),
                        AccountKeyring::Alice.to_account_id(),
                        AccountKeyring::Alice.sign(&msg1.encode()).into(),
                        None
                    ),
                    ReceiptDetails::new(
                        0,
                        id,
                        LegId(0),
                        AccountKeyring::Alice.to_account_id(),
                        AccountKeyring::Alice.sign(&msg2.encode()).into(),
                        None
                    ),
                ],
                Vec::new(),
            ),
            Error::DuplicateReceiptUid
        );

        assert_ok!(Settlement::affirm_with_receipts(
            alice.origin(),
            id,
            vec![
                ReceiptDetails::new(
                    0,
                    id,
                    LegId(0),
                    AccountKeyring::Alice.to_account_id(),
                    AccountKeyring::Alice.sign(&msg1.encode()).into(),
                    None
                ),
                ReceiptDetails::new(
                    1,
                    id,
                    LegId(1),
                    AccountKeyring::Alice.to_account_id(),
                    AccountKeyring::Alice.sign(&msg3.encode()).into(),
                    None
                ),
            ],
            Vec::new(),
        ));

        assert_affirms_pending(id, 0);
        assert_eq!(
            OffChainAffirmations::get(id, LegId(0)),
            AffirmationStatus::Affirmed
        );
        assert_eq!(
            OffChainAffirmations::get(id, LegId(1)),
            AffirmationStatus::Affirmed
        );
        assert_leg_status(
            id,
            LegId(0),
            LegStatus::ExecutionToBeSkipped(AccountKeyring::Alice.to_account_id(), 0),
        );
        assert_leg_status(
            id,
            LegId(1),
            LegStatus::ExecutionToBeSkipped(AccountKeyring::Alice.to_account_id(), 1),
        );
        assert_locked_assets(&TICKER, &alice, 0);

        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        set_current_block_number(1);

        // Advances block
        next_block();
        assert_user_affirms(id, &alice, AffirmationStatus::Unknown);
        assert_user_affirms(id, &bob, AffirmationStatus::Unknown);
        assert_locked_assets(&TICKER, &alice, 0);
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
    });
}

#[test]
fn overload_instruction() {
    test_with_cdd_provider(|_eve| {
        let alice = User::new(AccountKeyring::Alice);
        let bob = User::new(AccountKeyring::Bob);
        let venue_counter = create_token_and_venue(TICKER, alice);
        let leg_limit =
            <TestStorage as pallet_settlement::Config>::MaxNumberOfFungibleAssets::get() as usize;

        let mut legs = vec![
            Leg::Fungible {
                sender: PortfolioId::default_portfolio(alice.did),
                receiver: PortfolioId::default_portfolio(bob.did),
                ticker: TICKER,
                amount: 1,
            };
            leg_limit + 1
        ];

        assert_noop!(
            Settlement::add_instruction(
                alice.origin(),
                venue_counter,
                SettlementType::SettleOnAffirmation,
                None,
                None,
                legs.clone(),
                None,
            ),
            Error::MaxNumberOfFungibleAssetsExceeded
        );
        legs.truncate(leg_limit);
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs,
            None,
        ));
    });
}

#[test]
fn encode_receipt() {
    ExtBuilder::default().build().execute_with(|| {
        let id = InstructionId(0);
        let token_name = [0x01u8];
        let ticker = Ticker::from_slice_truncated(&token_name[..]);
        let identity_id = IdentityId::try_from(
            "did:poly:0600000000000000000000000000000000000000000000000000000000000000",
        )
        .unwrap();
        let msg1 = Receipt::new(0, id, LegId(0), identity_id, identity_id, ticker, 100);
        println!("{:?}", AccountKeyring::Alice.sign(&msg1.encode()));
    });
}

#[test]
fn test_weights_for_settlement_transaction() {
    ExtBuilder::default()
        .cdd_providers(vec![AccountKeyring::Eve.to_account_id()])
        .build()
        .execute_with(|| {
            let alice = AccountKeyring::Alice.to_account_id();
            let (alice_signed, alice_did) = make_account_without_cdd(alice.clone()).unwrap();

            let bob = AccountKeyring::Bob.to_account_id();
            let (bob_signed, bob_did) = make_account_without_cdd(bob).unwrap();

            let dave = AccountKeyring::Dave.to_account_id();
            let (dave_signed, dave_did) = make_account_without_cdd(dave).unwrap();

            let venue_counter =
                create_token_and_venue(TICKER, User::existing(AccountKeyring::Alice));
            let instruction_id = Settlement::instruction_counter();

            // Get token Id.
            let ticker_id = Identity::get_token_did(&TICKER).unwrap();

            // Remove existing rules
            assert_ok!(ComplianceManager::remove_compliance_requirement(
                alice_signed.clone(),
                TICKER,
                1
            ));
            // Add claim rules for settlement
            assert_ok!(ComplianceManager::add_compliance_requirement(
                alice_signed.clone(),
                TICKER,
                vec![
                    Condition::from_dids(
                        ConditionType::IsPresent(Claim::Accredited(ticker_id.into())),
                        &[dave_did]
                    ),
                    Condition::from_dids(
                        ConditionType::IsAbsent(Claim::BuyLockup(ticker_id.into())),
                        &[dave_did]
                    )
                ],
                vec![
                    Condition::from_dids(
                        ConditionType::IsPresent(Claim::Accredited(ticker_id.into())),
                        &[dave_did]
                    ),
                    Condition::from_dids(
                        ConditionType::IsAnyOf(vec![
                            Claim::BuyLockup(ticker_id.into()),
                            Claim::KnowYourCustomer(ticker_id.into())
                        ]),
                        &[dave_did]
                    )
                ]
            ));

            // Providing claim to sender and receiver
            // For Alice
            assert_add_claim!(
                dave_signed.clone(),
                alice_did,
                Claim::Accredited(ticker_id.into())
            );
            // For Bob
            assert_add_claim!(
                dave_signed.clone(),
                bob_did,
                Claim::Accredited(ticker_id.into())
            );
            assert_add_claim!(
                dave_signed.clone(),
                bob_did,
                Claim::KnowYourCustomer(ticker_id.into())
            );

            // Create instruction
            let legs = vec![Leg::Fungible {
                sender: PortfolioId::default_portfolio(alice_did),
                receiver: PortfolioId::default_portfolio(bob_did),
                ticker: TICKER,
                amount: 100,
            }];

            assert_ok!(Settlement::add_instruction(
                alice_signed.clone(),
                venue_counter,
                SettlementType::SettleOnAffirmation,
                None,
                None,
                legs.clone(),
                None,
            ));

            assert_affirm_instruction!(alice_signed.clone(), instruction_id, alice_did);
            set_current_block_number(100);
            assert_affirm_instruction!(bob_signed.clone(), instruction_id, bob_did);

            let mut weight_meter = WeightMeter::max_limit_no_minimum();
            assert_ok!(
                Asset::_is_valid_transfer(
                    &TICKER,
                    PortfolioId::default_portfolio(alice_did),
                    PortfolioId::default_portfolio(bob_did),
                    100,
                    &mut weight_meter
                ),
                ERC1400_TRANSFER_SUCCESS
            );
        });
}

#[test]
fn cross_portfolio_settlement() {
    test_with_cdd_provider(|_eve| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER]);
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        let name = PortfolioName::from([42u8].to_vec());
        let num = Portfolio::next_portfolio_number(&bob.did);
        assert_ok!(Portfolio::create_portfolio(bob.origin(), name.clone()));
        let instruction_id = Settlement::instruction_counter();
        let amount = 100u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        // Instruction referencing a user defined portfolio is created
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            vec![Leg::Fungible {
                sender: PortfolioId::default_portfolio(alice.did),
                receiver: PortfolioId::user_portfolio(bob.did, num),
                ticker: TICKER,
                amount,
            }],
            None,
        ));
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        alice.assert_default_portfolio_bal_unchanged();
        bob.assert_default_portfolio_bal_unchanged();
        bob.assert_portfolio_bal(num, 0);

        assert_locked_assets(&TICKER, &alice, 0);
        set_current_block_number(10);

        // Approved by Alice
        assert_affirm_instruction!(alice.origin(), instruction_id, alice.did);
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        assert_locked_assets(&TICKER, &alice, amount);
        // Bob fails to approve the instruction with a
        // different portfolio than the one specified in the instruction
        next_block();
        assert_noop!(
            Settlement::affirm_instruction(
                bob.origin(),
                instruction_id,
                default_portfolio_vec(bob.did),
            ),
            Error::UnexpectedAffirmationStatus
        );

        next_block();
        // Bob approves the instruction with the correct portfolio
        assert_ok!(Settlement::affirm_instruction(
            bob.origin(),
            instruction_id,
            user_portfolio_vec(bob.did, num),
        ));

        // Instruction should've settled
        next_block();
        alice.assert_balance_decreased(&TICKER, amount);
        bob.assert_balance_increased(&TICKER, amount);
        alice.assert_default_portfolio_bal_decreased(amount);
        bob.assert_default_portfolio_bal_unchanged();
        bob.assert_portfolio_bal(num, amount);
        assert_locked_assets(&TICKER, &alice, 0);
    });
}

#[test]
fn multiple_portfolio_settlement() {
    test_with_cdd_provider(|_eve| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER]);
        let name = PortfolioName::from([42u8].to_vec());
        let alice_num = Portfolio::next_portfolio_number(&alice.did);
        let bob_num = Portfolio::next_portfolio_number(&bob.did);
        assert_ok!(Portfolio::create_portfolio(bob.origin(), name.clone()));
        assert_ok!(Portfolio::create_portfolio(alice.origin(), name.clone()));
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        let instruction_id = Settlement::instruction_counter();
        let amount = 100u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        // An instruction is created with multiple legs referencing multiple portfolios
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            vec![
                Leg::Fungible {
                    sender: PortfolioId::user_portfolio(alice.did, alice_num),
                    receiver: PortfolioId::default_portfolio(bob.did),
                    ticker: TICKER,
                    amount,
                },
                Leg::Fungible {
                    sender: PortfolioId::default_portfolio(alice.did),
                    receiver: PortfolioId::user_portfolio(bob.did, bob_num),
                    ticker: TICKER,
                    amount,
                }
            ],
            None,
        ));
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        alice.assert_default_portfolio_bal_unchanged();
        bob.assert_default_portfolio_bal_unchanged();
        bob.assert_portfolio_bal(bob_num, 0);
        assert_locked_assets(&TICKER, &alice, 0);

        // Alice approves the instruction from her default portfolio
        assert_affirm_instruction!(alice.origin(), instruction_id, alice.did);

        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        alice.assert_default_portfolio_bal_unchanged();
        bob.assert_default_portfolio_bal_unchanged();
        bob.assert_portfolio_bal(bob_num, 0);
        assert_locked_assets(&TICKER, &alice, amount);

        // Alice tries to withdraw affirmation from multiple portfolios where only one has been affirmed.
        assert_noop!(
            Settlement::withdraw_affirmation(
                alice.origin(),
                instruction_id,
                vec![
                    PortfolioId::default_portfolio(alice.did),
                    PortfolioId::user_portfolio(alice.did, alice_num)
                ],
            ),
            Error::UnexpectedAffirmationStatus
        );

        // Alice fails to approve the instruction from her user specified portfolio due to lack of funds
        assert_noop!(
            Settlement::affirm_instruction(
                alice.origin(),
                instruction_id,
                user_portfolio_vec(alice.did, alice_num),
            ),
            PortfolioError::InsufficientPortfolioBalance
        );

        // Alice moves her funds to the correct portfolio
        assert_ok!(Portfolio::move_portfolio_funds(
            alice.origin(),
            PortfolioId::default_portfolio(alice.did),
            PortfolioId::user_portfolio(alice.did, alice_num),
            vec![Fund {
                description: FundDescription::Fungible {
                    ticker: TICKER,
                    amount,
                },
                memo: None,
            }]
        ));
        set_current_block_number(15);
        // Alice is now able to approve the instruction with the user portfolio
        assert_ok!(Settlement::affirm_instruction(
            alice.origin(),
            instruction_id,
            user_portfolio_vec(alice.did, alice_num),
        ));
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        alice.assert_default_portfolio_bal_decreased(amount);
        alice.assert_portfolio_bal(alice_num, amount);
        bob.assert_default_portfolio_bal_unchanged();
        bob.assert_portfolio_bal(bob_num, 0);
        assert_locked_assets(&TICKER, &alice, amount);
        assert_eq!(
            Portfolio::locked_assets(PortfolioId::user_portfolio(alice.did, alice_num), &TICKER),
            amount
        );

        // Bob approves the instruction with both of his portfolios in a single transaction
        let portfolios_vec = vec![
            PortfolioId::default_portfolio(bob.did),
            PortfolioId::user_portfolio(bob.did, bob_num),
        ];

        next_block();
        assert_ok!(Settlement::affirm_instruction(
            bob.origin(),
            instruction_id,
            portfolios_vec,
        ));

        // Instruction should've settled
        next_block();
        alice.assert_balance_decreased(&TICKER, amount * 2);
        bob.assert_balance_increased(&TICKER, amount * 2);
        alice.assert_default_portfolio_bal_decreased(amount * 2);
        bob.assert_default_portfolio_bal_increased(amount);
        bob.assert_portfolio_bal(bob_num, amount);
        assert_locked_assets(&TICKER, &alice, 0);
    });
}

#[test]
fn multiple_custodian_settlement() {
    test_with_cdd_provider(|_eve| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER]);

        // Create portfolios
        let name = PortfolioName::from([42u8].to_vec());
        let alice_num = Portfolio::next_portfolio_number(&alice.did);
        let bob_num = Portfolio::next_portfolio_number(&bob.did);
        assert_ok!(Portfolio::create_portfolio(bob.origin(), name.clone()));
        assert_ok!(Portfolio::create_portfolio(alice.origin(), name.clone()));

        // Give custody of Bob's user portfolio to Alice
        let auth_id = Identity::add_auth(
            bob.did,
            Signatory::from(alice.did),
            AuthorizationData::PortfolioCustody(PortfolioId::user_portfolio(bob.did, bob_num)),
            None,
        );
        assert_ok!(Portfolio::accept_portfolio_custody(alice.origin(), auth_id));

        // Create a token
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        let instruction_id = Settlement::instruction_counter();
        let amount = 100u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        assert_ok!(Portfolio::move_portfolio_funds(
            alice.origin(),
            PortfolioId::default_portfolio(alice.did),
            PortfolioId::user_portfolio(alice.did, alice_num),
            vec![Fund {
                description: FundDescription::Fungible {
                    ticker: TICKER,
                    amount,
                },
                memo: None,
            }]
        ));

        // An instruction is created with multiple legs referencing multiple portfolios
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            vec![
                Leg::Fungible {
                    sender: PortfolioId::user_portfolio(alice.did, alice_num),
                    receiver: PortfolioId::default_portfolio(bob.did),
                    ticker: TICKER,
                    amount,
                },
                Leg::Fungible {
                    sender: PortfolioId::default_portfolio(alice.did),
                    receiver: PortfolioId::user_portfolio(bob.did, bob_num),
                    ticker: TICKER,
                    amount,
                }
            ],
            None,
        ));
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        alice.assert_default_portfolio_bal_decreased(amount);
        bob.assert_default_portfolio_bal_unchanged();
        bob.assert_portfolio_bal(bob_num, 0);
        assert_locked_assets(&TICKER, &alice, 0);

        // Alice approves the instruction from both of her portfolios
        let portfolios_vec = vec![
            PortfolioId::default_portfolio(alice.did),
            PortfolioId::user_portfolio(alice.did, alice_num),
        ];
        set_current_block_number(10);
        assert_ok!(Settlement::affirm_instruction(
            alice.origin(),
            instruction_id,
            portfolios_vec.clone(),
        ));
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        alice.assert_default_portfolio_bal_decreased(amount);
        bob.assert_default_portfolio_bal_unchanged();
        bob.assert_portfolio_bal(bob_num, 0);
        assert_locked_assets(&TICKER, &alice, amount);
        assert_eq!(
            Portfolio::locked_assets(PortfolioId::user_portfolio(alice.did, alice_num), &TICKER),
            amount
        );

        // Alice transfers custody of her portfolios but it won't affect any already approved instruction
        let auth_id2 = Identity::add_auth(
            alice.did,
            Signatory::from(bob.did),
            AuthorizationData::PortfolioCustody(PortfolioId::user_portfolio(alice.did, alice_num)),
            None,
        );
        assert_ok!(Portfolio::accept_portfolio_custody(bob.origin(), auth_id2));

        // Bob fails to approve the instruction with both of his portfolios since he doesn't have custody for the second one
        let portfolios_bob = vec![
            PortfolioId::default_portfolio(bob.did),
            PortfolioId::user_portfolio(bob.did, bob_num),
        ];
        assert_noop!(
            Settlement::affirm_instruction(bob.origin(), instruction_id, portfolios_bob),
            PortfolioError::UnauthorizedCustodian
        );

        next_block();
        // Bob can approve instruction from the portfolio he has custody of
        assert_affirm_instruction!(bob.origin(), instruction_id, bob.did);

        // Alice fails to deny the instruction from both her portfolios since she doesn't have the custody
        next_block();
        assert_noop!(
            Settlement::withdraw_affirmation(alice.origin(), instruction_id, portfolios_vec,),
            PortfolioError::UnauthorizedCustodian
        );

        // Alice can deny instruction from the portfolio she has custody of
        assert_ok!(Settlement::withdraw_affirmation(
            alice.origin(),
            instruction_id,
            default_portfolio_vec(alice.did),
        ));
        assert_locked_assets(&TICKER, &alice, 0);

        // Alice can authorize instruction from remaining portfolios since she has the custody
        let portfolios_final = vec![
            PortfolioId::default_portfolio(alice.did),
            PortfolioId::user_portfolio(bob.did, bob_num),
        ];
        next_block();
        assert_ok!(Settlement::affirm_instruction(
            alice.origin(),
            instruction_id,
            portfolios_final,
        ));

        // Instruction should've settled
        next_block();
        alice.assert_balance_decreased(&TICKER, amount * 2);
        bob.assert_balance_increased(&TICKER, amount * 2);
        alice.assert_default_portfolio_bal_decreased(amount * 2);
        bob.assert_default_portfolio_bal_increased(amount);
        bob.assert_portfolio_bal(bob_num, amount);
        assert_locked_assets(&TICKER, &alice, 0);
    });
}

#[test]
fn reject_instruction() {
    ExtBuilder::default().build().execute_with(|| {
        let alice = User::new(AccountKeyring::Alice);
        let bob = User::new(AccountKeyring::Bob);
        let charlie = User::new(AccountKeyring::Charlie);

        let venue_counter = create_token_and_venue(TICKER, alice);
        let amount = 100u128;

        let reject_instruction = |user: &User, instruction_id| {
            Settlement::reject_instruction(
                user.origin(),
                instruction_id,
                PortfolioId::default_portfolio(user.did),
            )
        };

        let assert_user_affirmations = |instruction_id, alice_status, bob_status| {
            assert_eq!(
                Settlement::user_affirmations(
                    PortfolioId::default_portfolio(alice.did),
                    instruction_id
                ),
                alice_status
            );
            assert_eq!(
                Settlement::user_affirmations(
                    PortfolioId::default_portfolio(bob.did),
                    instruction_id
                ),
                bob_status
            );
        };

        let instruction_id = create_instruction(&alice, &bob, venue_counter, TICKER, amount);
        assert_user_affirmations(
            instruction_id,
            AffirmationStatus::Affirmed,
            AffirmationStatus::Pending,
        );
        next_block();
        // Try rejecting the instruction from a non-party account.
        assert_noop!(
            reject_instruction(&charlie, instruction_id),
            Error::CallerIsNotAParty
        );
        next_block();
        assert_ok!(reject_instruction(&alice, instruction_id,));
        next_block();
        // Instruction should've been deleted
        assert_user_affirmations(
            instruction_id,
            AffirmationStatus::Unknown,
            AffirmationStatus::Unknown,
        );

        // Test that the receiver can also reject the instruction
        let instruction_id2 = create_instruction(&alice, &bob, venue_counter, TICKER, amount);

        assert_ok!(reject_instruction(&bob, instruction_id2,));
        next_block();
        // Instruction should've been deleted
        assert_user_affirmations(
            instruction_id2,
            AffirmationStatus::Unknown,
            AffirmationStatus::Unknown,
        );
    });
}

#[test]
fn dirty_storage_with_tx() {
    test_with_cdd_provider(|_eve| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER]);
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        let instruction_id = Settlement::instruction_counter();
        let amount1 = 100u128;
        let amount2 = 50u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            vec![
                Leg::Fungible {
                    sender: PortfolioId::default_portfolio(alice.did),
                    receiver: PortfolioId::default_portfolio(bob.did),
                    ticker: TICKER,
                    amount: amount1,
                },
                Leg::Fungible {
                    sender: PortfolioId::default_portfolio(alice.did),
                    receiver: PortfolioId::default_portfolio(bob.did),
                    ticker: TICKER,
                    amount: amount2,
                }
            ],
            None,
        ));

        assert_affirm_instruction!(alice.origin(), instruction_id, alice.did);
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        set_current_block_number(5);
        assert_affirm_instruction!(bob.origin(), instruction_id, bob.did);

        // Advances the block no. to execute the instruction.
        let total_amount = amount1 + amount2;
        assert_eq!(Settlement::instruction_affirms_pending(instruction_id), 0);
        next_block();
        assert_eq!(InstructionLegs::iter_prefix(instruction_id).count(), 0);

        // Ensure proper balance transfers
        alice.assert_balance_decreased(&TICKER, total_amount);
        bob.assert_balance_increased(&TICKER, total_amount);
    });
}

#[test]
fn reject_failed_instruction() {
    ExtBuilder::default().build().execute_with(|| {
        let alice = User::new(AccountKeyring::Alice);
        let bob = User::new(AccountKeyring::Bob);

        let venue_counter = create_token_and_venue(TICKER, alice);
        let amount = 100u128;

        let instruction_id = create_instruction(&alice, &bob, venue_counter, TICKER, amount);

        assert_ok!(Settlement::affirm_instruction(
            bob.origin(),
            instruction_id,
            default_portfolio_vec(bob.did),
        ));

        // Resume compliance to cause transfer failure.
        assert_ok!(ComplianceManager::resume_asset_compliance(
            alice.origin(),
            TICKER
        ));
        assert_ok!(ComplianceManager::reset_asset_compliance(
            alice.origin(),
            TICKER
        ));

        // Go to next block to have the scheduled execution run and ensure it has failed.
        next_block();
        assert_instruction_status(instruction_id, InstructionStatus::<BlockNumber>::Failed);

        // Reject instruction so that it is pruned on next execution.
        assert_ok!(Settlement::reject_instruction(
            bob.origin(),
            instruction_id,
            PortfolioId::default_portfolio(bob.did),
        ));

        // Go to next block to have the scheduled execution run and ensure it has pruned the instruction.
        next_block();
        assert_instruction_status(
            instruction_id,
            InstructionStatus::Rejected(System::block_number() - 1),
        );
    });
}

#[test]
fn modify_venue_signers() {
    ExtBuilder::default().build().execute_with(|| {
        let alice = User::new(AccountKeyring::Alice);
        let charlie = User::new(AccountKeyring::Charlie);
        let venue_counter = Settlement::venue_counter();

        assert_ok!(Settlement::create_venue(
            alice.origin(),
            VenueDetails::default(),
            vec![
                AccountKeyring::Alice.to_account_id(),
                AccountKeyring::Bob.to_account_id()
            ],
            VenueType::Exchange
        ));

        // Charlie fails to add dave to signer list
        assert_noop!(
            Settlement::update_venue_signers(
                charlie.origin(),
                venue_counter,
                vec![AccountKeyring::Dave.to_account_id(),],
                true
            ),
            Error::Unauthorized
        );

        // Alice adds charlie to signer list
        assert_ok!(Settlement::update_venue_signers(
            alice.origin(),
            venue_counter,
            vec![AccountKeyring::Charlie.to_account_id(),],
            true
        ));

        // Alice fails to remove dave from signer list
        assert_noop!(
            Settlement::update_venue_signers(
                alice.origin(),
                venue_counter,
                vec![AccountKeyring::Dave.to_account_id(),],
                false
            ),
            Error::SignerDoesNotExist
        );

        // Alice fails to add charlie to the signer list
        assert_noop!(
            Settlement::update_venue_signers(
                alice.origin(),
                venue_counter,
                vec![AccountKeyring::Charlie.to_account_id(),],
                true
            ),
            Error::SignerAlreadyExists
        );

        // Alice removes charlie from signer list
        assert_ok!(Settlement::update_venue_signers(
            alice.origin(),
            venue_counter,
            vec![AccountKeyring::Charlie.to_account_id(),],
            false
        ));

        // this checks if the signer is already in the signer list
        assert_eq!(Settlement::venue_signers(venue_counter, alice.acc()), true);
        assert_eq!(
            Settlement::venue_signers(venue_counter, AccountKeyring::Bob.to_account_id()),
            true
        );
        assert_eq!(
            Settlement::venue_signers(venue_counter, AccountKeyring::Charlie.to_account_id()),
            false
        );

        // Alice adds charlie, dave and eve
        assert_ok!(Settlement::update_venue_signers(
            alice.origin(),
            venue_counter,
            vec![
                AccountKeyring::Charlie.to_account_id(),
                AccountKeyring::Dave.to_account_id(),
                AccountKeyring::Eve.to_account_id(),
            ],
            true
        ));

        // Alice removes charlie, dave and eve
        assert_ok!(Settlement::update_venue_signers(
            alice.origin(),
            venue_counter,
            vec![
                AccountKeyring::Charlie.to_account_id(),
                AccountKeyring::Dave.to_account_id(),
                AccountKeyring::Eve.to_account_id(),
            ],
            false
        ));

        // Alice fails to adds charlie, dave, eve and bob
        assert_noop!(
            Settlement::update_venue_signers(
                alice.origin(),
                venue_counter,
                vec![
                    AccountKeyring::Charlie.to_account_id(),
                    AccountKeyring::Dave.to_account_id(),
                    AccountKeyring::Eve.to_account_id(),
                    AccountKeyring::Bob.to_account_id()
                ],
                true
            ),
            Error::SignerAlreadyExists
        );

        assert_eq!(Settlement::venue_signers(venue_counter, alice.acc()), true);
        assert_eq!(
            Settlement::venue_signers(venue_counter, AccountKeyring::Bob.to_account_id()),
            true
        );
        assert_eq!(
            Settlement::venue_signers(venue_counter, AccountKeyring::Charlie.to_account_id()),
            false
        );
        assert_eq!(
            Settlement::venue_signers(venue_counter, AccountKeyring::Dave.to_account_id()),
            false
        );
        assert_eq!(
            Settlement::venue_signers(venue_counter, AccountKeyring::Eve.to_account_id()),
            false
        );
    });
}

#[test]
fn assert_number_of_venue_signers() {
    ExtBuilder::default().build().execute_with(|| {
        let max_signers =
            <TestStorage as pallet_settlement::Config>::MaxNumberOfVenueSigners::get();
        let venue_id = VenueId(0);
        let alice = User::new(AccountKeyring::Alice);
        let initial_signers: Vec<AccountId32> = (0..max_signers as u8)
            .map(|i| AccountId32::from([i; 32]))
            .collect();
        // Verifies that an error will be thrown when the limit is exceeded
        assert_noop!(
            Settlement::create_venue(
                alice.origin(),
                VenueDetails::default(),
                (0..max_signers as u8 + 1)
                    .map(|i| AccountId32::from([i; 32]))
                    .collect(),
                VenueType::Exchange
            ),
            Error::NumberOfVenueSignersExceeded
        );
        // Successfully creates a venue with max_signers
        assert_ok!(Settlement::create_venue(
            alice.origin(),
            VenueDetails::default(),
            initial_signers.clone(),
            VenueType::Exchange
        ));
        assert_eq!(NumberOfVenueSigners::get(venue_id), max_signers);
        // Verifies that an error will be thrown when the limit is exceeded
        assert_noop!(
            Settlement::update_venue_signers(
                alice.origin(),
                venue_id,
                vec![AccountId32::from([51; 32])],
                true
            ),
            Error::NumberOfVenueSignersExceeded
        );
        // Verifies that the count is being updated when adding removing signers
        assert_ok!(Settlement::update_venue_signers(
            alice.origin(),
            venue_id,
            initial_signers[0..3].to_vec(),
            false
        ));
        assert_eq!(NumberOfVenueSigners::get(venue_id), max_signers - 3);
        // Verifies that the count is being updated when adding adding new signers
        assert_ok!(Settlement::update_venue_signers(
            alice.origin(),
            venue_id,
            initial_signers[0..2].to_vec(),
            true
        ));
        assert_eq!(NumberOfVenueSigners::get(venue_id), max_signers - 1);
    })
}

#[test]

fn reject_instruction_with_zero_amount() {
    test_with_cdd_provider(|_eve| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER]);
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        let amount = 0u128;

        alice.refresh_init_balances();
        bob.refresh_init_balances();

        assert_noop!(
            Settlement::add_instruction(
                alice.origin(),
                venue_counter,
                SettlementType::SettleOnAffirmation,
                None,
                None,
                vec![Leg::Fungible {
                    sender: PortfolioId::default_portfolio(alice.did),
                    receiver: PortfolioId::default_portfolio(bob.did),
                    ticker: TICKER,
                    amount,
                }],
                None,
            ),
            Error::ZeroAmount
        );
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
    });
}

fn basic_settlement_with_memo() {
    test_with_cdd_provider(|_eve| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER]);
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        let instruction_id = Settlement::instruction_counter();
        let amount = 100u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            vec![Leg::Fungible {
                sender: PortfolioId::default_portfolio(alice.did),
                receiver: PortfolioId::default_portfolio(bob.did),
                ticker: TICKER,
                amount,
            }],
            Some(Memo::default()),
        ));
        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();

        // check that the memo was stored correctly
        assert_eq!(Settlement::memo(instruction_id).unwrap(), Memo::default());

        assert_affirm_instruction!(alice.origin(), instruction_id, alice.did);

        alice.assert_all_balances_unchanged();
        bob.assert_all_balances_unchanged();
        set_current_block_number(5);
        // Instruction get scheduled to next block.
        assert_affirm_instruction!(bob.origin(), instruction_id, bob.did);

        // Advances the block no. to execute the instruction.
        next_block();
        alice.assert_balance_decreased(&TICKER, amount);
        bob.assert_balance_increased(&TICKER, amount);
    });
}

fn create_instruction(
    alice: &User,
    bob: &User,
    venue_counter: VenueId,
    ticker: Ticker,
    amount: u128,
) -> InstructionId {
    let instruction_id = Settlement::instruction_counter();
    set_current_block_number(10);
    assert_ok!(Settlement::add_and_affirm_instruction(
        alice.origin(),
        venue_counter,
        SettlementType::SettleOnAffirmation,
        None,
        None,
        vec![Leg::Fungible {
            sender: PortfolioId::default_portfolio(alice.did),
            receiver: PortfolioId::default_portfolio(bob.did),
            ticker,
            amount
        }],
        default_portfolio_vec(alice.did),
        None,
    ));
    instruction_id
}

#[test]
fn settle_manual_instruction() {
    test_with_cdd_provider(|_eve| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER]);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER]);
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        let instruction_id = Settlement::instruction_counter();
        let block_number = System::block_number() + 1;
        let amount = 10u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        let legs = vec![Leg::Fungible {
            sender: PortfolioId::default_portfolio(alice.did),
            receiver: PortfolioId::default_portfolio(bob.did),
            ticker: TICKER,
            amount,
        }];

        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleManual(block_number),
            None,
            None,
            legs.clone(),
            None,
        ));

        // Ensure instruction is pending
        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Pending);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Pending);

        // Affirm instruction for alice and bob
        assert_affirm_instruction!(alice.origin(), instruction_id, alice.did);
        assert_affirm_instruction!(bob.origin(), instruction_id, bob.did);

        // Ensure it gave the correct error message after it failed because the execution block number hasn't reached yet
        assert_noop!(
            Settlement::execute_manual_instruction(
                alice.origin(),
                instruction_id,
                None,
                1,
                0,
                0,
                None
            ),
            DispatchErrorWithPostInfo {
                post_info: Some(Settlement::execute_manual_instruction_minimum_weight()).into(),
                error: Error::InstructionSettleBlockNotReached.into()
            }
        );
        next_block();
        // Ensure bob can't execute instruction with portfolio set to none since he is not the venue creator
        assert_noop!(
            Settlement::execute_manual_instruction(
                bob.origin(),
                instruction_id,
                None,
                1,
                0,
                0,
                None
            ),
            DispatchErrorWithPostInfo {
                post_info: Some(Settlement::execute_manual_instruction_minimum_weight()).into(),
                error: Error::Unauthorized.into()
            }
        );
        // Ensure correct error message when wrong number of legs is given
        assert_noop!(
            Settlement::execute_manual_instruction(
                alice.origin(),
                instruction_id,
                None,
                0,
                0,
                0,
                None
            ),
            DispatchErrorWithPostInfo {
                post_info: Some(Settlement::execute_manual_instruction_minimum_weight()).into(),
                error: Error::NumberOfFungibleTransfersUnderestimated.into()
            }
        );
        // Ensure it succeeds as the execute block was reached
        assert_ok!(Settlement::execute_manual_instruction(
            alice.origin(),
            instruction_id,
            None,
            1,
            0,
            0,
            None
        ));
        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Unknown);
        assert_locked_assets(&TICKER, &alice, 0);

        alice.assert_balance_decreased(&TICKER, amount);
        bob.assert_balance_increased(&TICKER, amount);
    });
}

#[test]
fn settle_manual_instruction_with_portfolio() {
    test_with_cdd_provider(|_eve| {
        let mut alice = UserWithBalance::new(AccountKeyring::Alice, &[TICKER]);
        let alice_portfolio = PortfolioId::default_portfolio(alice.did);
        let mut bob = UserWithBalance::new(AccountKeyring::Bob, &[TICKER]);
        let charlie = UserWithBalance::new(AccountKeyring::Charlie, &[TICKER]);
        let charlie_portfolio = PortfolioId::default_portfolio(charlie.did);
        let venue_counter = create_token_and_venue(TICKER, alice.user);
        let instruction_id = Settlement::instruction_counter();
        let block_number = System::block_number() + 1;
        let amount = 10u128;
        alice.refresh_init_balances();
        bob.refresh_init_balances();

        let legs = vec![Leg::Fungible {
            sender: alice_portfolio.clone(),
            receiver: PortfolioId::default_portfolio(bob.did),
            ticker: TICKER,
            amount,
        }];

        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleManual(block_number),
            None,
            None,
            legs.clone(),
            None,
        ));

        // Ensure instruction is pending
        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Pending);
        assert_user_affirms(instruction_id, &bob, AffirmationStatus::Pending);

        // Affirm instruction for alice and bob
        assert_affirm_instruction!(alice.origin(), instruction_id, alice.did);
        assert_affirm_instruction!(bob.origin(), instruction_id, bob.did);

        // Ensure it gave the correct error message after it failed because the execution block number hasn't reached yet
        assert_noop!(
            Settlement::execute_manual_instruction(
                alice.origin(),
                instruction_id,
                Some(alice_portfolio),
                1,
                0,
                0,
                None
            ),
            DispatchErrorWithPostInfo {
                post_info: Some(Settlement::execute_manual_instruction_minimum_weight()).into(),
                error: Error::InstructionSettleBlockNotReached.into()
            }
        );
        next_block();
        // Ensure correct error is shown when non party member tries to execute function
        assert_noop!(
            Settlement::execute_manual_instruction(
                charlie.origin(),
                instruction_id,
                Some(charlie_portfolio),
                1,
                0,
                0,
                None,
            ),
            DispatchErrorWithPostInfo {
                post_info: Some(Settlement::execute_manual_instruction_minimum_weight()).into(),
                error: Error::CallerIsNotAParty.into()
            }
        );
        // Ensure correct error message when wrong number of legs is given
        assert_noop!(
            Settlement::execute_manual_instruction(
                alice.origin(),
                instruction_id,
                Some(alice_portfolio),
                0,
                0,
                0,
                None
            ),
            DispatchErrorWithPostInfo {
                post_info: Some(Settlement::execute_manual_instruction_minimum_weight()).into(),
                error: Error::NumberOfFungibleTransfersUnderestimated.into()
            }
        );
        // Ensure it succeeds as the execute block was reached
        assert_ok!(Settlement::execute_manual_instruction(
            alice.origin(),
            instruction_id,
            Some(alice_portfolio),
            1,
            0,
            0,
            None
        ));
        assert_user_affirms(instruction_id, &alice, AffirmationStatus::Unknown);
        assert_locked_assets(&TICKER, &alice, 0);

        alice.assert_balance_decreased(&TICKER, amount);
        bob.assert_balance_increased(&TICKER, amount);

        let mut system_events = System::events();
        assert_eq!(
            system_events.pop().unwrap().event,
            super::storage::EventTest::Settlement(RawEvent::SettlementManuallyExecuted(
                alice.did,
                instruction_id
            ))
        );
        assert_eq!(
            system_events.pop().unwrap().event,
            super::storage::EventTest::Settlement(RawEvent::InstructionExecuted(
                alice.did,
                instruction_id
            ))
        );
    });
}

/// An instruction with non-fungible assets, must reject duplicated NFTIds.
#[test]
fn add_nft_instruction_with_duplicated_nfts() {
    ExtBuilder::default().build().execute_with(|| {
        let alice = User::new(AccountKeyring::Alice);
        let bob = User::new(AccountKeyring::Bob);
        let venue_counter = create_token_and_venue(TICKER, alice);

        let nfts = NFTs::new_unverified(TICKER, vec![NFTId(1), NFTId(1)]);
        let legs: Vec<Leg> = vec![Leg::NonFungible {
            sender: PortfolioId::default_portfolio(alice.did),
            receiver: PortfolioId::default_portfolio(bob.did),
            nfts,
        }];
        assert_noop!(
            Settlement::add_instruction(
                alice.origin(),
                venue_counter,
                SettlementType::SettleOnAffirmation,
                None,
                None,
                legs,
                Some(Memo::default()),
            ),
            NFTError::DuplicatedNFTId
        );
    });
}

/// An instruction with non-fungible assets, must reject legs with more than MaxNumberOfNFTsPerLeg.
#[test]
fn add_nft_instruction_exceeding_nfts() {
    ExtBuilder::default().build().execute_with(|| {
        let alice = User::new(AccountKeyring::Alice);
        let bob = User::new(AccountKeyring::Bob);
        let venue_counter = create_token_and_venue(TICKER, alice);

        let nfts = NFTs::new_unverified(
            TICKER,
            vec![
                NFTId(1),
                NFTId(2),
                NFTId(3),
                NFTId(4),
                NFTId(5),
                NFTId(6),
                NFTId(7),
                NFTId(8),
                NFTId(9),
                NFTId(10),
                NFTId(11),
            ],
        );
        let legs: Vec<Leg> = vec![Leg::NonFungible {
            sender: PortfolioId::default_portfolio(alice.did),
            receiver: PortfolioId::default_portfolio(bob.did),
            nfts,
        }];
        assert_noop!(
            Settlement::add_instruction(
                alice.origin(),
                venue_counter,
                SettlementType::SettleOnAffirmation,
                None,
                None,
                legs,
                Some(Memo::default()),
            ),
            NFTError::MaxNumberOfNFTsPerLegExceeded
        );
    });
}

/// Successfully adds an instruction with non-fungible assets.
#[test]
fn add_nft_instruction() {
    ExtBuilder::default().build().execute_with(|| {
        let alice = User::new(AccountKeyring::Alice);
        let bob = User::new(AccountKeyring::Bob);
        let venue_counter = create_token_and_venue(TICKER, alice);

        let nfts = NFTs::new_unverified(TICKER, vec![NFTId(1)]);
        let legs: Vec<Leg> = vec![Leg::NonFungible {
            sender: PortfolioId::default_portfolio(alice.did),
            receiver: PortfolioId::default_portfolio(bob.did),
            nfts,
        }];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_counter,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs,
            Some(Memo::default()),
        ));
    });
}

/// Successfully adds and affirms an instruction with non-fungible assets.
#[test]
fn add_and_affirm_nft_instruction() {
    test_with_cdd_provider(|_eve| {
        // First we need to create a collection, mint one NFT, and create a venue
        let alice: User = User::new(AccountKeyring::Alice);
        let bob: User = User::new(AccountKeyring::Bob);
        let collection_keys: NFTCollectionKeys =
            vec![AssetMetadataKey::Local(AssetMetadataLocalKey(1))].into();
        create_nft_collection(
            alice.clone(),
            TICKER,
            AssetType::NonFungible(NonFungibleType::Derivative),
            collection_keys,
        );
        let nfts_metadata: Vec<NFTMetadataAttribute> = vec![NFTMetadataAttribute {
            key: AssetMetadataKey::Local(AssetMetadataLocalKey(1)),
            value: AssetMetadataValue(b"test".to_vec()),
        }];
        mint_nft(alice.clone(), TICKER, nfts_metadata, PortfolioKind::Default);
        ComplianceManager::pause_asset_compliance(alice.origin(), TICKER).unwrap();
        let venue_id = create_venue(alice);

        // Adds and affirms the instruction
        let instruction_id = Settlement::instruction_counter();
        let nfts = NFTs::new_unverified(TICKER, vec![NFTId(1)]);
        let legs: Vec<Leg> = vec![Leg::NonFungible {
            sender: PortfolioId::default_portfolio(alice.did),
            receiver: PortfolioId::default_portfolio(bob.did),
            nfts,
        }];
        assert_ok!(Settlement::add_and_affirm_instruction(
            alice.origin(),
            venue_id,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs,
            default_portfolio_vec(alice.did),
            Some(Memo::default()),
        ));

        // Before bob accepts the transaction balances must not be changed and the NFT must be locked.
        assert_eq!(NumberOfNFTs::get(TICKER, alice.did), 1);
        assert_eq!(
            PortfolioNFT::get(
                PortfolioId::default_portfolio(alice.did),
                (TICKER, NFTId(1))
            ),
            true
        );
        assert_eq!(
            PortfolioLockedNFT::get(
                PortfolioId::default_portfolio(alice.did),
                (TICKER, NFTId(1))
            ),
            true
        );

        // Bob affirms the instruction. Balances must be updated and NFT unlocked.
        assert_ok!(Settlement::affirm_instruction(
            bob.origin(),
            instruction_id,
            default_portfolio_vec(bob.did),
        ));
        next_block();
        assert_eq!(NumberOfNFTs::get(TICKER, alice.did), 0);
        assert_eq!(NumberOfNFTs::get(TICKER, bob.did), 1);
        assert_eq!(
            PortfolioNFT::get(
                PortfolioId::default_portfolio(alice.did),
                (TICKER, NFTId(1))
            ),
            false
        );
        assert_eq!(
            PortfolioNFT::get(PortfolioId::default_portfolio(bob.did), (TICKER, NFTId(1))),
            true
        );
        assert_eq!(
            PortfolioLockedNFT::get(
                PortfolioId::default_portfolio(alice.did),
                (TICKER, NFTId(1))
            ),
            false
        );
        assert_eq!(
            PortfolioLockedNFT::get(PortfolioId::default_portfolio(bob.did), (TICKER, NFTId(1))),
            false
        );
    });
}

/// Only instructions with NFTS owned by the caller can be affirmed.
#[test]
fn add_and_affirm_nft_not_owned() {
    test_with_cdd_provider(|_eve| {
        // First we need to create a collection, mint one NFT, and create a venue
        let alice: User = User::new(AccountKeyring::Alice);
        let bob: User = User::new(AccountKeyring::Bob);
        let collection_keys: NFTCollectionKeys =
            vec![AssetMetadataKey::Local(AssetMetadataLocalKey(1))].into();
        create_nft_collection(
            alice.clone(),
            TICKER,
            AssetType::NonFungible(NonFungibleType::Derivative),
            collection_keys,
        );
        let nfts_metadata: Vec<NFTMetadataAttribute> = vec![NFTMetadataAttribute {
            key: AssetMetadataKey::Local(AssetMetadataLocalKey(1)),
            value: AssetMetadataValue(b"test".to_vec()),
        }];
        mint_nft(alice.clone(), TICKER, nfts_metadata, PortfolioKind::Default);
        let venue_id = create_venue(alice);

        // Adds and affirms the instruction
        let nfts = NFTs::new_unverified(TICKER, vec![NFTId(2)]);
        let legs: Vec<Leg> = vec![Leg::NonFungible {
            sender: PortfolioId::default_portfolio(alice.did),
            receiver: PortfolioId::default_portfolio(bob.did),
            nfts,
        }];
        assert_noop!(
            Settlement::add_and_affirm_instruction(
                alice.origin(),
                venue_id,
                SettlementType::SettleOnAffirmation,
                None,
                None,
                legs,
                default_portfolio_vec(alice.did),
                Some(Memo::default()),
            ),
            PortfolioError::NFTNotFoundInPortfolio
        );
    });
}

/// An NFT can only be included in one of the legs.
#[test]
fn add_same_nft_different_legs() {
    test_with_cdd_provider(|_eve| {
        // First we need to create a collection, mint two NFTs, and create a venue
        let alice: User = User::new(AccountKeyring::Alice);
        let bob: User = User::new(AccountKeyring::Bob);
        let collection_keys: NFTCollectionKeys =
            vec![AssetMetadataKey::Local(AssetMetadataLocalKey(1))].into();
        create_nft_collection(
            alice.clone(),
            TICKER,
            AssetType::NonFungible(NonFungibleType::Derivative),
            collection_keys,
        );
        let nfts_metadata: Vec<NFTMetadataAttribute> = vec![NFTMetadataAttribute {
            key: AssetMetadataKey::Local(AssetMetadataLocalKey(1)),
            value: AssetMetadataValue(b"test".to_vec()),
        }];
        mint_nft(
            alice.clone(),
            TICKER,
            nfts_metadata.clone(),
            PortfolioKind::Default,
        );
        mint_nft(alice.clone(), TICKER, nfts_metadata, PortfolioKind::Default);
        let venue_id = create_venue(alice);

        // Adds and affirms the instruction
        let legs: Vec<Leg> = vec![
            Leg::NonFungible {
                sender: PortfolioId::default_portfolio(alice.did),
                receiver: PortfolioId::default_portfolio(bob.did),
                nfts: NFTs::new_unverified(TICKER, vec![NFTId(1)]),
            },
            Leg::NonFungible {
                sender: PortfolioId::default_portfolio(alice.did),
                receiver: PortfolioId::default_portfolio(bob.did),
                nfts: NFTs::new_unverified(TICKER, vec![NFTId(1)]),
            },
        ];
        assert_noop!(
            Settlement::add_and_affirm_instruction(
                alice.origin(),
                venue_id,
                SettlementType::SettleOnAffirmation,
                None,
                None,
                legs,
                default_portfolio_vec(alice.did),
                Some(Memo::default()),
            ),
            PortfolioError::NFTAlreadyLocked
        );
    });
}

/// Receipts can only be used for offchain assets.
#[test]
fn add_and_affirm_with_receipts_nfts() {
    test_with_cdd_provider(|_eve| {
        // First we need to create a collection, mint one NFT, and create a venue
        let id = InstructionId(0);
        let alice: User = User::new(AccountKeyring::Alice);
        let bob: User = User::new(AccountKeyring::Bob);
        let collection_keys: NFTCollectionKeys =
            vec![AssetMetadataKey::Local(AssetMetadataLocalKey(1))].into();
        create_nft_collection(
            alice.clone(),
            TICKER,
            AssetType::NonFungible(NonFungibleType::Derivative),
            collection_keys,
        );
        let nfts_metadata: Vec<NFTMetadataAttribute> = vec![NFTMetadataAttribute {
            key: AssetMetadataKey::Local(AssetMetadataLocalKey(1)),
            value: AssetMetadataValue(b"test".to_vec()),
        }];
        mint_nft(alice.clone(), TICKER, nfts_metadata, PortfolioKind::Default);
        let venue_id = create_venue(alice);

        // Adds the instruction and fails to use a receipt
        let legs: Vec<Leg> = vec![Leg::NonFungible {
            sender: PortfolioId::default_portfolio(alice.did),
            receiver: PortfolioId::default_portfolio(bob.did),
            nfts: NFTs::new_unverified(TICKER, vec![NFTId(1)]),
        }];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_id,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs,
            Some(Memo::default()),
        ));
        assert_noop!(
            Settlement::affirm_with_receipts(
                alice.origin(),
                InstructionId(0),
                vec![ReceiptDetails::new(
                    0,
                    id,
                    LegId(0),
                    AccountKeyring::Alice.to_account_id(),
                    AccountKeyring::Alice
                        .sign(
                            &Receipt::new(0, id, LegId(0), alice.did, bob.did, TICKER, 1).encode()
                        )
                        .into(),
                    None
                )],
                Vec::new(),
            ),
            Error::ReceiptForInvalidLegType
        );
    });
}

/// An instruction must reject legs that are not of type off-chain if the ticker is not on chain.
#[test]
fn add_instruction_unexpected_offchain_asset() {
    ExtBuilder::default().build().execute_with(|| {
        let alice = User::new(AccountKeyring::Alice);
        let bob = User::new(AccountKeyring::Bob);
        let venue_counter = create_venue(alice);

        let nfts = NFTs::new_unverified(TICKER, vec![NFTId(1)]);
        let legs: Vec<Leg> = vec![Leg::NonFungible {
            sender: PortfolioId::default_portfolio(alice.did),
            receiver: PortfolioId::default_portfolio(bob.did),
            nfts,
        }];
        assert_noop!(
            Settlement::add_instruction(
                alice.origin(),
                venue_counter,
                SettlementType::SettleOnAffirmation,
                None,
                None,
                legs,
                Some(Memo::default()),
            ),
            Error::UnexpectedOFFChainAsset
        );

        let legs: Vec<Leg> = vec![Leg::Fungible {
            sender: PortfolioId::default_portfolio(alice.did),
            receiver: PortfolioId::default_portfolio(bob.did),
            ticker: TICKER,
            amount: 1,
        }];
        assert_noop!(
            Settlement::add_instruction(
                alice.origin(),
                venue_counter,
                SettlementType::SettleOnAffirmation,
                None,
                None,
                legs,
                Some(Memo::default()),
            ),
            Error::UnexpectedOFFChainAsset
        );
    });
}

#[test]
fn add_and_execute_offchain_instruction() {
    ExtBuilder::default().build().execute_with(|| {
        let charlie = User::new(AccountKeyring::Charlie);
        let alice = User::new(AccountKeyring::Alice);
        let dave = User::new(AccountKeyring::Dave);
        let bob = User::new(AccountKeyring::Bob);
        let venue_id = create_token_and_venue(TICKER, alice);
        let amount = 1;
        let id = InstructionId(0);

        let legs: Vec<Leg> = vec![Leg::OffChain {
            sender_identity: charlie.did,
            receiver_identity: bob.did,
            ticker: TICKER,
            amount,
        }];
        let receipt = Receipt::new(0, id, LegId(0), charlie.did, bob.did, TICKER, amount);
        let receipts_details = vec![ReceiptDetails::new(
            0,
            id,
            LegId(0),
            AccountKeyring::Alice.to_account_id(),
            AccountKeyring::Alice.sign(&receipt.encode()).into(),
            None,
        )];

        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_id,
            SettlementType::SettleManual(System::block_number() + 1),
            None,
            None,
            legs,
            Some(Memo::default()),
        ),);
        assert_ok!(Settlement::affirm_with_receipts(
            alice.origin(),
            id,
            receipts_details,
            Vec::new(),
        ),);
        next_block();

        assert_noop!(
            Settlement::execute_manual_instruction(
                dave.origin(),
                InstructionId(0),
                None,
                0,
                0,
                1,
                None
            ),
            DispatchErrorWithPostInfo {
                post_info: Some(Settlement::execute_manual_instruction_minimum_weight()).into(),
                error: Error::Unauthorized.into()
            }
        );
        assert_ok!(Settlement::execute_manual_instruction(
            charlie.origin(),
            InstructionId(0),
            None,
            0,
            0,
            1,
            None
        ),);
    });
}

/// Off-chain assets can only be affirmed with receipts.
#[test]
fn affirm_offchain_asset_without_receipt() {
    ExtBuilder::default().build().execute_with(|| {
        let alice = User::new(AccountKeyring::Alice);
        let bob = User::new(AccountKeyring::Bob);
        let venue = create_venue(alice);
        let alice_portfolio = PortfolioId::default_portfolio(alice.did);

        let legs: Vec<Leg> = vec![Leg::OffChain {
            sender_identity: alice.did,
            receiver_identity: bob.did,
            ticker: TICKER,
            amount: 1,
        }];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs,
            Some(Memo::default()),
        ),);
        assert_noop!(
            Settlement::affirm_instruction(
                alice.origin(),
                InstructionId(0),
                vec![alice_portfolio],
            ),
            Error::UnexpectedAffirmationStatus
        );
    });
}

#[test]
fn add_instruction_with_offchain_assets() {
    ExtBuilder::default().build().execute_with(|| {
        // Setup base parameters
        let alice = User::new(AccountKeyring::Alice);
        let alice_default_portfolio = PortfolioId::default_portfolio(alice.did);
        let bob = User::new(AccountKeyring::Bob);
        let bob_default_portfolio = PortfolioId::default_portfolio(bob.did);
        let venue = create_token_and_venue(TICKER, alice);
        let instruction_memo = Some(Memo::default());
        Portfolio::create_portfolio(bob.origin(), b"BobUserPortfolio".into()).unwrap();

        // Both users have pre-affirmed the ticker
        Asset::pre_approve_ticker(alice.origin(), TICKER2).unwrap();
        Asset::pre_approve_ticker(bob.origin(), TICKER2).unwrap();

        let legs: Vec<Leg> = vec![
            Leg::Fungible {
                sender: alice_default_portfolio,
                receiver: bob_default_portfolio,
                ticker: TICKER,
                amount: ONE_UNIT,
            },
            Leg::OffChain {
                sender_identity: alice.did,
                receiver_identity: bob.did,
                ticker: TICKER2,
                amount: ONE_UNIT,
            },
            Leg::OffChain {
                sender_identity: alice.did,
                receiver_identity: bob.did,
                ticker: TICKER2,
                amount: ONE_UNIT,
            },
        ];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs.clone(),
            instruction_memo.clone(),
        ));
        // Only the sender still has to approve the transfer
        let portfolios_pending_approval =
            BTreeSet::from([alice_default_portfolio, bob_default_portfolio]);
        let portfolios_pre_approved = BTreeSet::new();
        let offchain_legs = BTreeSet::from([LegId(1), LegId(2)]);
        let instruction_id = InstructionId(0);
        assert_add_instruction_storage(
            &instruction_id,
            &portfolios_pending_approval,
            &portfolios_pre_approved,
            &offchain_legs,
            instruction_memo,
            &legs,
        );
    });
}

/// The number of pending affirmations can't include receivers that have pre-affirmed the ticker.
#[test]
fn add_instruction_with_pre_affirmed_tickers() {
    ExtBuilder::default().build().execute_with(|| {
        // Setup base parameters
        let alice = User::new(AccountKeyring::Alice);
        let alice_default_portfolio = PortfolioId::default_portfolio(alice.did);
        let bob = User::new(AccountKeyring::Bob);
        let bob_default_portfolio = PortfolioId::default_portfolio(bob.did);
        let bob_user_porfolio = PortfolioId::user_portfolio(bob.did, PortfolioNumber(1));
        let venue = create_token_and_venue(TICKER, alice);
        let instruction_memo = Some(Memo::default());
        Portfolio::create_portfolio(bob.origin(), b"BobUserPortfolio".into()).unwrap();

        // Both users have pre-affirmed the ticker
        Asset::pre_approve_ticker(alice.origin(), TICKER).unwrap();
        Asset::pre_approve_ticker(bob.origin(), TICKER).unwrap();

        let legs: Vec<Leg> = vec![
            Leg::Fungible {
                sender: alice_default_portfolio,
                receiver: bob_default_portfolio,
                ticker: TICKER,
                amount: ONE_UNIT,
            },
            Leg::Fungible {
                sender: alice_default_portfolio,
                receiver: bob_user_porfolio,
                ticker: TICKER,
                amount: ONE_UNIT,
            },
        ];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs.clone(),
            instruction_memo.clone(),
        ));
        // Only the sender still has to approve the transfer
        let portfolios_pending_approval = BTreeSet::from([alice_default_portfolio]);
        let portfolios_pre_approved = BTreeSet::from([bob_user_porfolio, bob_default_portfolio]);
        let instruction_id = InstructionId(0);
        assert_add_instruction_storage(
            &instruction_id,
            &portfolios_pending_approval,
            &portfolios_pre_approved,
            &BTreeSet::new(),
            instruction_memo,
            &legs,
        );
    });
}

/// The number of pending affirmations must include receivers that have pre-affirmed the ticker, but
/// have assigned custodians that have not pre-affirmed the portfolio.
#[test]
fn add_instruction_with_pre_affirmed_tickers_with_assigned_custodian() {
    ExtBuilder::default().build().execute_with(|| {
        // Setup base parameters
        let bob = User::new(AccountKeyring::Bob);
        let alice = User::new(AccountKeyring::Alice);
        let charlie = User::new(AccountKeyring::Charlie);
        let bob_default_portfolio = PortfolioId::default_portfolio(bob.did);
        let alice_default_portfolio = PortfolioId::default_portfolio(alice.did);
        let bob_user_porfolio = PortfolioId::user_portfolio(bob.did, PortfolioNumber(1));
        let venue = create_token_and_venue(TICKER, alice);
        let instruction_memo = Some(Memo::default());
        Portfolio::create_portfolio(bob.origin(), b"BobUserPortfolio".into()).unwrap();

        // Both users have pre-affirmed the ticker
        Asset::pre_approve_ticker(alice.origin(), TICKER).unwrap();
        Asset::pre_approve_ticker(bob.origin(), TICKER).unwrap();

        // Bob assigns a custodian to its user portfolio
        let authorization_id = Identity::add_auth(
            bob.did,
            Signatory::from(charlie.did),
            AuthorizationData::PortfolioCustody(bob_user_porfolio),
            None,
        );
        Portfolio::accept_portfolio_custody(charlie.origin(), authorization_id).unwrap();

        let legs: Vec<Leg> = vec![
            Leg::Fungible {
                sender: alice_default_portfolio,
                receiver: bob_default_portfolio,
                ticker: TICKER,
                amount: ONE_UNIT,
            },
            Leg::Fungible {
                sender: alice_default_portfolio,
                receiver: bob_user_porfolio,
                ticker: TICKER,
                amount: ONE_UNIT,
            },
        ];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs.clone(),
            instruction_memo.clone(),
        ));
        // Both the sender and the custodian have to affirm the instruction
        let portfolios_pending_approval =
            BTreeSet::from([alice_default_portfolio, bob_user_porfolio]);
        let portfolios_pre_approved = BTreeSet::from([bob_default_portfolio]);
        let instruction_id = InstructionId(0);
        assert_add_instruction_storage(
            &instruction_id,
            &portfolios_pending_approval,
            &portfolios_pre_approved,
            &BTreeSet::new(),
            instruction_memo,
            &legs,
        );
    });
}

/// The number of pending affirmations can't include receivers that have pre-affirmed transfers to a portfolio.
#[test]
fn add_instruction_with_pre_affirmed_portfolio() {
    ExtBuilder::default().build().execute_with(|| {
        // Setup base parameters
        let alice = User::new(AccountKeyring::Alice);
        let alice_default_portfolio = PortfolioId::default_portfolio(alice.did);
        let alice_user_porfolio = PortfolioId::user_portfolio(alice.did, PortfolioNumber(1));
        let bob = User::new(AccountKeyring::Bob);
        let bob_default_portfolio = PortfolioId::default_portfolio(bob.did);
        let bob_user_porfolio = PortfolioId::user_portfolio(bob.did, PortfolioNumber(1));
        let venue = create_token_and_venue(TICKER, alice);
        let instruction_memo = Some(Memo::default());
        Portfolio::create_portfolio(bob.origin(), b"BobUserPortfolio".into()).unwrap();
        Portfolio::create_portfolio(alice.origin(), b"AliceUserPortfolio".into()).unwrap();

        // Both users have pre-affirmed their user portfolios
        Portfolio::pre_approve_portfolio(bob.origin(), TICKER, bob_user_porfolio).unwrap();
        Portfolio::pre_approve_portfolio(alice.origin(), TICKER, alice_user_porfolio).unwrap();

        let legs: Vec<Leg> = vec![
            Leg::Fungible {
                sender: alice_user_porfolio,
                receiver: bob_user_porfolio,
                ticker: TICKER,
                amount: ONE_UNIT,
            },
            Leg::Fungible {
                sender: alice_default_portfolio,
                receiver: bob_default_portfolio,
                ticker: TICKER,
                amount: ONE_UNIT,
            },
        ];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs.clone(),
            instruction_memo.clone(),
        ));
        // The sender has to approve both portfolios and the receiver only the default one
        let portfolios_pending_approval = BTreeSet::from([
            alice_default_portfolio,
            alice_user_porfolio,
            bob_default_portfolio,
        ]);
        let portfolios_pre_approved = BTreeSet::from([bob_user_porfolio]);
        let instruction_id = InstructionId(0);
        assert_add_instruction_storage(
            &instruction_id,
            &portfolios_pending_approval,
            &portfolios_pre_approved,
            &BTreeSet::new(),
            instruction_memo,
            &legs,
        );
    });
}

/// In case a single not pre-affirmed asset is transferred to a portfolio, the number of pending
/// affirmations must include that portfolio.
#[test]
fn add_instruction_with_single_pre_affirmed() {
    ExtBuilder::default().build().execute_with(|| {
        // Setup base parameters
        let alice = User::new(AccountKeyring::Alice);
        let alice_default_portfolio = PortfolioId::default_portfolio(alice.did);
        let bob = User::new(AccountKeyring::Bob);
        let bob_default_portfolio = PortfolioId::default_portfolio(bob.did);
        let venue = create_token_and_venue(TICKER, alice);
        let instruction_memo = Some(Memo::default());
        create_token(TICKER2, alice);

        // Bob has pre-affirmed TICKER but not TICKER2
        Asset::pre_approve_ticker(bob.origin(), TICKER).unwrap();
        Asset::pre_approve_ticker(alice.origin(), TICKER).unwrap();
        Asset::pre_approve_ticker(alice.origin(), TICKER2).unwrap();

        let legs: Vec<Leg> = vec![
            Leg::Fungible {
                sender: alice_default_portfolio,
                receiver: bob_default_portfolio,
                ticker: TICKER,
                amount: ONE_UNIT,
            },
            Leg::Fungible {
                sender: alice_default_portfolio,
                receiver: bob_default_portfolio,
                ticker: TICKER2,
                amount: ONE_UNIT,
            },
        ];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs.clone(),
            instruction_memo.clone(),
        ));
        // Both the sender and receiver have to affirm their portfolio
        let portfolios_pending_approval =
            BTreeSet::from([alice_default_portfolio, bob_default_portfolio]);
        let instruction_id = InstructionId(0);
        assert_add_instruction_storage(
            &instruction_id,
            &portfolios_pending_approval,
            &BTreeSet::new(),
            &BTreeSet::new(),
            instruction_memo,
            &legs,
        );
    });
}

/// Successfully executes an instruction after one failed attempt.
#[test]
fn manually_execute_failed_instruction() {
    ExtBuilder::default().build().execute_with(|| {
        // Setup base parameters
        let alice = User::new(AccountKeyring::Alice);
        let alice_default_portfolio = PortfolioId::default_portfolio(alice.did);
        let bob = User::new(AccountKeyring::Bob);
        let bob_default_portfolio = PortfolioId::default_portfolio(bob.did);
        let venue = create_token_and_venue(TICKER, alice);
        let instruction_memo = Some(Memo::default());
        create_token(TICKER2, alice);

        // Creates and affirms an instruction and force a failed execution
        let legs: Vec<Leg> = vec![
            Leg::Fungible {
                sender: alice_default_portfolio,
                receiver: bob_default_portfolio,
                ticker: TICKER,
                amount: 1,
            },
            Leg::Fungible {
                sender: alice_default_portfolio,
                receiver: bob_default_portfolio,
                ticker: TICKER2,
                amount: 1,
            },
        ];
        assert_ok!(Settlement::add_and_affirm_instruction(
            alice.origin(),
            venue,
            SettlementType::SettleOnBlock(System::block_number() + 1),
            None,
            None,
            legs.clone(),
            vec![alice_default_portfolio],
            instruction_memo.clone(),
        ));
        assert_ok!(Settlement::affirm_instruction(
            bob.origin(),
            InstructionId(0),
            vec![bob_default_portfolio],
        ));
        assert_ok!(Asset::freeze(alice.origin(), TICKER));
        next_block();
        assert_instruction_status(InstructionId(0), InstructionStatus::Failed);
        assert_eq!(BalanceOf::get(TICKER, alice.did), 100_000);
        assert_eq!(BalanceOf::get(TICKER2, alice.did), 100_000);
        // Executes the instruction once again, now successfully.
        assert_ok!(Asset::unfreeze(alice.origin(), TICKER));
        assert_ok!(Settlement::execute_manual_instruction(
            alice.origin(),
            InstructionId(0),
            None,
            2,
            0,
            0,
            None
        ));
        assert_eq!(BalanceOf::get(TICKER, bob.did), 1);
        assert_eq!(BalanceOf::get(TICKER2, bob.did), 1);
        assert_eq!(BalanceOf::get(TICKER, alice.did), 99_999);
        assert_eq!(BalanceOf::get(TICKER2, alice.did), 99_999);
        assert_instruction_status(
            InstructionId(0),
            InstructionStatus::Success(System::block_number()),
        );
    });
}

#[test]
fn affirm_with_receipts_cost() {
    ExtBuilder::default().build().execute_with(|| {
        let charlie = User::new(AccountKeyring::Charlie);
        let alice = User::new(AccountKeyring::Alice);
        let bob = User::new(AccountKeyring::Bob);
        let venue_id = create_token_and_venue(TICKER, alice);
        let amount = 1;
        let id = InstructionId(0);

        let legs: Vec<Leg> = vec![Leg::OffChain {
            sender_identity: charlie.did,
            receiver_identity: bob.did,
            ticker: TICKER,
            amount,
        }];
        let receipt = Receipt::new(0, id, LegId(0), charlie.did, bob.did, TICKER, amount);
        let receipts_details = vec![ReceiptDetails::new(
            0,
            id,
            LegId(0),
            AccountKeyring::Alice.to_account_id(),
            AccountKeyring::Alice.sign(&receipt.encode()).into(),
            None,
        )];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue_id,
            SettlementType::SettleManual(System::block_number() + 1),
            None,
            None,
            legs,
            Some(Memo::default()),
        ),);

        let affirmation_count =
            AffirmationCount::new(AssetCount::default(), AssetCount::default(), 0);
        assert_noop!(
            Settlement::affirm_with_receipts_with_count(
                alice.origin(),
                id,
                receipts_details,
                Vec::new(),
                Some(affirmation_count)
            ),
            Error::NumberOfOffChainTransfersUnderestimated
        );
    });
}

#[test]
fn affirm_instruction_cost() {
    ExtBuilder::default().build().execute_with(|| {
        // Setup base parameters
        let alice = User::new(AccountKeyring::Alice);
        let alice_default_portfolio = PortfolioId::default_portfolio(alice.did);
        let alice_user_porfolio = PortfolioId::user_portfolio(alice.did, PortfolioNumber(1));
        let bob = User::new(AccountKeyring::Bob);
        let bob_default_portfolio = PortfolioId::default_portfolio(bob.did);
        let bob_user_porfolio = PortfolioId::user_portfolio(bob.did, PortfolioNumber(1));
        let venue = create_token_and_venue(TICKER, alice);
        let instruction_memo = Some(Memo::default());
        Portfolio::create_portfolio(bob.origin(), b"BobUserPortfolio".into()).unwrap();
        Portfolio::create_portfolio(alice.origin(), b"AliceUserPortfolio".into()).unwrap();

        let legs: Vec<Leg> = vec![
            Leg::Fungible {
                sender: alice_user_porfolio,
                receiver: bob_user_porfolio,
                ticker: TICKER,
                amount: ONE_UNIT,
            },
            Leg::Fungible {
                sender: alice_default_portfolio,
                receiver: bob_default_portfolio,
                ticker: TICKER,
                amount: ONE_UNIT,
            },
        ];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs.clone(),
            instruction_memo.clone(),
        ));

        let affirmation_count =
            AffirmationCount::new(AssetCount::new(0, 0, 0), AssetCount::default(), 0);
        assert_noop!(
            Settlement::affirm_instruction_with_count(
                alice.origin(),
                InstructionId(0),
                vec![alice_user_porfolio, alice_default_portfolio],
                Some(affirmation_count)
            ),
            Error::NumberOfFungibleTransfersUnderestimated
        );
        let affirmation_count =
            AffirmationCount::new(AssetCount::default(), AssetCount::new(1, 0, 0), 0);
        assert_noop!(
            Settlement::affirm_instruction_with_count(
                bob.origin(),
                InstructionId(0),
                vec![bob_user_porfolio, bob_default_portfolio],
                Some(affirmation_count)
            ),
            Error::NumberOfFungibleTransfersUnderestimated
        );
    });
}

#[test]
fn withdraw_affirmation_cost() {
    ExtBuilder::default().build().execute_with(|| {
        // Setup base parameters
        let alice = User::new(AccountKeyring::Alice);
        let alice_default_portfolio = PortfolioId::default_portfolio(alice.did);
        let bob = User::new(AccountKeyring::Bob);
        let bob_default_portfolio = PortfolioId::default_portfolio(bob.did);
        let venue = create_token_and_venue(TICKER, alice);
        let instruction_memo = Some(Memo::default());

        let legs: Vec<Leg> = vec![Leg::Fungible {
            sender: alice_default_portfolio,
            receiver: bob_default_portfolio,
            ticker: TICKER,
            amount: 1,
        }];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs.clone(),
            instruction_memo.clone(),
        ));

        let affirmation_count =
            AffirmationCount::new(AssetCount::new(1, 0, 0), AssetCount::default(), 0);
        assert_ok!(Settlement::affirm_instruction_with_count(
            alice.origin(),
            InstructionId(0),
            vec![alice_default_portfolio],
            Some(affirmation_count)
        ),);
        let affirmation_count =
            AffirmationCount::new(AssetCount::new(0, 0, 0), AssetCount::default(), 0);
        assert_noop!(
            Settlement::withdraw_affirmation_with_count(
                alice.origin(),
                InstructionId(0),
                vec![alice_default_portfolio],
                Some(affirmation_count)
            ),
            Error::NumberOfFungibleTransfersUnderestimated
        );
    });
}

#[test]
fn reject_instruction_cost() {
    ExtBuilder::default().build().execute_with(|| {
        // Setup base parameters
        let alice = User::new(AccountKeyring::Alice);
        let alice_default_portfolio = PortfolioId::default_portfolio(alice.did);
        let bob = User::new(AccountKeyring::Bob);
        let bob_default_portfolio = PortfolioId::default_portfolio(bob.did);
        let venue = create_token_and_venue(TICKER, alice);
        let instruction_memo = Some(Memo::default());

        create_nft_collection(
            alice.clone(),
            TICKER2,
            AssetType::NonFungible(NonFungibleType::Derivative),
            NFTCollectionKeys::default(),
        );
        mint_nft(alice.clone(), TICKER2, Vec::new(), PortfolioKind::Default);

        let legs: Vec<Leg> = vec![
            Leg::Fungible {
                sender: alice_default_portfolio,
                receiver: bob_default_portfolio,
                ticker: TICKER,
                amount: 1,
            },
            Leg::NonFungible {
                sender: alice_default_portfolio,
                receiver: bob_default_portfolio,
                nfts: NFTs::new_unverified(TICKER2, vec![NFTId(1)]),
            },
        ];
        assert_ok!(Settlement::add_instruction(
            alice.origin(),
            venue,
            SettlementType::SettleOnAffirmation,
            None,
            None,
            legs.clone(),
            instruction_memo.clone(),
        ));

        assert_noop!(
            Settlement::reject_instruction_with_count(
                bob.origin(),
                InstructionId(0),
                bob_default_portfolio,
                Some(AssetCount::new(1, 0, 0))
            ),
            Error::NumberOfTransferredNFTsUnderestimated
        );
        assert_ok!(Settlement::reject_instruction_with_count(
            bob.origin(),
            InstructionId(0),
            bob_default_portfolio,
            Some(AssetCount::new(1, 1, 0))
        ),);
    });
}

/// Asserts the storage has been updated after adding an instruction.
/// While each portfolio in `portfolios_pending_approval` must have a pending `AffirmationStatus`, each portfolio in `portfolios_pre_approved`
/// must have an affirmed status. The number of pending affirmations must be equal to the number of portfolios in `portfolios_pending_approval` + the number of offchain legs,
/// all legs must have been included in `InstructionLegs` and `InstructionMemos` must be equal to `instruction_memo`.
fn assert_add_instruction_storage(
    instruction_id: &InstructionId,
    portfolios_pending_approval: &BTreeSet<PortfolioId>,
    portfolios_pre_approved: &BTreeSet<PortfolioId>,
    offchain_legs: &BTreeSet<LegId>,
    instruction_memo: Option<Memo>,
    legs: &[Leg],
) {
    portfolios_pending_approval.iter().for_each(|portfolio_id| {
        assert_eq!(
            UserAffirmations::get(portfolio_id, instruction_id),
            AffirmationStatus::Pending
        )
    });
    portfolios_pre_approved.iter().for_each(|portfolio_id| {
        assert_eq!(
            UserAffirmations::get(portfolio_id, instruction_id),
            AffirmationStatus::Affirmed
        );
        assert_eq!(
            AffirmsReceived::get(instruction_id, portfolio_id),
            AffirmationStatus::Affirmed
        )
    });
    offchain_legs.iter().for_each(|leg_id| {
        assert_eq!(
            OffChainAffirmations::get(instruction_id, leg_id),
            AffirmationStatus::Pending
        );
    });
    assert_eq!(
        InstructionAffirmsPending::get(instruction_id),
        portfolios_pending_approval.len() as u64 + offchain_legs.len() as u64
    );

    assert_eq!(InstructionMemos::get(instruction_id), instruction_memo);

    (0..legs.len()).for_each(|i| {
        assert_eq!(
            InstructionLegs::get(instruction_id, LegId(i as u64)),
            Some(legs[i].clone())
        )
    });
}

#[track_caller]
fn assert_instruction_details(
    instruction_id: InstructionId,
    details: Instruction<Moment, BlockNumber>,
) {
    assert_eq!(Settlement::instruction_details(instruction_id), details);
}

#[track_caller]
fn assert_instruction_status(
    instruction_id: InstructionId,
    status: InstructionStatus<BlockNumber>,
) {
    assert_eq!(Settlement::instruction_status(instruction_id), status);
}

#[track_caller]
fn assert_balance(ticker: &Ticker, user: &User, balance: Balance) {
    assert_eq!(Asset::balance_of(&ticker, user.did), balance);
}

#[track_caller]
fn assert_user_affirms(instruction_id: InstructionId, user: &User, status: AffirmationStatus) {
    assert_eq!(
        Settlement::user_affirmations(PortfolioId::default_portfolio(user.did), instruction_id),
        status
    );

    let affirms_received_status = match status {
        AffirmationStatus::Pending => AffirmationStatus::Unknown,
        AffirmationStatus::Affirmed => AffirmationStatus::Affirmed,
        _ => return,
    };

    assert_eq!(
        Settlement::affirms_received(instruction_id, PortfolioId::default_portfolio(user.did)),
        affirms_received_status
    );
}

#[track_caller]
fn assert_leg_status(instruction_id: InstructionId, leg: LegId, status: LegStatus<AccountId>) {
    assert_eq!(
        Settlement::instruction_leg_status(instruction_id, leg),
        status
    );
}

#[track_caller]
fn assert_affirms_pending(instruction_id: InstructionId, pending: u64) {
    assert_eq!(
        Settlement::instruction_affirms_pending(instruction_id),
        pending
    );
}

#[track_caller]
fn assert_locked_assets(ticker: &Ticker, user: &User, num_of_assets: Balance) {
    assert_eq!(
        Portfolio::locked_assets(PortfolioId::default_portfolio(user.did), ticker),
        num_of_assets
    );
}
