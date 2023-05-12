use super::{
    storage::{TestStorage, User},
    ExtBuilder,
};
use frame_support::assert_ok;

use mercat::{
    confidential_identity_core::asset_proofs::Balance as MercatBalance,
    transaction::{CtxMediator, CtxReceiver, CtxSender},
    Account, AmountSource, EncryptedAmount, PubAccount, SecAccount, TransferTransactionMediator,
    TransferTransactionReceiver, TransferTransactionSender,
};
use pallet_confidential_asset::{
    AffirmLeg, MercatAccount, TransactionId, TransactionLeg, TransactionLegId, VenueId,
};
use polymesh_primitives::Ticker;
use rand::prelude::*;
use test_client::AccountKeyring;

use super::confidential_asset_test::{create_account_and_mint_token, init_account};

type ConfidentialAsset = pallet_confidential_asset::Module<TestStorage>;

#[derive(Clone)]
struct AccountCredentials {
    user: User,
    account: MercatAccount,
    public_account: PubAccount,
}

#[derive(Clone)]
struct MediatorCredentials {
    user: User,
    mediator_public_account: PubAccount,
    ticker: Ticker,
}

fn initialize_transaction(
    ticker: Ticker,
    sender_secret_account: SecAccount,
    sender_creds: AccountCredentials,
    sender_pending_enc_balance: EncryptedAmount,
    receiver_secret_account: SecAccount,
    receiver_creds: AccountCredentials,
    mediator_creds: MediatorCredentials,
    amount: MercatBalance,
    validation_failure_expected: bool,
) -> Option<(TransactionId, EncryptedAmount, EncryptedAmount)> {
    // The rest of rngs are built from it.
    let mut rng = StdRng::from_seed([10u8; 32]);

    // Mediator creates a venue.
    let venue_counter = VenueId(0); /*ConfidentialAsset::venue_counter();
                                    assert_ok!(ConfidentialAsset::create_venue(
                                        mediator_creds.user.origin(),
                                        VenueDetails::default(),
                                        vec![mediator_creds.user.acc()],
                                        VenueType::Other
                                    ));
                                    */

    // Mediator creates an transaction.
    let transaction_id = ConfidentialAsset::transaction_counter();
    let leg_id = TransactionLegId(0);

    assert_ok!(ConfidentialAsset::add_transaction(
        mediator_creds.user.origin(),
        venue_counter,
        vec![TransactionLeg {
            ticker,
            sender: sender_creds.account.clone(),
            receiver: receiver_creds.account.clone(),
            mediator: mediator_creds.user.did,
        }]
    ));

    let sender_pending_balance = sender_secret_account
        .enc_keys
        .secret
        .decrypt(&sender_pending_enc_balance)
        .unwrap();
    // Sender authorizes.
    // Sender computes the proofs in the wallet.
    let sender_tx = CtxSender
        .create_transaction(
            &Account {
                public: sender_creds.public_account.clone(),
                secret: sender_secret_account.clone(),
            },
            &sender_pending_enc_balance,
            sender_pending_balance,
            &receiver_creds.public_account,
            Some(&mediator_creds.mediator_public_account.owner_enc_pub_key),
            &[],
            amount,
            &mut rng,
        )
        .unwrap();
    let initialized_tx = AffirmLeg::new_sender(leg_id, sender_tx);
    // Sender authorizes the transaction and passes in the proofs.
    let result = ConfidentialAsset::affirm_transaction(
        sender_creds.user.origin(),
        transaction_id,
        initialized_tx,
    );

    if validation_failure_expected {
        assert!(result.is_err());
        return None;
    } else {
        assert!(result.is_ok());
    }

    // Receiver authorizes.
    // Receiver reads the sender's proof from the chain.
    let sender_proof =
        ConfidentialAsset::sender_proofs(transaction_id, leg_id).expect("Sender proof");
    let init_tx = sender_proof.0;
    let sender_encrypted_transfer_amount = init_tx.memo.enc_amount_using_sender;
    let receiver_encrypted_transfer_amount = init_tx.memo.enc_amount_using_receiver;

    // Receiver computes the proofs in the wallet.
    CtxReceiver
        .finalize_transaction(
            &init_tx,
            Account {
                public: receiver_creds.public_account.clone(),
                secret: receiver_secret_account.clone(),
            },
            amount,
        )
        .unwrap();
    let finalized_tx = AffirmLeg::new_receiver(leg_id);

    // Receiver submits the proof to the chain.
    assert_ok!(ConfidentialAsset::affirm_transaction(
        receiver_creds.user.origin(),
        transaction_id,
        finalized_tx,
    ));

    Some((
        transaction_id,
        sender_encrypted_transfer_amount,
        receiver_encrypted_transfer_amount,
    ))
}

