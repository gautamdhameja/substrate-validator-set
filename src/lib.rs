#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use frame_support::{
    StorageValue,
	decl_event, decl_storage, decl_module, decl_error,
	dispatch
};
use system::{self as system, ensure_root};
use sp_runtime::traits::Convert;

pub trait Trait: system::Trait + session::Trait {
	type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_storage! {
	trait Store for Module<T: Trait> as ValidatorSet {
		pub Validators get(fn validators) config(): Option<Vec<T::AccountId>>;
		Flag get(fn flag): bool;
	}
}

decl_event!(
	pub enum Event<T>
	where
		AccountId = <T as system::Trait>::AccountId,
	{
		// New validator added.
		ValidatorAdded(AccountId),

		// Validator removed.
		ValidatorRemoved(AccountId),
	}
);

decl_error! {
	/// Errors for the module.
	pub enum Error for Module<T: Trait> {
		NoValidators,
	}
}

decl_module! {
	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		fn on_initialize() {
			if Self::flag() {
				Flag::put(false);
			}
		}

		/// Add a new validator using root/sudo privileges.
		///
		/// New validator's session keys should be set using session module before calling this.
		pub fn add_validator(origin, validator_id: T::AccountId) -> dispatch::DispatchResult {
			ensure_root(origin)?;
			let mut validators = Self::validators().ok_or(Error::<T>::NoValidators)?;
			validators.push(validator_id.clone());
			<Validators<T>>::put(validators);
			Self::deposit_event(RawEvent::ValidatorAdded(validator_id));

			Flag::put(true);
			Ok(())
		}

		/// Remove a validator using root/sudo privileges.
		pub fn remove_validator(origin, validator_id: T::AccountId) -> dispatch::DispatchResult {
			ensure_root(origin)?;
			let mut validators = Self::validators().ok_or(Error::<T>::NoValidators)?;
			// Assuming that this will be a PoA network for enterprise use-cases, 
			// the validator count may not be too big, hence the for loop.
			// In case the validator count is large, we need to find another way.
			for (i, v) in validators.clone().into_iter().enumerate() {
				if v == validator_id {
					validators.swap_remove(i);
				}
			}
			<Validators<T>>::put(validators);
			Self::deposit_event(RawEvent::ValidatorRemoved(validator_id));

			Flag::put(true);
			Ok(())
		}
	}
}

/// Indicates to the session module if the session should be rotated.
/// We set this flag to true when we add/remove a validator.
impl<T: Trait> session::ShouldEndSession<T::BlockNumber> for Module<T> {
	fn should_end_session(_now: T::BlockNumber) -> bool {
		Self::flag()
	}
}

/// Provides the new set of validators to the session module when session is being rotated.
impl<T: Trait> session::OnSessionEnding<T::AccountId> for Module<T> {
	fn on_session_ending(_ending: u32, _start_session: u32) -> Option<Vec<T::AccountId>> {
		Self::validators()
	}
}

/// Provides the initial set of validators.
impl<T: Trait> session::SelectInitialValidators<T::AccountId> for Module<T> {
	fn select_initial_validators() -> Option<Vec<T::AccountId>> {
		Self::validators()
	}
}

/// Implementation of Convert trait for mapping ValidatorId with AccountId.
/// This is mainly used to map stash and controller keys.
/// In this module, for simplicity, we just return the same AccountId.
pub struct ValidatorOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Trait> Convert<T::AccountId, Option<T::AccountId>> for ValidatorOf<T> {
	fn convert(account: T::AccountId) -> Option<T::AccountId> {
		Some(account)
	}
}
