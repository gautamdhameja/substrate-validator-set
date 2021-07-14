//! # Validator Set Pallet
//!
//! The Validator Set Pallet allows addition and removal of authorities/validators via extrinsics (transaction calls), in Substrate-based
//! PoA networks.
//!
//! The pallet uses the Session pallet and implements related traits for session
//! management.

#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use sp_runtime::traits::{Convert, Zero};
use pallet_session::{Pallet as Session};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
    use frame_system::pallet_prelude::*;
    use super::*;

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_session::Config {
        /// The Event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Origin for adding or removing a validator.
	    type AddRemoveOrigin: EnsureOrigin<Self::Origin>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // The pallet's storage items.
    #[pallet::storage]
    #[pallet::getter(fn validators)]
    pub type Validators<T: Config> =  StorageValue<_, Vec<T::AccountId>>;

    #[pallet::storage]
    #[pallet::getter(fn flag)]
    pub type Flag<T: Config> =  StorageValue<_, bool>;

    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        // New validator added.
        ValidatorAdded(T::AccountId),

        // Validator removed.
        ValidatorRemoved(T::AccountId),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        NoValidators,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub validators: Vec<T::AccountId>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self { validators: Vec::new() }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            Pallet::<T>::initialize_validators(&self.validators);
        }
    }

    #[pallet::call]
    impl<T:Config> Pallet<T> {
        /// Add a new validator using elevated privileges.
        ///
        /// New validator's session keys should be set in session module before calling this.
        ///
        /// The origin can be configured using the `AddRemoveOrigin` type in the host runtime. 
        /// Can also be set to sudo/root.
        #[pallet::weight(0)]
        pub fn add_validator(origin: OriginFor<T>, validator_id: T::AccountId) -> DispatchResult {
            T::AddRemoveOrigin::ensure_origin(origin)?;

            let mut validators: Vec<T::AccountId>;

            if <Validators<T>>::get().is_none() {
                validators = vec![validator_id.clone()];
            } else {
                validators = <Validators<T>>::get().unwrap();
                validators.push(validator_id.clone());
            }

            <Validators<T>>::put(validators);

            // Calling rotate_session to queue the new session keys.
            Session::<T>::rotate_session();

            // Triggering rotate session again for the queued keys to take effect.
            Flag::<T>::put(true);

            Self::deposit_event(Event::ValidatorAdded(validator_id));
            Ok(())
        }

        /// Remove a validator using elevated privileges.
        ///
        /// The origin can be configured using the `AddRemoveOrigin` type in the host runtime. 
        /// Can also be set to sudo/root.
        #[pallet::weight(0)]
        pub fn remove_validator(origin: OriginFor<T>, validator_id: T::AccountId) -> DispatchResult {
            T::AddRemoveOrigin::ensure_origin(origin)?;
            let mut validators = <Validators<T>>::get().ok_or(Error::<T>::NoValidators)?;

            // Assuming that this will be a PoA network for enterprise use-cases,
            // the validator count may not be too big; the for loop shouldn't be too heavy.
            // In case the validator count is large, we need to find another way. **TODO**
            for (i, v) in validators.clone().into_iter().enumerate() {
                if v == validator_id {
                    validators.swap_remove(i);
                }
            }
            <Validators<T>>::put(validators);
            
            // Calling rotate_session to queue the new session keys.
            <pallet_session::Module<T>>::rotate_session();

            // Triggering rotate session again for the queued keys to take effect.
            Flag::<T>::put(true);

            Self::deposit_event(Event::ValidatorRemoved(validator_id));
            Ok(())
        }

        /// Force rotate session using elevated privileges.
        #[pallet::weight(0)]
        pub fn force_rotate_session(origin: OriginFor<T>) -> DispatchResult {
            T::AddRemoveOrigin::ensure_origin(origin)?;
            
            <pallet_session::Module<T>>::rotate_session();
            
            // Triggering rotate session again for any queued keys to take effect.
            // Not sure if double rotate is needed in this scenario. **TODO**
            Flag::<T>::put(true);
            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn initialize_validators(validators: &[T::AccountId]) {
            if !validators.is_empty() {
                assert!(<Validators<T>>::get().is_none(), "Validators are already initialized!");
                <Validators<T>>::put(validators);
            }
    }
}

/// Indicates to the session module if the session should be rotated.
/// We set this flag to true when we add/remove a validator.
impl<T: Config> pallet_session::ShouldEndSession<T::BlockNumber> for Pallet<T> {
    fn should_end_session(_now: T::BlockNumber) -> bool {
        Self::flag().unwrap()
    }
}

/// Provides the new set of validators to the session module when session is being rotated.
impl<T: Config> pallet_session::SessionManager<T::AccountId> for Pallet<T> {
    fn new_session(_new_index: u32) -> Option<Vec<T::AccountId>> {
        // Flag is set to false so that the session doesn't keep rotating.
        Flag::<T>::put(false);

        Self::validators()
    }

    fn end_session(_end_index: u32) {}

    fn start_session(_start_index: u32) {}
}

impl<T: Config> frame_support::traits::EstimateNextSessionRotation<T::BlockNumber> for Pallet<T> {
    fn average_session_length() -> T::BlockNumber {
        Zero::zero()
    }

    fn estimate_current_session_progress(_now: T::BlockNumber) -> (Option<sp_runtime::Permill>, frame_support::dispatch::Weight) {
        (None, Zero::zero())
    }

    fn estimate_next_session_rotation(_now: T::BlockNumber) -> (Option<T::BlockNumber>, frame_support::dispatch::Weight) {
        (None, Zero::zero())
    }
}

/// Implementation of Convert trait for mapping ValidatorId with AccountId.
/// This is mainly used to map stash and controller keys.
/// In this module, for simplicity, we just return the same AccountId.
pub struct ValidatorOf<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> Convert<T::AccountId, Option<T::AccountId>> for ValidatorOf<T> {
    fn convert(account: T::AccountId) -> Option<T::AccountId> {
        Some(account)
    }
}