fn decrypt_balance(secret_account: &SecAccount, balance: &EncryptedAmount) -> MercatBalance {
    secret_account.enc_keys.secret.decrypt(balance).unwrap()
}

fn finalize_transaction(
    ticker: Ticker,
    transaction_id: TransactionId,
    sender_creds: AccountCredentials,
    receiver_creds: AccountCredentials,
    mediator_creds: MediatorCredentials,
    mediator_secret_account: SecAccount,
    expected_sender_balance: EncryptedAmount,
    expected_receiver_balance: EncryptedAmount,
    sender_secret_account: Option<SecAccount>,
    receiver_secret_account: Option<SecAccount>,
    validation_failure_expected: bool,
) {
    // The rest of rngs are built from it.
    let mut rng = StdRng::from_seed([10u8; 32]);
    let leg_id = TransactionLegId(0);

    // Mediator authorizes.
    // Mediator reads the receiver's proofs from the chain (it contains the sender's proofs as well).
    let sender_proof =
        ConfidentialAsset::sender_proofs(transaction_id, leg_id).expect("Sender proof");
    let init_tx = sender_proof.0;

    // Mediator verifies the proofs in the wallet.
    // Mediator has access to the ticker name in plaintext.
    // Mediator gets the pending state for this transaction from chain.
    let sender_pending_balance = *ConfidentialAsset::mercat_tx_pending_state((
        &sender_creds.account,
        ticker,
        transaction_id,
    ));

    let result = CtxMediator.justify_transaction(
        &init_tx,
        AmountSource::Encrypted(&mediator_secret_account.enc_keys),
        &sender_creds.public_account,
        &sender_pending_balance,
        &receiver_creds.public_account,
        &[],
        &mut rng,
    );

    if validation_failure_expected {
        assert!(result.is_err());
        return;
    }

    let justified_tx = AffirmLeg::new_mediator(leg_id);

    // Affirms and process the transaction.
    assert_ok!(ConfidentialAsset::affirm_transaction(
        mediator_creds.user.origin(),
        transaction_id,
        justified_tx,
    ));

    // Execute affirmed transaction.
    assert_ok!(ConfidentialAsset::execute_transaction(
        mediator_creds.user.origin(),
        transaction_id,
        1,
    ));

    // Transaction should've settled.
    // Verify by decrypting the new balance of both Sender and Receiver.
    let new_sender_balance =
        *ConfidentialAsset::mercat_account_balance(&sender_creds.account, ticker);

    if let Some(secret_account) = sender_secret_account {
        // Invoked for debugging
        let new_balance_plain = decrypt_balance(&secret_account, &new_sender_balance);
        let expected_balance_plain = decrypt_balance(&secret_account, &expected_sender_balance);
        assert_eq!(new_balance_plain, expected_balance_plain, "Sender side");
    }
    assert_eq!(new_sender_balance, expected_sender_balance);

    let new_receiver_balance =
        *ConfidentialAsset::mercat_account_balance(&receiver_creds.account, ticker);

    if let Some(secret_account) = receiver_secret_account {
        // Invoked for debugging
        let new_balance_plain = decrypt_balance(&secret_account, &new_receiver_balance);
        let expected_balance_plain = decrypt_balance(&secret_account, &expected_receiver_balance);
        assert_eq!(new_balance_plain, expected_balance_plain, "Receiver side");
    }
    assert_eq!(new_receiver_balance, expected_receiver_balance);
}

