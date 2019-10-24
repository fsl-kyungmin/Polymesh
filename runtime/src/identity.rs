use rstd::prelude::*;
//use codec::Codec;

pub static DID_PREFIX: &'static str = "did:poly:";
use crate::balances;

use codec::Encode;
use sr_primitives::traits::{CheckedAdd, CheckedSub};
use srml_support::{
    decl_event, decl_module, decl_storage,
    dispatch::Result,
    ensure,
    traits::{Currency, ExistenceRequirement, WithdrawReason},
};
use system::{self, ensure_signed};

#[derive(codec::Encode, codec::Decode, Default, Clone, PartialEq, Debug)]
pub struct Issuer {
    did: Vec<u8>,
    access_level: u16,
    active: bool,
}

#[derive(codec::Encode, codec::Decode, Default, Clone, PartialEq, Debug)]
pub struct Investor {
    pub did: Vec<u8>,
    pub access_level: u16,
    pub active: bool,
    pub jurisdiction: u16,
}

#[derive(codec::Encode, codec::Decode, Default, Clone, PartialEq, Debug)]
pub struct DidRecord<U> {
    pub master_key: Vec<u8>,
    pub signing_keys: Vec<Vec<u8>>,
    pub balance: U,
}

#[derive(codec::Encode, codec::Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct Claim<U> {
    issuance_date: U,
    expiry: U,
    claim_value: ClaimValue,
}

#[derive(codec::Encode, codec::Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct ClaimMetaData {
    claim_key: Vec<u8>,
    claim_issuer: Vec<u8>,
}

#[derive(codec::Encode, codec::Decode, Default, Clone, PartialEq, Eq, Debug)]
pub struct ClaimValue {
    pub data_type: DataTypes,
    pub value: Vec<u8>,
}

#[derive(codec::Encode, codec::Decode, Clone, Copy, PartialEq, Eq, Debug, PartialOrd, Ord)]
pub enum DataTypes {
    U8,
    U16,
    U32,
    U64,
    U128,
    Bool,
    VecU8,
}

impl Default for DataTypes {
    fn default() -> Self {
        DataTypes::VecU8
    }
}

/// The module's configuration trait.
pub trait Trait: system::Trait + balances::Trait + timestamp::Trait {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
    trait Store for Module<T: Trait> as identity {

        Owner get(owner) config(): T::AccountId;

        SimpleTokenIssuerList get(simple_token_issuer_list): map Vec<u8> => Issuer;
        IssuerList get(issuer_list): map Vec<u8> => Issuer;
        pub InvestorList get(investor_list): map Vec<u8> => Investor;

        /// DID -> identity info
        pub DidRecords get(did_records): map Vec<u8> => DidRecord<T::Balance>;

        /// DID -> DID claim issuers
        pub ClaimIssuers get(claim_issuers): map Vec<u8> => Vec<Vec<u8>>;

        /// (DID, claim_key, claim_issuer) -> Associated claims
        pub Claims get(claims): map(Vec<u8>, ClaimMetaData) => Claim<T::Moment>;

        /// DID -> array of (claim_key and claim_issuer)
        pub ClaimKeys get(claim_keys): map Vec<u8> => Vec<ClaimMetaData>;

        // Signing key => DID
        pub SigningKeyDid get(signing_key_did): map Vec<u8> => Vec<u8>;

        // Signing key => Charge Fee to did?. Default is false i.e. the fee will be charged from user balance
        pub ChargeDid get(charge_did): map Vec<u8> => bool;

        /// How much does creating a DID cost
        pub DidCreationFee get(did_creation_fee) config(): T::Balance;
    }
}

