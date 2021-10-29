# Substrate Validator Set Pallet

A [Substrate](https://github.com/paritytech/substrate/) pallet to add/remove authorities/validators using extrinsics, in Substrate-based PoA networks.

**Note: Current master is compatible with Substrate [monthly-2021-10](https://github.com/paritytech/substrate/releases/tag/monthly-2021-10) tag. For older versions, please see releases/tags.**

## Demo

To see this pallet in action in a Substrate runtime, watch this video - https://www.youtube.com/watch?v=lIYxE-tOAdw

## Setup with Substrate Node Template

* Add the module's dependency in the `Cargo.toml` of your runtime directory. Make sure to enter the correct path or git url of the pallet as per your setup.

* Make sure that you also have the Substrate [session pallet](https://github.com/paritytech/substrate/tree/master/frame/session) as part of your runtime. This is because the validator-set pallet is dependent on the session pallet.

```toml
[dependencies.validator-set]
default-features = false
package = 'substrate-validator-set'
git = 'https://github.com/gautamdhameja/substrate-validator-set.git'
version = '4.0.0-dev'

[dependencies.pallet-session]
default-features = false
git = 'https://github.com/paritytech/substrate.git'
tag = 'monthly-2021-09+1'
version = '4.0.0-dev'
```

```toml
std = [
	...
	'validator-set/std',
	'pallet-session/std',
]
```

* Import `OpaqueKeys` in your `runtime/src/lib.rs`.

```rust
use sp_runtime::traits::{
	AccountIdLookup, BlakeTwo256, Block as BlockT, Verify, IdentifyAccount, NumberFor, OpaqueKeys
};
```

* Also in `runtime/src/lib.rs` import the `EnsureRoot` trait. This would change if you want to configure a custom origin (see below).

```rust
	use frame_system::EnsureRoot;
```

* Declare the pallet in your `runtime/src/lib.rs`. The pallet supports configurable origin and you can either set it to use one of the governance pallets (Collective, Democracy, etc.), or just use root as shown below. But **do not use a normal origin here** because the addition and removal of validators should be done using elevated privileges.

```rust
impl validator_set::Config for Runtime {
	type Event = Event;
	type AddRemoveOrigin = EnsureRoot<AccountId>;
}
```

* Also, declare the session pallet in  your `runtime/src/lib.rs`. The type configuration of session pallet would depend on the ValidatorSet pallet as shown below.

```rust
parameter_types! {
	pub const Period: u32 = 2 * MINUTES;
	pub const Offset: u32 = 0;
}

impl pallet_session::Config for Runtime {
	type SessionHandler = <opaque::SessionKeys as OpaqueKeys>::KeyTypeIdProviders;
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = ValidatorSet;
	type Event = Event;
	type Keys = opaque::SessionKeys;
	type ValidatorId = <Self as frame_system::Config>::AccountId;
	type ValidatorIdOf = validator_set::ValidatorOf<Self>;
	type DisabledValidatorsThreshold = ();
	type WeightInfo = ();
}
```

* Add both `session` and `validator_set` pallets in `construct_runtime` macro. **Make sure to add them before `Aura` and `Grandpa` pallets and after `Balances`.**

```rust
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		...
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		ValidatorSet: validator_set::{Pallet, Call, Storage, Event<T>, Config<T>},
		Aura: pallet_aura::{Pallet, Config<T>},
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event},
		...
		...
	}
);
```

* Add genesis config in the `chain_spec.rs` file for `session` and `validatorset` pallets, and update it for `Aura` and `Grandpa` pallets. Because the validators are provided by the `session` pallet, we do not initialize them explicitly for `Aura` and `Grandpa` pallets. Order is important, notice that `pallet_session` is declared after `pallet_balances` since it depends on it (session accounts should have some balance).

```rust
fn testnet_genesis(initial_authorities: Vec<(AccountId, AuraId, GrandpaId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool) -> GenesisConfig {
	GenesisConfig {
		...,
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k|(k, 1 << 60)).collect(),
		},
		validator_set: ValidatorSetConfig {
			validators: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
		},
		session: SessionConfig {
			keys: initial_authorities.iter().map(|x| {
				(x.0.clone(), x.0.clone(), session_keys(x.1.clone(), x.2.clone()))
			}).collect::<Vec<_>>(),
		},
		aura: AuraConfig {
			authorities: vec![],
		},
		grandpa: GrandpaConfig {
			authorities: vec![],
		},
	}
}
```

* Make sure you have the same number and order of session keys for your runtime. First in `runtime/src/lib.rs`:

```rust
pub struct SessionKeys {
	pub aura: Aura,
	pub grandpa: Grandpa,
}
```

* And then in `node/src/chain_spec.rs`:

```rust
fn session_keys(
	aura: AuraId,
	grandpa: GrandpaId,
) -> SessionKeys {
	SessionKeys { aura, grandpa }
}

pub fn authority_keys_from_seed(s: &str) -> (
	AccountId,
	AuraId,
	GrandpaId
) {
	(
		get_account_id_from_seed::<sr25519::Public>(s),
		get_from_seed::<AuraId>(s),
		get_from_seed::<GrandpaId>(s)
	)
}
```

* Import `opaque::SessionKeys, ValidatorSetConfig, SessionConfig` from the runtime in `node/src/chain_spec.rs`.
  
```rust
use node_template_runtime::{
	AccountId, AuraConfig, BalancesConfig, GenesisConfig, GrandpaConfig,
	SudoConfig, SystemConfig, WASM_BINARY, Signature, 
	opaque::SessionKeys, ValidatorSetConfig, SessionConfig
};
```

## Run

Once you have set up the pallet in your node/node-template and everything compiles, watch this video to see how to run the chain and add validators - https://www.youtube.com/watch?v=lIYxE-tOAdw.

To use the pallet with the `Collective` pallet, follow the steps in [docs/council-integration.md](./docs/council-integration.md).

## Types for Polkadot JS Apps/API

```json
{
  "Keys": "SessionKeys2"
}
```

## Disclaimer

This code not audited and reviewed for production use cases. You can expect bugs and security vulnerabilities. Do not use it as-is in real applications.