fn chain_set_up(
    total_supply: u128,
) -> (
    StdRng,
    Ticker,
    AccountCredentials,
    SecAccount,
    EncryptedAmount,
    MediatorCredentials,
    SecAccount,
) {
    // The rest of rngs are built from it.
    let mut rng = StdRng::from_seed([10u8; 32]);

    // Setting:
    //   - Alice is the token issuer.
    //   - Alice is also the sender of the token.
    //   - Bob is the receiver of the token.
    //   - Charlie is the mediator.
    let alice = User::new(AccountKeyring::Alice);

    let charlie = User::new(AccountKeyring::Charlie);

    // Setup a mercat asset.
    let token_name = b"ACME";
    let ticker = Ticker::from_slice_truncated(&token_name[..]);

    // Create an account for Alice and mint `total_supply` tokens to ACME.
    let (alice_secret_account, alice_account, alice_public_account, alice_encrypted_init_balance) =
        create_account_and_mint_token(
            alice, // owner of ACME.
            total_supply,
            token_name.to_vec(),
            &mut rng,
        );

    let alice_creds = AccountCredentials {
        user: alice,
        account: alice_account,
        public_account: alice_public_account,
    };

    // Create an account for Charlie.
    let (charlie_secret_account, _, charlie_public_account, _) =
        init_account(&mut rng, ticker, charlie);
    let charlie_creds = MediatorCredentials {
        user: charlie,
        mediator_public_account: charlie_public_account,
        ticker,
    };

    (
        rng,
        ticker,
        alice_creds,
        alice_secret_account,
        alice_encrypted_init_balance,
        charlie_creds,
        charlie_secret_account,
    )
}

fn create_investor_account(
    key: AccountKeyring,
    mut rng: &mut StdRng,
) -> (SecAccount, AccountCredentials, EncryptedAmount) {
    let token_name = b"ACME";
    // Create accounts for the key holder.
    let user = User::new(key);

    let (secret_account, account, public_account, init_balance) = init_account(
        &mut rng,
        Ticker::from_slice_truncated(token_name.as_ref()),
        user,
    );

    let creds = AccountCredentials {
        user,
        account,
        public_account,
    };

    (secret_account, creds, init_balance)
}

#[test]
fn settle_out_of_order() {
    ExtBuilder::default()
        .cdd_providers(vec![AccountKeyring::Eve.to_account_id()])
        .build()
        .execute_with(|| {
            // Setting:
            //   - Alice is the token issuer, and has 10 assets in her supply.
            //   - Bob has a normal account.
            //   - Charlie is the mediator.
            //   - Eve is the CDD provider.
            let (
                mut rng,
                ticker,
                alice_creds,
                alice_secret_account,
                alice_init_balance,
                charlie_creds,
                charlie_secret_account,
            ) = chain_set_up(10u128);

            let (bob_secret_account, bob_creds, bob_init_balance) =
                create_investor_account(AccountKeyring::Bob, &mut rng);

            // tx_id:1000 => Alice sends 5 assets to Bob.
            // tx_id:1001 => Alice sends 3 assets to Bob.
            //            => Charlie (the mediator) approves tx_id:1001 first.
            //            => Charlie (the mediator) approves tx_id:1000 second.
            let (transaction_id1000, alice_sent_amount_1000, bob_received_amount_1000) =
                initialize_transaction(
                    ticker,
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    alice_init_balance.clone(),
                    bob_secret_account.clone(),
                    bob_creds.clone(),
                    charlie_creds.clone(),
                    5,
                    false,
                )
                .expect("initialized_tx");

            let alice_init_balance2 = alice_init_balance - alice_sent_amount_1000;
            let (transaction_id1001, alice_sent_amount_1001, bob_received_amount_1001) =
                initialize_transaction(
                    ticker,
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    alice_init_balance2.clone(),
                    bob_secret_account.clone(),
                    bob_creds.clone(),
                    charlie_creds.clone(),
                    3,
                    false,
                )
                .expect("initialized_tx");

            // Approve and process tx:1001.
            finalize_transaction(
                ticker,
                transaction_id1001,
                alice_creds.clone(),
                bob_creds.clone(),
                charlie_creds.clone(),
                charlie_secret_account.clone(),
                alice_init_balance - alice_sent_amount_1001,
                bob_init_balance + bob_received_amount_1001,
                None,
                None,
                false,
            );

            // Approve and process tx:1000.
            finalize_transaction(
                ticker,
                transaction_id1000,
                alice_creds.clone(),
                bob_creds.clone(),
                charlie_creds.clone(),
                charlie_secret_account.clone(),
                alice_init_balance - alice_sent_amount_1001 - alice_sent_amount_1000,
                bob_init_balance + bob_received_amount_1001 + bob_received_amount_1000,
                None,
                None,
                false,
            );
        });
}

