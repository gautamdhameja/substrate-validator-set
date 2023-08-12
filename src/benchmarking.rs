#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::v1::{account, benchmarks, BenchmarkError};
use frame_support::traits::EnsureOrigin;

const SEED: u32 = 0;

benchmarks! {
	add_validator {
		let origin =
			T::AddRemoveOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let validator: T::ValidatorId = account("validator", 0, SEED);
	}: _<T::RuntimeOrigin>(origin, validator)

	remove_validator {
		let origin =
			T::AddRemoveOrigin::try_successful_origin().map_err(|_| BenchmarkError::Weightless)?;
		let validator: T::ValidatorId = account("validator", 0, SEED);
	}: _<T::RuntimeOrigin>(origin, validator)

	impl_benchmark_test_suite!(
		ValidatorSet,
		crate::mock::new_test_ext(),
		crate::mock::Test,
	);
}
