//! # Validator Set Pallet
//!
//! The Validator Set Pallet allows addition and removal of
//! authorities/validators via extrinsics (transaction calls), in
//! Substrate-based PoA networks. It also integrates with the im-online pallet
//! to automatically remove offline validators.
//!
//! The pallet depends on the Session pallet and implements related traits for session
//! management. Currently it uses periodic session rotation provided by the
//! session pallet to automatically rotate sessions. For this reason, the
//! validator addition and removal becomes effective only after 2 sessions
//! (queuing + applying).

#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

use frame_support::{
	ensure,
	pallet_prelude::*,
	traits::{EstimateNextSessionRotation, Get, ValidatorSet, ValidatorSetWithIdentification},
};
use log;
pub use pallet::*;
use sp_runtime::traits::{Convert, Zero};
use sp_staking::offence::{Offence, OffenceError, ReportOffence};
use sp_std::prelude::*;

pub const LOG_TARGET: &'static str = "runtime::validator-set";

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	/// Configure the pallet by specifying the parameters and types on which it
	/// depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_session::Config {
		/// The Event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Origin for adding or removing a validator.
		type AddRemoveOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Minimum number of validators to leave in the validator set during
		/// auto removal.
		type MinAuthorities: Get<u32>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn validators)]
	pub type Validators<T: Config> = StorageValue<_, Vec<T::ValidatorId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn approved_validators)]
	pub type ApprovedValidators<T: Config> = StorageValue<_, Vec<T::ValidatorId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn offline_validators)]
	pub type OfflineValidators<T: Config> = StorageValue<_, Vec<T::ValidatorId>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New validator addition initiated. Effective in ~2 sessions.
		ValidatorAdditionInitiated(T::ValidatorId),

		/// Validator removal initiated. Effective in ~2 sessions.
		ValidatorRemovalInitiated(T::ValidatorId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Target (post-removal) validator count is below the minimum.
		TooLowValidatorCount,
		/// Validator is already in the validator set.
		Duplicate,
		/// Validator is not approved for re-addition.
		ValidatorNotApproved,
		/// Only the validator can add itself back after coming online.
		BadOrigin,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_validators: Vec<T::ValidatorId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { initial_validators: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_validators(&self.initial_validators);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Add a new validator.
		///
		/// New validator's session keys should be set in Session pallet before
		/// calling this.
		///
		/// The origin can be configured using the `AddRemoveOrigin` type in the
		/// host runtime. Can also be set to sudo/root.
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn add_validator(origin: OriginFor<T>, validator_id: T::ValidatorId) -> DispatchResult {
			T::AddRemoveOrigin::ensure_origin(origin)?;

			Self::do_add_validator(validator_id.clone())?;
			Self::approve_validator(validator_id)?;

			Ok(())
		}

		/// Remove a validator.
		///
		/// The origin can be configured using the `AddRemoveOrigin` type in the
		/// host runtime. Can also be set to sudo/root.
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn remove_validator(
			origin: OriginFor<T>,
			validator_id: T::ValidatorId,
		) -> DispatchResult {
			T::AddRemoveOrigin::ensure_origin(origin)?;

			Self::do_remove_validator(validator_id.clone())?;
			Self::unapprove_validator(validator_id)?;

			Ok(())
		}

		/// Add an approved validator again when it comes back online.
		///
		/// The origin can be configured using the `AddRemoveOrigin` type in the
		/// host runtime. Can also be set to sudo/root.
		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn add_validator_again(
			origin: OriginFor<T>,
			validator_id: T::ValidatorId,
		) -> DispatchResult {
			T::AddRemoveOrigin::ensure_origin(origin)?;

			ensure!(
				<ApprovedValidators<T>>::get().contains(&validator_id),
				Error::<T>::ValidatorNotApproved
			);

			Self::do_add_validator(validator_id)?;

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn initialize_validators(validators: &[T::ValidatorId]) {
		assert!(
			validators.len() as u32 >= T::MinAuthorities::get(),
			"Initial set of validators must be at least T::MinAuthorities"
		);
		assert!(<Validators<T>>::get().is_empty(), "Validators are already initialized!");

		<Validators<T>>::put(validators);
		<ApprovedValidators<T>>::put(validators);
	}

	fn do_add_validator(validator_id: T::ValidatorId) -> DispatchResult {
		ensure!(!<Validators<T>>::get().contains(&validator_id), Error::<T>::Duplicate);
		<Validators<T>>::mutate(|v| v.push(validator_id.clone()));

		Self::deposit_event(Event::ValidatorAdditionInitiated(validator_id.clone()));
		log::debug!(target: LOG_TARGET, "Validator addition initiated.");

		Ok(())
	}

	fn do_remove_validator(validator_id: T::ValidatorId) -> DispatchResult {
		let mut validators = <Validators<T>>::get();

		// Ensuring that the post removal, target validator count doesn't go
		// below the minimum.
		ensure!(
			validators.len().saturating_sub(1) as u32 >= T::MinAuthorities::get(),
			Error::<T>::TooLowValidatorCount
		);

		validators.retain(|v| *v != validator_id);

		<Validators<T>>::put(validators);

		Self::deposit_event(Event::ValidatorRemovalInitiated(validator_id.clone()));
		log::debug!(target: LOG_TARGET, "Validator removal initiated.");

		Ok(())
	}

	fn approve_validator(validator_id: T::ValidatorId) -> DispatchResult {
		ensure!(!<ApprovedValidators<T>>::get().contains(&validator_id), Error::<T>::Duplicate);
		<ApprovedValidators<T>>::mutate(|v| v.push(validator_id.clone()));
		Ok(())
	}

	fn unapprove_validator(validator_id: T::ValidatorId) -> DispatchResult {
		let mut approved_set = <ApprovedValidators<T>>::get();
		approved_set.retain(|v| *v != validator_id);
		<ApprovedValidators<T>>::set(approved_set);
		Ok(())
	}

	// Adds offline validators to a local cache for removal at new session.
	fn mark_for_removal(validator_id: T::ValidatorId) {
		<OfflineValidators<T>>::mutate(|v| v.push(validator_id));
		log::debug!(target: LOG_TARGET, "Offline validator marked for auto removal.");
	}

	// Removes offline validators from the validator set and clears the offline
	// cache. It is called in the session change hook and removes the validators
	// who were reported offline during the session that is ending. We do not
	// check for `MinAuthorities` here, because the offline validators will not
	// produce blocks and will have the same overall effect on the runtime.
	fn remove_offline_validators() {
		let validators_to_remove = <OfflineValidators<T>>::get();

		// Delete from active validator set.
		<Validators<T>>::mutate(|vs| vs.retain(|v| !validators_to_remove.contains(v)));
		log::debug!(
			target: LOG_TARGET,
			"Initiated removal of {:?} offline validators.",
			validators_to_remove.len()
		);

		// Clear the offline validator list to avoid repeated deletion.
		<OfflineValidators<T>>::put(Vec::<T::ValidatorId>::new());
	}
}

// Provides the new set of validators to the session module when session is
// being rotated.
impl<T: Config> pallet_session::SessionManager<T::ValidatorId> for Pallet<T> {
	// Plan a new session and provide new validator set.
	fn new_session(_new_index: u32) -> Option<Vec<T::ValidatorId>> {
		// Remove any offline validators. This will only work when the runtime
		// also has the im-online pallet.
		Self::remove_offline_validators();

		log::debug!(target: LOG_TARGET, "New session called; updated validator set provided.");

		Some(Self::validators())
	}

	fn end_session(_end_index: u32) {}

	fn start_session(_start_index: u32) {}
}

impl<T: Config> EstimateNextSessionRotation<T::BlockNumber> for Pallet<T> {
	fn average_session_length() -> T::BlockNumber {
		Zero::zero()
	}

	fn estimate_current_session_progress(
		_now: T::BlockNumber,
	) -> (Option<sp_runtime::Permill>, frame_support::dispatch::Weight) {
		(None, Zero::zero())
	}

	fn estimate_next_session_rotation(
		_now: T::BlockNumber,
	) -> (Option<T::BlockNumber>, frame_support::dispatch::Weight) {
		(None, Zero::zero())
	}
}

// Implementation of Convert trait.
// This is to satisfy trait bounds in session pallet.
// Here it just returns the same ValidatorId.
pub struct ValidatorOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> Convert<T::ValidatorId, Option<T::ValidatorId>> for ValidatorOf<T> {
	fn convert(account: T::ValidatorId) -> Option<T::ValidatorId> {
		Some(account)
	}
}

impl<T: Config> ValidatorSet<T::ValidatorId> for Pallet<T> {
	type ValidatorId = T::ValidatorId;
	type ValidatorIdOf = ValidatorOf<T>;

	fn session_index() -> sp_staking::SessionIndex {
		pallet_session::Pallet::<T>::current_index()
	}

	fn validators() -> Vec<T::ValidatorId> {
		pallet_session::Pallet::<T>::validators()
	}
}

impl<T: Config> ValidatorSetWithIdentification<T::ValidatorId> for Pallet<T> {
	type Identification = T::ValidatorId;
	type IdentificationOf = ValidatorOf<T>;
}

// Offence reporting and unresponsiveness management.
// This is for the ImOnline pallet integration.
impl<T: Config, O: Offence<(T::ValidatorId, T::ValidatorId)>>
	ReportOffence<T::AccountId, (T::ValidatorId, T::ValidatorId), O> for Pallet<T>
{
	fn report_offence(_reporters: Vec<T::AccountId>, offence: O) -> Result<(), OffenceError> {
		let offenders = offence.offenders();

		for (v, _) in offenders.into_iter() {
			Self::mark_for_removal(v);
		}

		Ok(())
	}

	fn is_known_offence(
		_offenders: &[(T::ValidatorId, T::ValidatorId)],
		_time_slot: &O::TimeSlot,
	) -> bool {
		false
	}
}