#[test]
fn double_spending_fails() {
    ExtBuilder::default()
        .cdd_providers(vec![AccountKeyring::Eve.to_account_id()])
        .build()
        .execute_with(|| {
            // Setting:
            //   - Alice is the token issuer, and has 10 assets in her supply.
            //   - Bob has a normal account.
            //   - Charlie is the mediator.
            //   - Eve is the CDD provider.
            let (
                mut rng,
                ticker,
                alice_creds,
                alice_secret_account,
                alice_init_balance,
                charlie_creds,
                charlie_secret_account,
            ) = chain_set_up(10u128);

            let (bob_secret_account, bob_creds, bob_init_balance) =
                create_investor_account(AccountKeyring::Bob, &mut rng);

            let (dave_secret_account, dave_creds, _dave_init_balance) =
                create_investor_account(AccountKeyring::Dave, &mut rng);

            // Alice has 10 assets.
            // tx_id:1000 => Alice sends 5 assets to Bob.
            // tx_id:1001 => Alice sends 10 assets to Dave.
            //            => Charlie (the mediator) catches tx_id:1001's double spend.
            //            => Charlie (the mediator) approves tx_id:1000.
            let (transaction_id1000, alice_sent_amount_1000, bob_received_amount_1000) =
                initialize_transaction(
                    ticker,
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    alice_init_balance.clone(),
                    bob_secret_account.clone(),
                    bob_creds.clone(),
                    charlie_creds.clone(),
                    5,
                    false,
                )
                .expect("initialized_tx");

            assert!(initialize_transaction(
                ticker,
                alice_secret_account.clone(),
                alice_creds.clone(),
                // Alice is reusing her initial balance as the pending balance.
                // This is an attempt to double spend.
                // She should have used `alice_init_balance - alice_sent_amount_1000`.
                alice_init_balance.clone(),
                dave_secret_account.clone(),
                dave_creds.clone(),
                charlie_creds.clone(),
                10,
                true,
            )
            .is_none());

            // Mediator fails the tx:1001.
            // The sender can't affirm a double spend.
            //finalize_transaction(
            //    ticker,
            //    transaction_id1001,
            //    alice_creds.clone(),
            //    dave_creds.clone(),
            //    charlie_creds.clone(),
            //    charlie_secret_account.clone(),
            //    alice_init_balance - alice_sent_amount_1001,
            //    dave_init_balance + dave_received_amount_1001,
            //    None,
            //    None,
            //    true, // Validation failure expected.
            //);

            // Approve and process tx:1000.
            finalize_transaction(
                ticker,
                transaction_id1000,
                alice_creds.clone(),
                bob_creds.clone(),
                charlie_creds.clone(),
                charlie_secret_account.clone(),
                alice_init_balance - alice_sent_amount_1000,
                bob_init_balance + bob_received_amount_1000,
                None,
                None,
                false,
            );
        });
}