decl_module! {
    /// The module declaration.
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        // Initializing events
        // this is needed only if you are using events in your module
        fn deposit_event() = default;

        fn create_issuer(origin, issuer_did: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(Self::owner() == sender,"Sender must be the identity module owner");

            Self::do_create_issuer(issuer_did)
        }

        fn create_simple_token_issuer(origin, issuer_did: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(Self::owner() == sender,"Sender must be the identity module owner");

            Self::do_create_simple_token_issuer(issuer_did)
        }

        fn create_investor(origin, investor_did: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;
            ensure!(Self::owner() == sender,"Sender must be the identity module owner");

            Self::do_create_investor(investor_did)
        }

        fn set_charge_did(origin, charge_did: bool) -> Result {
            let sender = ensure_signed(origin)?;
            <ChargeDid>::insert(sender.encode(), charge_did);
            Ok(())
        }

        /// Register signing keys for a new DID. Uses origin key as the master key
        pub fn register_did(origin, did: Vec<u8>, signing_keys: Vec<Vec<u8>>) -> Result {

            let sender = ensure_signed(origin)?;

            let master_key = sender.encode();

            // Make sure caller specified a correct DID
            validate_did(did.as_slice())?;

            // Make sure there's no pre-existing entry for the DID
            ensure!(!<DidRecords<T>>::exists(&did), "DID must be unique");

            // TODO: Subtract the fee
            let _imbalance = <balances::Module<T> as Currency<_>>::withdraw(
                &sender,
                Self::did_creation_fee(),
                WithdrawReason::Fee,
                ExistenceRequirement::KeepAlive
                )?;

            for key in &signing_keys {
                if <SigningKeyDid>::exists(key) {
                    ensure!(<SigningKeyDid>::get(key) == did, "One signing key can only belong to one DID");
                }
            }

            for key in &signing_keys {
                <SigningKeyDid>::insert(key, did.clone());
            }

            let record = DidRecord {
                signing_keys: signing_keys.clone(),
                master_key,
                ..Default::default()
            };

            <DidRecords<T>>::insert(&did, record);

            Self::deposit_event(RawEvent::NewDid(did, sender, signing_keys));

            Ok(())
        }

        /// Adds new signing keys for a DID. Only called by master key owner.
        fn add_signing_keys(origin, did: Vec<u8>, additional_keys: Vec<Vec<u8>>) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<DidRecords<T>>::exists(&did), "DID must already exist");

            // Verify that sender key is current master key
            let sender_key = sender.encode();
            let record = <DidRecords<T>>::get(&did);
            ensure!(sender_key == record.master_key, "Sender must hold the master key");

            for key in &additional_keys {
                if <SigningKeyDid>::exists(key) {
                    ensure!(<SigningKeyDid>::get(key) == did, "One signing key can only belong to one DID");
                }
            }

            for key in &additional_keys {
                <SigningKeyDid>::insert(key, did.clone());
            }

            <DidRecords<T>>::mutate(&did,
            |record| {
                // Concatenate new keys while making sure the key set is
                // unique
                let mut new_additional_keys = additional_keys.iter()
                    .filter( |&key| !record.signing_keys.contains(key))
                    .cloned()
                    .collect::<Vec<_>>();
                record.signing_keys.append( &mut new_additional_keys);
            });

            Self::deposit_event(RawEvent::SigningKeysAdded(did, additional_keys));

            Ok(())
        }

        /// Removes specified signing keys of a DID if present. Only called by master key owner.
        fn remove_signing_keys(origin, did: Vec<u8>, keys_to_remove: Vec<Vec<u8>>) -> Result {
            let sender = ensure_signed(origin)?;

            // Verify that sender key is current master key
            let sender_key = sender.encode();
            let record = <DidRecords<T>>::get(&did);
            ensure!(sender_key == record.master_key, "Sender must hold the master key");

            ensure!(<DidRecords<T>>::exists(&did), "DID must already exist");

            for key in &keys_to_remove {
                if <SigningKeyDid>::exists(key) {
                    ensure!(<SigningKeyDid>::get(key) == did, "Signing key does not belong to this DID");
                }
            }

            for key in &keys_to_remove {
                <SigningKeyDid>::remove(key);
            }

            <DidRecords<T>>::mutate(&did,
            |record| {
                // Filter out keys meant for deletion
                let signed_keys = record.signing_keys
                    .iter()
                    .filter(|&key| !keys_to_remove.contains(key))
                    .cloned()
                    .collect::<Vec<_>>();

                (*record).signing_keys = signed_keys;
            });

            Self::deposit_event(RawEvent::SigningKeysRemoved(did, keys_to_remove));

            Ok(())
        }

        /// Sets a new master key for a DID. Only called by master key owner.
        fn set_master_key(origin, did: Vec<u8>, new_key: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;

            // Verify that sender key is current master key
            let sender_key = sender.encode();
            let record = <DidRecords<T>>::get(&did);
            ensure!(sender_key == record.master_key, "Sender must hold the master key");

            ensure!(<DidRecords<T>>::exists(&did), "DID must already exist");

            <DidRecords<T>>::mutate(&did,
            |record| {
                (*record).master_key = new_key.clone();
            });

            Self::deposit_event(RawEvent::NewMasterKey(did, sender, new_key));

            Ok(())
        }

        /// Adds funds to a DID.
        pub fn fund_poly(origin, did: Vec<u8>, amount: <T as balances::Trait>::Balance) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<DidRecords<T>>::exists(&did), "DID must already exist");

            let record = <DidRecords<T>>::get(&did);

            // We must know that new balance is valid without creating side effects
            let new_record_balance = record.balance.checked_add(&amount).ok_or("overflow occured when increasing DID balance")?;

            let _imbalance = <balances::Module<T> as Currency<_>>::withdraw(
                &sender,
                amount,
                WithdrawReason::Fee,
                ExistenceRequirement::KeepAlive
                )?;

            <DidRecords<T>>::mutate(&did, |record| {
                (*record).balance = new_record_balance;
            });

            Self::deposit_event(RawEvent::PolyDepositedInDid(did, sender, amount));

            Ok(())
        }

        /// Withdraws funds from a DID. Only called by master key owner.
        fn withdrawy_poly(origin, did: Vec<u8>, amount: <T as balances::Trait>::Balance) -> Result {
            let sender = ensure_signed(origin)?;

            // Verify that sender key is current master key
            let sender_key = sender.encode();
            let record = <DidRecords<T>>::get(&did);
            ensure!(sender_key == record.master_key, "Sender must hold the master key");

            ensure!(<DidRecords<T>>::exists(&did), "DID must already exist");

            let record = <DidRecords<T>>::get(&did);

            // We must know that new balance is valid without creating side effects
            let new_record_balance = record.balance.checked_sub(&amount).ok_or("underflow occured when decreasing DID balance")?;

            let _imbalance = <balances::Module<T> as Currency<_>>::deposit_into_existing(&sender, amount)?;

            <DidRecords<T>>::mutate(&did, |record| {
                (*record).balance = new_record_balance;
            });

            Self::deposit_event(RawEvent::PolyWithdrawnFromDid(did, sender, amount));

            Ok(())
        }

        /// Transfers funds between DIDs.
        fn transfer_poly(origin, did: Vec<u8>, to_did: Vec<u8>, amount: <T as balances::Trait>::Balance) -> Result {
            let sender = ensure_signed(origin)?;

            // Check that sender is allowed to act on behalf of `did`
            ensure!(Self::is_signing_key(&did, &sender.encode()), "sender must be a signing key for DID");

            let from_record = <DidRecords<T>>::get(did.clone());
            let to_record = <DidRecords<T>>::get(to_did.clone());

            // Same for `from`
            let new_from_balance = from_record.balance.checked_sub(&amount).ok_or("Sender must have sufficient funds")?;

            // Compute new `to_did` balance and check that beneficiary's balance can be increased
            let new_to_balance = to_record.balance.checked_add(&amount).ok_or("Failed to increase to_did balance")?;

            // Alter from record
            <DidRecords<T>>::mutate(did, |record| {
                record.balance = new_from_balance;
            });

            // Alter to record
            <DidRecords<T>>::mutate(to_did, |record| {
                record.balance = new_to_balance;
            });

            Ok(())
        }

        /// Appends a claim issuer DID to a DID. Only called by master key owner.
        fn add_claim_issuer(origin, did: Vec<u8>, did_issuer: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;

            // Verify that sender key is current master key
            let sender_key = sender.encode();
            let record = <DidRecords<T>>::get(&did);
            ensure!(sender_key == record.master_key, "Sender must hold the master key");

            <ClaimIssuers>::mutate(did.clone(), |old_claim_issuers| {
                if !old_claim_issuers.contains(&did_issuer) {
                    old_claim_issuers.push(did_issuer.clone());
                }
            });

            Self::deposit_event(RawEvent::NewClaimIssuer(did, did_issuer));

            Ok(())
        }

        /// Removes a claim issuer DID. Only called by master key owner.
        fn remove_claim_issuer(origin, did: Vec<u8>, did_issuer: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;

            // Verify that sender key is current master key
            let sender_key = sender.encode();
            let record = <DidRecords<T>>::get(&did);
            ensure!(sender_key == record.master_key, "Sender must hold the master key");

            ensure!(<DidRecords<T>>::exists(&did), "DID must already exist");
            ensure!(<DidRecords<T>>::exists(&did_issuer), "claim issuer DID must already exist");

            <ClaimIssuers>::mutate(&did, |old_claim_issuers| {
                *old_claim_issuers = old_claim_issuers
                    .iter()
                    .filter(|&issuer| *issuer != did_issuer)
                    .cloned()
                    .collect();
            });

            Self::deposit_event(RawEvent::RemovedClaimIssuer(did, did_issuer));

            Ok(())
        }

        /// Adds new claim record or edits an exisitng one. Only called by did_issuer's signing key
        fn add_claim(
            origin,
            did: Vec<u8>,
            claim_key: Vec<u8>,
            did_issuer: Vec<u8>,
            expiry: <T as timestamp::Trait>::Moment,
            claim_value: ClaimValue
        ) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<DidRecords<T>>::exists(&did), "DID must already exist");
            ensure!(<DidRecords<T>>::exists(&did_issuer), "claim issuer DID must already exist");

            let sender_key = sender.encode();
            ensure!(Self::is_claim_issuer(&did, &did_issuer) || Self::is_master_key(&did, &sender_key), "did_issuer must be a claim issuer or master key for DID");

            // Verify that sender key is one of did_issuer's signing keys
            ensure!(Self::is_signing_key(&did_issuer, &sender_key), "Sender must hold a claim issuer's signing key");

            let claim_meta_data = ClaimMetaData {
                claim_key: claim_key,
                claim_issuer: did_issuer,
            };

            let now = <timestamp::Module<T>>::get();

            let claim = Claim {
                issuance_date: now,
                expiry: expiry,
                claim_value: claim_value,
            };

            <Claims<T>>::insert((did.clone(), claim_meta_data.clone()), claim.clone());

            <ClaimKeys>::mutate(&did, |old_claim_data| {
                if !old_claim_data.contains(&claim_meta_data) {
                    old_claim_data.push(claim_meta_data.clone());
                }
            });

            Self::deposit_event(RawEvent::NewClaims(did, claim_meta_data, claim));

            Ok(())
        }

        /// Only for testing. Remove once UI can encode/decode properly
        fn add_u128_claim(
            origin,
            did: Vec<u8>,
            claim_key: Vec<u8>,
            did_issuer: Vec<u8>,
            expiry: <T as timestamp::Trait>::Moment,
            claim_value: u128
        ) -> Result {
            let claim_value = ClaimValue {
                value: claim_value.encode(),
                data_type: DataTypes::U128,
            };
            Self::add_claim(origin, did, claim_key, did_issuer, expiry, claim_value)
        }

        /// Marks the specified claim as revoked
        fn revoke_claim(origin, did: Vec<u8>, claim_key: Vec<u8>, did_issuer: Vec<u8>) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(<DidRecords<T>>::exists(&did), "DID must already exist");
            ensure!(<DidRecords<T>>::exists(&did_issuer), "claim issuer DID must already exist");

            // Verify that sender key is one of did_issuer's signing keys
            let sender_key = sender.encode();
            ensure!(Self::is_signing_key(&did_issuer, &sender_key), "Sender must hold a claim issuer's signing key");

            let claim_meta_data = ClaimMetaData {
                claim_key: claim_key,
                claim_issuer: did_issuer,
            };

            <Claims<T>>::remove((did.clone(), claim_meta_data.clone()));

            <ClaimKeys>::mutate(&did, |old_claim_metadata| {
                *old_claim_metadata = old_claim_metadata
                    .iter()
                    .filter(|&metadata| *metadata != claim_meta_data)
                    .cloned()
                    .collect();
            });

            Self::deposit_event(RawEvent::RevokedClaim(did, claim_meta_data));

            Ok(())
        }
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as system::Trait>::AccountId,
        Balance = <T as balances::Trait>::Balance,
        Moment = <T as timestamp::Trait>::Moment,
    {
        /// DID, master key account ID, signing keys
        NewDid(Vec<u8>, AccountId, Vec<Vec<u8>>),

        /// DID, new keys
        SigningKeysAdded(Vec<u8>, Vec<Vec<u8>>),

        /// DID, the keys that got removed
        SigningKeysRemoved(Vec<u8>, Vec<Vec<u8>>),

        /// DID, old master key account ID, new key
        NewMasterKey(Vec<u8>, AccountId, Vec<u8>),

        /// beneficiary DID, sender, amount
        PolyDepositedInDid(Vec<u8>, AccountId, Balance),

        /// DID, beneficiary, amount
        PolyWithdrawnFromDid(Vec<u8>, AccountId, Balance),

        /// DID, amount
        PolyChargedFromDid(Vec<u8>, Balance),

        /// DID from, DID to, amount
        PolyTransfer(Vec<u8>, Vec<u8>, Balance),

        /// DID, claim issuer DID
        NewClaimIssuer(Vec<u8>, Vec<u8>),

        /// DID, removed claim issuer DID
        RemovedClaimIssuer(Vec<u8>, Vec<u8>),

        /// DID, claim issuer DID, claims
        NewClaims(Vec<u8>, ClaimMetaData, Claim<Moment>),

        /// DID, claim issuer DID, claim
        RevokedClaim(Vec<u8>, ClaimMetaData),

        /// DID
        NewIssuer(Vec<u8>),
    }
);

