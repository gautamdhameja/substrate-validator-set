// Tests for the Validator Set pallet

use super::*;
use codec::Decode;
use frame_support::{assert_noop, assert_ok, pallet_prelude::*, traits::OnInitialize};
use mock::{
    authorities, before_session_end_called, new_test_ext,
    reset_before_session_end_called, session_changed, set_next_validators,
    Origin, PreUpgradeMockSessionKeys, Session, System, ValidatorSet, SESSION_CHANGED,
};
use sp_core::crypto::key_types::DUMMY;
use sp_runtime::testing::UintAuthorityId;

fn initialize_block(block: u64) {
    SESSION_CHANGED.with(|l| *l.borrow_mut() = false);
    System::set_block_number(block);
    Session::on_initialize(block);
}

#[test]
fn simple_setup_should_work() {
    new_test_ext().execute_with(|| {
        assert_eq!(
            authorities(),
            vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]
        );
        assert_eq!(Session::validators(), vec![1, 2, 3]);
        assert_eq!(ValidatorSet::validators(), Some(vec![1, 2, 3]));
    });
}

#[test]
fn add_validator_updates_validators_list() {
    new_test_ext().execute_with(|| {
        assert_ok!(ValidatorSet::add_validator(Origin::root(), 4));
        assert_eq!(ValidatorSet::validators(), Some(vec![1, 2, 3, 4]));
    });
}

#[test]
fn add_validator_triggers_session_change() {
    new_test_ext().execute_with(|| {
        
		// Block 1: No change
		initialize_block(1);
		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]);
		assert_eq!(Session::current_index(), 0);

		// Block 2: No change
		initialize_block(2);
		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]);
		assert_eq!(Session::current_index(), 0);

		// Block 3: Set key for validator 4; no visible change.
		initialize_block(3);
		assert_ok!(Session::set_keys(Origin::signed(4), UintAuthorityId(4).into(), vec![]));
		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]);
		assert_eq!(Session::current_index(), 0);

		// Block 4: Add validator 4; no visible change, but should trigger session change.
		initialize_block(4);
		assert_ok!(ValidatorSet::add_validator(Origin::root(), 4));
		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]);
		assert_eq!(Session::current_index(), 1);

		// Block 5: Session rollover; New authority was added.
		initialize_block(5);
		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3), UintAuthorityId(4)]);
		assert_eq!(Session::current_index(), 2);

		// Block 6: No change.
		initialize_block(6);
		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3), UintAuthorityId(4)]);
		assert_eq!(Session::current_index(), 2);
    });
}

#[test]
fn remove_validator_updates_validators_list() {
    new_test_ext().execute_with(|| {
        assert_ok!(ValidatorSet::remove_validator(Origin::root(), 2));
        assert_eq!(ValidatorSet::validators(), Some(vec![1, 3]));
    });
}

#[test]
fn remove_validator_triggers_session_change() {
    new_test_ext().execute_with(|| {
        
		// Block 1: No change
		initialize_block(1);
		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]);
		assert_eq!(Session::current_index(), 0);

		// Block 2: No change
		initialize_block(2);
		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]);
		assert_eq!(Session::current_index(), 0);

		// Block 3: Remove validator 2; no visible change, but should trigger session change.
		initialize_block(3);
		assert_ok!(ValidatorSet::remove_validator(Origin::root(), 2));
		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]);
		assert_eq!(Session::current_index(), 1);

		// Block 4: Session rollover; authority was removed.
		initialize_block(4);		
		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(3)]);
		assert_eq!(Session::current_index(), 2);

		// Block 5: No change.
		initialize_block(5);
		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(3)]);
		assert_eq!(Session::current_index(), 2);
    });
}

#[test]
fn add_validator_fails_with_invalid_origin() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ValidatorSet::add_validator(Origin::signed(1), 4),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn remove_validator_fails_with_invalid_origin() {
    new_test_ext().execute_with(|| {
        assert_noop!(
            ValidatorSet::remove_validator(Origin::signed(1), 4),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn force_rotate_session_works() {
	new_test_ext().execute_with(|| {
		initialize_block(1);
		assert_eq!(Session::current_index(), 0);

		initialize_block(2);
		assert_eq!(Session::current_index(), 0);

		assert_ok!(ValidatorSet::force_rotate_session(Origin::root()));
		initialize_block(3);
		// Session is changed twice ... is this needed ?
		assert_eq!(Session::current_index(), 2);

		initialize_block(9);
		assert_eq!(Session::current_index(), 2);

		assert_ok!(ValidatorSet::force_rotate_session(Origin::root()));
		initialize_block(10);
		assert_eq!(Session::current_index(), 4);
	});
}

// #[test]
// fn authorities_should_track_validators() {
// 	reset_before_session_end_called();

// 	new_test_ext().execute_with(|| {
// 		set_next_validators(vec![1, 2]);
// 		force_new_session();
// 		initialize_block(1);
// 		assert_eq!(Session::queued_keys(), vec![
// 			(1, UintAuthorityId(1).into()),
// 			(2, UintAuthorityId(2).into()),
// 		]);
// 		assert_eq!(Session::validators(), vec![1, 2, 3]);
// 		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(3)]);
// 		assert!(before_session_end_called());
// 		reset_before_session_end_called();

// 		force_new_session();
// 		initialize_block(2);
// 		assert_eq!(Session::queued_keys(), vec![
// 			(1, UintAuthorityId(1).into()),
// 			(2, UintAuthorityId(2).into()),
// 		]);
// 		assert_eq!(Session::validators(), vec![1, 2]);
// 		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2)]);
// 		assert!(before_session_end_called());
// 		reset_before_session_end_called();

// 		set_next_validators(vec![1, 2, 4]);
// 		assert_ok!(Session::set_keys(Origin::signed(4), UintAuthorityId(4).into(), vec![]));
// 		force_new_session();
// 		initialize_block(3);
// 		assert_eq!(Session::queued_keys(), vec![
// 			(1, UintAuthorityId(1).into()),
// 			(2, UintAuthorityId(2).into()),
// 			(4, UintAuthorityId(4).into()),
// 		]);
// 		assert_eq!(Session::validators(), vec![1, 2]);
// 		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2)]);
// 		assert!(before_session_end_called());

// 		force_new_session();
// 		initialize_block(4);
// 		assert_eq!(Session::queued_keys(), vec![
// 			(1, UintAuthorityId(1).into()),
// 			(2, UintAuthorityId(2).into()),
// 			(4, UintAuthorityId(4).into()),
// 		]);
// 		assert_eq!(Session::validators(), vec![1, 2, 4]);
// 		assert_eq!(authorities(), vec![UintAuthorityId(1), UintAuthorityId(2), UintAuthorityId(4)]);
// 	});
// }