#[test]
fn mercat_whitepaper_scenario1() {
    ExtBuilder::default()
        .cdd_providers(vec![AccountKeyring::Eve.to_account_id()])
        .build()
        .execute_with(|| {
            // Setting:
            //   - Alice is the token issuer, and has 90 assets in her supply.
            //   - Bob has a normal account.
            //   - Dave has a normal account.
            //   - Charlie is the mediator.
            //   - Eve is the CDD provider.
            let (
                mut rng,
                ticker,
                alice_creds,
                alice_secret_account,
                alice_init_balance,
                charlie_creds,
                charlie_secret_account,
            ) = chain_set_up(90u128);

            let (bob_secret_account, bob_creds, _) =
                create_investor_account(AccountKeyring::Bob, &mut rng);

            let (dave_secret_account, dave_creds, dave_init_balance) =
                create_investor_account(AccountKeyring::Dave, &mut rng);

            // Alice, the token issuer, sends 10 tokens to Dave so he has something in his account.
            let (transaction_id999, alice_sent_amount_999, dave_received_amount_999) =
                initialize_transaction(
                    ticker,
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    alice_init_balance.clone(),
                    dave_secret_account.clone(),
                    dave_creds.clone(),
                    charlie_creds.clone(),
                    10,
                    false,
                )
                .expect("initialized_tx");
            finalize_transaction(
                ticker,
                transaction_id999,
                alice_creds.clone(),
                dave_creds.clone(),
                charlie_creds.clone(),
                charlie_secret_account.clone(),
                alice_init_balance - alice_sent_amount_999,
                dave_init_balance + dave_received_amount_999,
                Some(alice_secret_account.clone()),
                Some(dave_secret_account.clone()),
                false,
            );
            // Reset Dave's pending state.
            assert_ok!(ConfidentialAsset::reset_ordering_state(
                dave_creds.user.origin(),
                dave_creds.account.clone(),
                ticker
            ));
            let dave_init_balance = dave_init_balance + dave_received_amount_999;
            let alice_init_balance = alice_init_balance - alice_sent_amount_999;

            // tx_id:1000 => Alice sends 10 assets to Bob.
            // tx_id:1001 => Alice receives 8 tokens from Dave.
            // tx_id:1002 => Alice sends 14 tokens to Dave.
            //            => Charlie (the mediator) fails tx_id:1000.
            //            => Charlie (the mediator) approves tx_id:1001.
            //            => Charlie (the mediator) approves tx_id:1002.
            let (_transaction_id1000, alice_sent_amount_1000, _bob_received_amount_1000) =
                initialize_transaction(
                    ticker,
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    alice_init_balance.clone(),
                    bob_secret_account.clone(),
                    bob_creds.clone(),
                    charlie_creds.clone(),
                    10,
                    false,
                )
                .expect("initialized_tx");
            let alice_pending_balance = alice_init_balance - alice_sent_amount_1000;

            let (transaction_id1001, dave_sent_amount_1001, alice_received_amount_1001) =
                initialize_transaction(
                    ticker,
                    dave_secret_account.clone(),
                    dave_creds.clone(),
                    dave_init_balance.clone(),
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    charlie_creds.clone(),
                    8,
                    false,
                )
                .expect("initialized_tx");

            let (transaction_id1002, alice_sent_amount_1002, dave_received_amount_1002) =
                initialize_transaction(
                    ticker,
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    alice_pending_balance.clone(),
                    dave_secret_account.clone(),
                    dave_creds.clone(),
                    charlie_creds.clone(),
                    14,
                    false,
                )
                .expect("initialized_tx");

            // Approve and process tx:1001.
            finalize_transaction(
                ticker,
                transaction_id1001,
                dave_creds.clone(),
                alice_creds.clone(),
                charlie_creds.clone(),
                charlie_secret_account.clone(),
                dave_init_balance - dave_sent_amount_1001,
                alice_init_balance + alice_received_amount_1001,
                Some(dave_secret_account.clone()),
                Some(alice_secret_account.clone()),
                false,
            );

            // Alice has a change of heart and rejects the transaction to Bob!
            /*
            TODO: add reject.
            assert_ok!(ConfidentialAsset::reject_transaction(
                alice_creds.user.origin(),
                transaction_id1000,
                PortfolioId::default_portfolio(alice_creds.user.did),
                1
            ));

            // Execute affirmed transaction.
            assert_ok!(ConfidentialAsset::execute_transaction(
                mediator_creds.user.origin(),
                transaction_id1000,
                1,
            ));
            */

            // Approve and process tx:1002.
            finalize_transaction(
                ticker,
                transaction_id1002,
                alice_creds.clone(),
                dave_creds.clone(),
                charlie_creds.clone(),
                charlie_secret_account.clone(),
                alice_init_balance + alice_received_amount_1001 - alice_sent_amount_1002,
                dave_init_balance - dave_sent_amount_1001 + dave_received_amount_1002,
                Some(alice_secret_account.clone()),
                Some(dave_secret_account.clone()),
                false,
            );
        });
}