impl<T: Trait> Module<T> {
    /// Add a new issuer. Warning: No identity module ownership checks are performed
    pub fn do_create_issuer(issuer_did: Vec<u8>) -> Result {
        let new_issuer = Issuer {
            did: issuer_did.clone(),
            access_level: 1,
            active: true,
        };

        <IssuerList>::insert(issuer_did.clone(), new_issuer);

        Self::deposit_event(RawEvent::NewIssuer(issuer_did));

        Ok(())
    }

    /// Add a new SimpleToken issuer. Warning: No identity module ownership checks are performed
    pub fn do_create_simple_token_issuer(issuer_did: Vec<u8>) -> Result {
        let new_simple_token_issuer = Issuer {
            did: issuer_did.clone(),
            access_level: 1,
            active: true,
        };

        <SimpleTokenIssuerList>::insert(issuer_did, new_simple_token_issuer);
        Ok(())
    }

    /// Add a new investor. Warning: No identity module ownership checks are performed
    pub fn do_create_investor(investor_did: Vec<u8>) -> Result {
        let new_investor = Investor {
            did: investor_did.clone(),
            access_level: 1,
            active: true,
            jurisdiction: 1,
        };

        <InvestorList>::insert(investor_did, new_investor);
        Ok(())
    }

    pub fn is_issuer(did: &Vec<u8>) -> bool {
        let user = Self::issuer_list(did);
        user.did == *did && user.access_level == 1 && user.active
    }

