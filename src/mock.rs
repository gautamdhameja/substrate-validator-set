//! Mock helpers for Validator Set pallet.

use super::*;
use crate as validatorset;
use frame_support::{parameter_types, traits::GenesisBuild, BasicExternalities};
use frame_system::EnsureRoot;
use pallet_session::*;
use sp_core::{crypto::key_types::DUMMY, H256};
use sp_runtime::{
    impl_opaque_keys,
    testing::{Header, UintAuthorityId},
    traits::{BlakeTwo256, IdentityLookup, OpaqueKeys},
    KeyTypeId, Perbill, RuntimeAppPublic,
};
use sp_staking::SessionIndex;
use std::cell::RefCell;

impl_opaque_keys! {
    pub struct MockSessionKeys {
        pub dummy: UintAuthorityId,
    }
}

impl From<UintAuthorityId> for MockSessionKeys {
    fn from(dummy: UintAuthorityId) -> Self {
        Self { dummy }
    }
}

pub const KEY_ID_A: KeyTypeId = KeyTypeId([4; 4]);
pub const KEY_ID_B: KeyTypeId = KeyTypeId([9; 4]);

#[derive(Debug, Clone, codec::Encode, codec::Decode, PartialEq, Eq)]
pub struct PreUpgradeMockSessionKeys {
    pub a: [u8; 32],
    pub b: [u8; 64],
}

impl OpaqueKeys for PreUpgradeMockSessionKeys {
    type KeyTypeIdProviders = ();

    fn key_ids() -> &'static [KeyTypeId] {
        &[KEY_ID_A, KEY_ID_B]
    }

    fn get_raw(&self, i: KeyTypeId) -> &[u8] {
        match i {
            i if i == KEY_ID_A => &self.a[..],
            i if i == KEY_ID_B => &self.b[..],
            _ => &[],
        }
    }
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
    pub enum Test where
        Block = Block,
        NodeBlock = Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
        Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
        ValidatorSet: validatorset::{Pallet, Call, Storage, Event<T>, Config<T>},
    }
);

thread_local! {
    pub static VALIDATORS: RefCell<Vec<u64>> = RefCell::new(vec![1, 2, 3]);
    pub static NEXT_VALIDATORS: RefCell<Vec<u64>> = RefCell::new(vec![1, 2, 3]);
    pub static AUTHORITIES: RefCell<Vec<UintAuthorityId>> =
        RefCell::new(vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]);
    pub static SESSION_CHANGED: RefCell<bool> = RefCell::new(false);
    pub static DISABLED: RefCell<bool> = RefCell::new(false);
    // Stores if `on_before_session_end` was called
    pub static BEFORE_SESSION_END_CALLED: RefCell<bool> = RefCell::new(false);
}

pub struct TestSessionHandler;
impl SessionHandler<u64> for TestSessionHandler {
    const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[UintAuthorityId::ID];
    fn on_genesis_session<T: OpaqueKeys>(_validators: &[(u64, T)]) {}
    fn on_new_session<T: OpaqueKeys>(
        changed: bool,
        validators: &[(u64, T)],
        _queued_validators: &[(u64, T)],
    ) {
        SESSION_CHANGED.with(|l| *l.borrow_mut() = changed);
        AUTHORITIES.with(|l| {
            *l.borrow_mut() = validators
                .iter()
                .map(|(_, id)| id.get::<UintAuthorityId>(DUMMY).unwrap_or_default())
                .collect()
        });
    }
    fn on_disabled(_validator_index: usize) {
        DISABLED.with(|l| *l.borrow_mut() = true)
    }
    fn on_before_session_ending() {
        BEFORE_SESSION_END_CALLED.with(|b| *b.borrow_mut() = true);
    }
}

pub fn authorities() -> Vec<UintAuthorityId> {
    AUTHORITIES.with(|l| l.borrow().to_vec())
}

pub fn session_changed() -> bool {
    SESSION_CHANGED.with(|l| *l.borrow())
}

pub fn set_next_validators(next: Vec<u64>) {
    NEXT_VALIDATORS.with(|v| *v.borrow_mut() = next);
}

pub fn before_session_end_called() -> bool {
    BEFORE_SESSION_END_CALLED.with(|b| *b.borrow())
}

pub fn reset_before_session_end_called() {
    BEFORE_SESSION_END_CALLED.with(|b| *b.borrow_mut() = false);
}

pub fn new_test_ext() -> sp_io::TestExternalities {
    let mut t = frame_system::GenesisConfig::default()
        .build_storage::<Test>()
        .unwrap();
    let keys: Vec<_> = NEXT_VALIDATORS.with(|l| {
        l.borrow()
            .iter()
            .cloned()
            .map(|i| (i, i, UintAuthorityId(i).into()))
            .collect()
    });
    BasicExternalities::execute_with_storage(&mut t, || {
        for (ref k, ..) in &keys {
            frame_system::Pallet::<Test>::inc_providers(k);
        }
        frame_system::Pallet::<Test>::inc_providers(&4);
        // An additional identity that we use.
        frame_system::Pallet::<Test>::inc_providers(&69);
    });
    pallet_session::GenesisConfig::<Test> { keys: keys.clone() }
        .assimilate_storage(&mut t)
        .unwrap();
    validatorset::GenesisConfig::<Test> {
        validators: keys.iter().map(|x| x.0).collect::<Vec<_>>(),
    }
    .assimilate_storage(&mut t)
    .unwrap();
    sp_io::TestExternalities::new(t)
}

parameter_types! {
    pub const MinimumPeriod: u64 = 5;
    pub const BlockHashCount: u64 = 250;
    pub BlockWeights: frame_system::limits::BlockWeights =
        frame_system::limits::BlockWeights::simple_max(1024);
}

impl frame_system::Config for Test {
    type BaseCallFilter = ();
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type Origin = Origin;
    type Index = u64;
    type BlockNumber = u64;
    type Call = Call;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = u64;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Header = Header;
    type Event = Event;
    type BlockHashCount = BlockHashCount;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = ();
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ();
    type OnSetCode = ();
}

impl pallet_timestamp::Config for Test {
    type Moment = u64;
    type OnTimestampSet = ();
    type MinimumPeriod = MinimumPeriod;
    type WeightInfo = ();
}

parameter_types! {
    pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(33);
}

impl validatorset::Config for Test {
    type AddRemoveOrigin = EnsureRoot<Self::AccountId>;
    type Event = Event;
}

impl pallet_session::Config for Test {
    type ShouldEndSession = ValidatorSet;
    type SessionManager = ValidatorSet;
    type SessionHandler = TestSessionHandler;
    type ValidatorId = <Self as frame_system::Config>::AccountId;
    type ValidatorIdOf = validatorset::ValidatorOf<Self>;
    type Keys = MockSessionKeys;
    type Event = Event;
    type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
    type NextSessionRotation = ValidatorSet;
    type WeightInfo = ();
}