#[test]
fn mercat_whitepaper_scenario2() {
    ExtBuilder::default()
        .cdd_providers(vec![AccountKeyring::Eve.to_account_id()])
        .build()
        .execute_with(|| {
            // Setting:
            //   - Alice is the token issuer, and has 90 assets in her supply.
            //   - Bob has a normal account.
            //   - Dave has a normal account.
            //   - Charlie is the mediator.
            //   - Eve is the CDD provider.
            let (
                mut rng,
                ticker,
                alice_creds,
                alice_secret_account,
                alice_init_balance,
                charlie_creds,
                charlie_secret_account,
            ) = chain_set_up(90u128);

            let (bob_secret_account, bob_creds, bob_init_balance) =
                create_investor_account(AccountKeyring::Bob, &mut rng);

            let (dave_secret_account, dave_creds, dave_init_balance) =
                create_investor_account(AccountKeyring::Dave, &mut rng);

            // Alice, the token issuer, sends 10 tokens to Dave so he has something in his account.
            let (transaction_id999, alice_sent_amount_999, dave_received_amount_999) =
                initialize_transaction(
                    ticker,
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    alice_init_balance.clone(),
                    dave_secret_account.clone(),
                    dave_creds.clone(),
                    charlie_creds.clone(),
                    10,
                    false,
                )
                .expect("initialized_tx");
            finalize_transaction(
                ticker,
                transaction_id999,
                alice_creds.clone(),
                dave_creds.clone(),
                charlie_creds.clone(),
                charlie_secret_account.clone(),
                alice_init_balance - alice_sent_amount_999,
                dave_init_balance + dave_received_amount_999,
                None,
                None,
                false,
            );
            // Reset Dave's pending state.
            assert_ok!(ConfidentialAsset::reset_ordering_state(
                dave_creds.user.origin(),
                dave_creds.account.clone(),
                ticker
            ));
            let dave_init_balance = dave_init_balance + dave_received_amount_999;
            let alice_init_balance = alice_init_balance - alice_sent_amount_999;

            // tx_id:1000 => Alice sends 10 assets to Bob.
            // tx_id:1001 => Alice receives 8 tokens from Dave.
            // tx_id:1002 => Alice sends 14 tokens to Dave.
            //            => Charlie (the mediator) fails tx_id:1000.
            //            => Charlie (the mediator) approves tx_id:1001.
            //            => Charlie (the mediator) approves tx_id:1002.
            // tx_id:1003 => Alice sends 19 assets to Bob.
            // Alice resets her pending state.
            // tx_id:1004 => Alice sends 55 assets to Dave.
            let (_transaction_id1000, alice_sent_amount_1000, _bob_received_amount_1000) =
                initialize_transaction(
                    ticker,
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    alice_init_balance.clone(),
                    bob_secret_account.clone(),
                    bob_creds.clone(),
                    charlie_creds.clone(),
                    10,
                    false,
                )
                .expect("initialized_tx");
            let alice_pending_balance = alice_init_balance - alice_sent_amount_1000;

            let (transaction_id1001, dave_sent_amount_1001, alice_received_amount_1001) =
                initialize_transaction(
                    ticker,
                    dave_secret_account.clone(),
                    dave_creds.clone(),
                    dave_init_balance.clone(),
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    charlie_creds.clone(),
                    8,
                    false,
                )
                .expect("initialized_tx");

            let (transaction_id1002, alice_sent_amount_1002, dave_received_amount_1002) =
                initialize_transaction(
                    ticker,
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    alice_pending_balance.clone(),
                    dave_secret_account.clone(),
                    dave_creds.clone(),
                    charlie_creds.clone(),
                    14,
                    false,
                )
                .expect("initialized_tx");
            let alice_pending_balance = alice_pending_balance - alice_sent_amount_1002;

            // Approve and process tx:1001.
            finalize_transaction(
                ticker,
                transaction_id1001,
                dave_creds.clone(),
                alice_creds.clone(),
                charlie_creds.clone(),
                charlie_secret_account.clone(),
                dave_init_balance - dave_sent_amount_1001,
                alice_init_balance + alice_received_amount_1001,
                None,
                None,
                false,
            );

            // Alice has a change of heart and rejects the transaction to Bob!
            /*
            TODO: add reject.
            assert_ok!(ConfidentialAsset::reject_transaction(
                alice_creds.user.origin(),
                transaction_id1000,
                PortfolioId::default_portfolio(alice_creds.user.did),
                1
            ));

            // Execute affirmed transaction.
            assert_ok!(ConfidentialAsset::execute_transaction(
                mediator_creds.user.origin(),
                transaction_id1000,
                1,
            ));
            */

            // Approve and process tx:1002.
            finalize_transaction(
                ticker,
                transaction_id1002,
                alice_creds.clone(),
                dave_creds.clone(),
                charlie_creds.clone(),
                charlie_secret_account.clone(),
                alice_init_balance + alice_received_amount_1001 - alice_sent_amount_1002,
                dave_init_balance - dave_sent_amount_1001 + dave_received_amount_1002,
                None,
                None,
                false,
            );

            // tx_id:1003 => Alice sends 19 assets to Bob.
            let (transaction_id1003, alice_sent_amount_1003, bob_received_amount_1003) =
                initialize_transaction(
                    ticker,
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    alice_pending_balance.clone(),
                    bob_secret_account.clone(),
                    bob_creds.clone(),
                    charlie_creds.clone(),
                    19,
                    false,
                )
                .expect("initialized_tx");

            // Reset Alice's pending state.
            assert_ok!(ConfidentialAsset::reset_ordering_state(
                alice_creds.user.origin(),
                alice_creds.account.clone(),
                ticker
            ));
            // On the Alice's wallet side, she also resets her pending state.
            let alice_init_balance =
                *ConfidentialAsset::mercat_account_balance(&alice_creds.account, ticker);
            // Since tx_1003 has not settled yet, it has to be accounted for in the pending balance.
            let alice_pending_balance = alice_init_balance - alice_sent_amount_1003;

            // tx_id:1004 => Alice sends 55 assets to Dave.
            let (transaction_id1004, alice_sent_amount_1004, dave_received_amount_1004) =
                initialize_transaction(
                    ticker,
                    alice_secret_account.clone(),
                    alice_creds.clone(),
                    alice_pending_balance.clone(),
                    dave_secret_account.clone(),
                    dave_creds.clone(),
                    charlie_creds.clone(),
                    55,
                    false,
                )
                .expect("initialized_tx");

            // Approve and process tx:1004.
            finalize_transaction(
                ticker,
                transaction_id1004,
                alice_creds.clone(),
                dave_creds.clone(),
                charlie_creds.clone(),
                charlie_secret_account.clone(),
                alice_init_balance - alice_sent_amount_1004,
                dave_init_balance - dave_sent_amount_1001
                    + dave_received_amount_1002
                    + dave_received_amount_1004,
                None,
                None,
                false,
            );

            // Approve and process tx:1003.
            finalize_transaction(
                ticker,
                transaction_id1003,
                alice_creds.clone(),
                bob_creds.clone(),
                charlie_creds.clone(),
                charlie_secret_account.clone(),
                alice_init_balance - alice_sent_amount_1004 - alice_sent_amount_1003,
                bob_init_balance + bob_received_amount_1003,
                None,
                None,
                false,
            );
        });
}