    pub fn is_simple_token_issuer(did: Vec<u8>) -> bool {
        let user = Self::simple_token_issuer_list(did.clone());
        user.did == did && user.access_level == 1 && user.active
    }

    pub fn is_investor(did: Vec<u8>) -> bool {
        let user = Self::investor_list(did.clone());
        user.did == did && user.access_level == 1 && user.active
    }

    pub fn is_claim_issuer(did: &Vec<u8>, issuer_did: &Vec<u8>) -> bool {
        <ClaimIssuers>::get(did).contains(issuer_did)
    }

    pub fn is_signing_key(did: &Vec<u8>, key: &Vec<u8>) -> bool {
        let record = <DidRecords<T>>::get(did);
        record.signing_keys.contains(key) || Self::is_master_key(did, key)
    }

    /// Use `did` as reference.
    pub fn is_master_key(did: &Vec<u8>, key: &Vec<u8>) -> bool {
        &<DidRecords<T>>::get(did).master_key == key
    }

    /// Withdraws funds from a DID balance
    pub fn charge_poly(did: Vec<u8>, amount: T::Balance) -> bool {
        if !<DidRecords<T>>::exists(did.clone()) {
            return false;
        }

        let record = <DidRecords<T>>::get(did.clone());

        if record.balance < amount {
            return false;
        }

        <DidRecords<T>>::mutate(did.clone(), |record| {
            (*record).balance = record.balance - amount;
        });

        Self::deposit_event(RawEvent::PolyChargedFromDid(did, amount));

        return true;
    }

    pub fn fetch_claim_value(
        did: Vec<u8>,
        claim_key: Vec<u8>,
        claim_issuer: Vec<u8>,
    ) -> Option<ClaimValue> {
        let claim_meta_data = ClaimMetaData {
            claim_key: claim_key,
            claim_issuer: claim_issuer,
        };
        if <Claims<T>>::exists((did.clone(), claim_meta_data.clone())) {
            let now = <timestamp::Module<T>>::get();
            let claim = <Claims<T>>::get((did, claim_meta_data));
            if claim.expiry > now {
                return Some(claim.claim_value);
            }
        }
        return None;
    }

    pub fn fetch_claim_value_multiple_issuers(
        did: Vec<u8>,
        claim_key: Vec<u8>,
        claim_issuers: Vec<Vec<u8>>,
    ) -> Option<ClaimValue> {
        for claim_issuer in claim_issuers {
            let claim_value = Self::fetch_claim_value(did.clone(), claim_key.clone(), claim_issuer);
            if claim_value.is_some() {
                return claim_value;
            }
        }
        return None;
    }
}

/// Make sure the supplied slice is a valid Polymesh DID
pub fn validate_did(did: &[u8]) -> Result {
    // TODO: Also check length after prefix,
    if did.starts_with(DID_PREFIX.as_bytes()) {
        Ok(())
    } else {
        Err("DID has no valid prefix")
    }
}

pub trait IdentityTrait<T> {
    fn signing_key_charge_did(signing_key: &Vec<u8>) -> bool;
    fn charge_poly(did: &Vec<u8>, amount: T) -> bool;
}

impl<T: Trait> IdentityTrait<T::Balance> for Module<T> {
    fn charge_poly(signing_key: &Vec<u8>, amount: T::Balance) -> bool {
        Self::charge_poly(<SigningKeyDid>::get(signing_key), amount)
    }

    fn signing_key_charge_did(signing_key: &Vec<u8>) -> bool {
        if <SigningKeyDid>::exists(signing_key) {
            if Self::is_signing_key(&<SigningKeyDid>::get(signing_key), signing_key) {
                if <ChargeDid>::exists(signing_key) {
                    return <ChargeDid>::get(signing_key);
                }
            }
        }
        return false;
    }
}

/// tests for this module
#[cfg(test)]
mod tests {
    use super::*;

    use sr_io::{with_externalities, TestExternalities};
    use sr_primitives::{
        testing::Header,
        traits::{BlakeTwo256, ConvertInto, IdentityLookup},
        Perbill,
    };
    use srml_support::{assert_err, assert_ok, impl_outer_origin, parameter_types};
    use std::result::Result;
    use substrate_primitives::{Blake2Hasher, H256};

    impl_outer_origin! {
        pub enum Origin for IdentityTest {}
    }

    // For testing the module, we construct most of a mock runtime. This means
    // first constructing a configuration type (`Test`) which `impl`s each of the
    // configuration traits of modules we want to use.
    #[derive(Clone, Eq, PartialEq)]
    pub struct IdentityTest;

    parameter_types! {
        pub const BlockHashCount: u32 = 250;
        pub const MaximumBlockWeight: u32 = 4096;
        pub const MaximumBlockLength: u32 = 4096;
        pub const AvailableBlockRatio: Perbill = Perbill::from_percent(75);
    }

    impl system::Trait for IdentityTest {
        type Origin = Origin;
        type Index = u64;
        type BlockNumber = u64;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = u64;
        type Lookup = IdentityLookup<Self::AccountId>;
        type Header = Header;
        type Event = ();

        type Call = ();
        type WeightMultiplierUpdate = ();
        type BlockHashCount = BlockHashCount;
        type MaximumBlockWeight = MaximumBlockWeight;
        type MaximumBlockLength = MaximumBlockLength;
        type AvailableBlockRatio = AvailableBlockRatio;
        type Version = ();
    }

    parameter_types! {
        pub const ExistentialDeposit: u64 = 0;
        pub const TransferFee: u64 = 0;
        pub const CreationFee: u64 = 0;
        pub const TransactionBaseFee: u64 = 0;
        pub const TransactionByteFee: u64 = 0;
    }

    impl balances::Trait for IdentityTest {
        type Balance = u128;
        type OnFreeBalanceZero = ();
        type OnNewAccount = ();
        type Event = ();
        type TransactionPayment = ();
        type DustRemoval = ();
        type TransferPayment = ();

        type ExistentialDeposit = ExistentialDeposit;
        type TransferFee = TransferFee;
        type CreationFee = CreationFee;
        type TransactionBaseFee = TransactionBaseFee;
        type TransactionByteFee = TransactionByteFee;
        type WeightToFee = ConvertInto;
        type Identity = super::Module<IdentityTest>;
    }

    parameter_types! {
        pub const MinimumPeriod: u64 = 3;
    }

    impl timestamp::Trait for IdentityTest {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = MinimumPeriod;
    }

    impl super::Trait for IdentityTest {
        type Event = ();
    }

    type Identity = super::Module<IdentityTest>;

    /// Create externalities
    fn build_ext() -> TestExternalities<Blake2Hasher> {
        system::GenesisConfig::default()
            .build_storage::<IdentityTest>()
            .unwrap()
            .into()
    }

    /// It creates an Account and registers its DID.
    fn make_account(
        id: u64,
    ) -> Result<(<IdentityTest as system::Trait>::Origin, Vec<u8>), &'static str> {
        let signed_id = Origin::signed(id);
        let did = format!("did:poly:{}", id).as_bytes().to_vec();

        Identity::register_did(signed_id.clone(), did.clone(), vec![])?;
        Ok((signed_id, did))
    }

    #[test]
    fn dids_are_unique() {
        with_externalities(&mut build_ext(), || {
            let did_1 = "did:poly:1".as_bytes().to_vec();

            assert_ok!(Identity::register_did(
                Origin::signed(1),
                did_1.clone(),
                vec![]
            ));

            assert_ok!(Identity::register_did(
                Origin::signed(2),
                "did:poly:2".as_bytes().to_vec(),
                vec![]
            ));

            assert_err!(
                Identity::register_did(Origin::signed(3), did_1, vec![]),
                "DID must be unique"
            );
        });
    }

    #[test]
    fn only_claim_issuers_can_add_claims() {
        with_externalities(&mut build_ext(), || {
            let owner_id = Identity::owner();
            let (owner, owner_did) = make_account(owner_id).unwrap();
            let (_issuer, issuer_did) = make_account(2).unwrap();
            let (claim_issuer, claim_issuer_did) = make_account(3).unwrap();

            assert_ok!(Identity::add_signing_keys(
                claim_issuer.clone(),
                claim_issuer_did.clone(),
                vec![owner_id.encode()]
            ));

            // Create issuer and claim issuer
            assert_ok!(Identity::create_issuer(owner.clone(), issuer_did.clone()));
            assert_ok!(Identity::add_claim_issuer(
                owner.clone(),
                owner_did.clone(),
                claim_issuer_did.clone()
            ));

            // Add Claims by master & claim_issuer
            let claims = vec![Claim {
                topic: 1,
                schema: 1,
                bytes: vec![],
                expiry: 10,
            }];

            assert_ok!(Identity::add_claim(
                owner.clone(),
                owner_did.clone(),
                claim_issuer_did.clone(),
                claims.clone()
            ));
            assert_ok!(Identity::add_claim(
                claim_issuer.clone(),
                owner_did.clone(),
                claim_issuer_did.clone(),
                claims.clone()
            ));

            assert_err!(
                Identity::add_claim(
                    claim_issuer.clone(),
                    owner_did.clone(),
                    issuer_did.clone(),
                    claims.clone()
                ),
                "did_issuer must be a claim issuer or master key for DID"
            );
            assert_err!(
                Identity::add_claim(owner.clone(), owner_did.clone(), issuer_did, claims),
                "Sender must hold a claim issuer\'s signing key"
            );
        });
    }

    #[test]
    fn only_master_or_signing_keys_can_authenticate_as_an_identity() {
        with_externalities(&mut build_ext(), || {
            let owner_id = Identity::owner();
            let owner_key = owner_id.encode();
            let (_owner, owner_did) = make_account(owner_id).unwrap();
            let (a, a_did) = make_account(2).unwrap();
            let (_b, b_did) = make_account(3).unwrap();

            assert_ok!(Identity::add_signing_keys(
                a.clone(),
                a_did.clone(),
                vec![owner_key.clone()]
            ));

            // Check master key on master and signing_keys.
            assert!(Identity::is_signing_key(&owner_did, &owner_key));
            assert!(Identity::is_signing_key(&a_did, &owner_key));

            assert!(Identity::is_signing_key(&b_did, &owner_key) == false);

            // ... and remove that key.
            assert_ok!(Identity::remove_signing_keys(
                a.clone(),
                a_did.clone(),
                vec![owner_key.clone()]
            ));
            assert!(Identity::is_signing_key(&a_did, &owner_key) == false);
        });
    }

    #[test]
    fn revoking_claims() {
        with_externalities(&mut build_ext(), || {
            let owner_id = Identity::owner();
            let (owner, owner_did) = make_account(Identity::owner()).unwrap();
            let (issuer, issuer_did) = make_account(2).unwrap();

            let (claim_issuer, claim_issuer_did) = make_account(3).unwrap();
            assert_ok!(Identity::add_claim_issuer(
                owner.clone(),
                owner_did.clone(),
                claim_issuer_did.clone()
            ));
            assert_ok!(Identity::add_signing_keys(
                claim_issuer.clone(),
                claim_issuer_did.clone(),
                vec![owner_id.encode()]
            ));

            // Add Claims by master & claim_issuer
            let claim = Claim {
                topic: 1,
                schema: 1,
                bytes: vec![],
                expiry: 10,
            };

            assert_ok!(Identity::add_claim(
                owner.clone(),
                owner_did.clone(),
                claim_issuer_did.clone(),
                vec![claim.clone()]
            ));

            assert_err!(
                Identity::revoke_claim(
                    issuer.clone(),
                    issuer_did.clone(),
                    claim_issuer_did.clone(),
                    claim.clone()
                ),
                "did_issuer must be a claim issuer for DID"
            );
            assert_err!(
                Identity::revoke_claim(
                    claim_issuer.clone(),
                    claim_issuer_did.clone(),
                    claim_issuer_did.clone(),
                    claim.clone()
                ),
                "did_issuer must be a claim issuer for DID"
            );

            assert_ok!(Identity::revoke_claim(
                owner.clone(),
                owner_did.clone(),
                claim_issuer_did.clone(),
                claim.clone()
            ));
            // TODO Revoke claim twice??
            assert_ok!(Identity::revoke_claim(
                owner,
                owner_did,
                claim_issuer_did,
                claim
            ));
        });
    }
}
