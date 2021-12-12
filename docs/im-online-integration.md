# How to use the Validator Set pallet with ImOnline pallet for automatic removal of offline validators

## Setup

* Before following the steps below, make sure you have completed all the steps in the [readme.md](../readme.md).

### Dependencies - runtime/cargo.toml

* Add the `im-online` pallet in your runtime's `cargo.toml`.

```toml
[dependencies.pallet-im-online]
default-features = false
git = 'https://github.com/paritytech/substrate.git'
tag = 'monthly-2021-12'
version = '4.0.0-dev'
```

```toml
std = [
	...
	'pallet-im-online/std',
]
```

### Pallet Initialization - runtime/src/lib.rs

* Import `ImOnlineId` and `Verify` `runtime/src/lib.rs`.

```rust
use sp_runtime::traits::{
	AccountIdLookup, BlakeTwo256, Block as BlockT, Verify, IdentifyAccount, NumberFor, OpaqueKeys,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
```

* Add the `ImOnline` key to the session keys for your runtime in `runtime/src/lib.rs`:

```rust
impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: Aura,
			pub grandpa: Grandpa,
			pub im_online: ImOnline,
		}
	}
```

* Add the `im-online` pallet and it's configuration. This will require more types to be imported.

```rust
parameter_types! {
	pub const ImOnlineUnsignedPriority: TransactionPriority = TransactionPriority::max_value();
	pub const MaxKeys: u32 = 10_000;
	pub const MaxPeerInHeartbeats: u32 = 10_000;
	pub const MaxPeerDataEncodingSize: u32 = 1_000;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Runtime
where
	Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		public: <Signature as Verify>::Signer,
		account: AccountId,
		nonce: Index,
	) -> Option<(Call, <UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload)> {
		let tip = 0;
		let period =
			BlockHashCount::get().checked_next_power_of_two().map(|c| c / 2).unwrap_or(2) as u64;
		let current_block = System::block_number().saturated_into::<u64>().saturating_sub(1);
		let era = Era::mortal(period, current_block);
		let extra = (
			frame_system::CheckSpecVersion::<Runtime>::new(),
			frame_system::CheckTxVersion::<Runtime>::new(),
			frame_system::CheckGenesis::<Runtime>::new(),
			frame_system::CheckEra::<Runtime>::from(era),
			frame_system::CheckNonce::<Runtime>::from(nonce),
			frame_system::CheckWeight::<Runtime>::new(),
			pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
		);
		let raw_payload = SignedPayload::new(call, extra)
			.map_err(|e| {
				log::warn!("Unable to create signed payload: {:?}", e);
			})
			.ok()?;
		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
		let address = account;
		let (call, extra, _) = raw_payload.deconstruct();
		Some((call, (sp_runtime::MultiAddress::Id(address), signature.into(), extra)))
	}
}

impl frame_system::offchain::SigningTypes for Runtime {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<C> frame_system::offchain::SendTransactionTypes<C> for Runtime
where
	Call: From<C>,
{
	type Extrinsic = UncheckedExtrinsic;
	type OverarchingCall = Call;
}

impl pallet_im_online::Config for Runtime {
	type AuthorityId = ImOnlineId;
	type Event = Event;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type ValidatorSet = ValidatorSet;
	type ReportUnresponsiveness = ValidatorSet;
	type UnsignedPriority = ImOnlineUnsignedPriority;
	type WeightInfo = pallet_im_online::weights::SubstrateWeight<Runtime>;
	type MaxKeys = MaxKeys;
	type MaxPeerInHeartbeats = MaxPeerInHeartbeats;
	type MaxPeerDataEncodingSize = MaxPeerDataEncodingSize;
}
```

* Declare the `SignedPayload` type in `runtime/src/lib.rs`.

```rust
pub type SignedPayload = generic::SignedPayload<Call, SignedExtra>;
```

* Add `im-online` pallet in `construct_runtime` macro.

```rust
construct_runtime!(
	pub enum Runtime where
		Block = Block,
		NodeBlock = opaque::Block,
		UncheckedExtrinsic = UncheckedExtrinsic
	{
		...
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		ValidatorSet: validator_set::{Pallet, Call, Storage, Event<T>, Config<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		ImOnline: pallet_im_online::{Pallet, Call, Storage, Event<T>, ValidateUnsigned, Config<T>},
		Aura: pallet_aura::{Pallet, Config<T>},
		Grandpa: pallet_grandpa::{Pallet, Call, Storage, Config, Event},
		...
		...
	}
);
```

### Genesis config - chain_spec.rs

* Add the `im-online` pallet in your node `cargo.toml`. This is needed because we need to import some types in the `chain_spec.rs`.

```toml
[dependencies.pallet-im-online]
default-features = false
git = 'https://github.com/paritytech/substrate.git'
tag = 'monthly-2021-10'
version = '4.0.0-dev'
```

* Import `ImOnlineId` in the `chain_spec.rs`.

```rust
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
```

* Also import `ImOnlineConfig` in  `chain_spec.rs`.

```rust
use node_template_runtime::{
	opaque::SessionKeys, AccountId, AuraConfig, BalancesConfig, GenesisConfig, GrandpaConfig,
	SessionConfig, Signature, SudoConfig, SystemConfig, ValidatorSetConfig, ImOnlineConfig,
	WASM_BINARY,
};
```

* Add `ImOnlineId` to the key generation functions in `chain_spec.rs`.

```rust
fn session_keys(aura: AuraId, grandpa: GrandpaId, im_online: ImOnlineId) -> SessionKeys {
	SessionKeys { aura, grandpa, im_online }
}

pub fn authority_keys_from_seed(s: &str) -> (AccountId, AuraId, GrandpaId, ImOnlineId) {
	(
		get_account_id_from_seed::<sr25519::Public>(s),
		get_from_seed::<AuraId>(s),
		get_from_seed::<GrandpaId>(s),
		get_from_seed::<ImOnlineId>(s),
	)
}
```

* Add genesis config in the `chain_spec.rs` file for the `im_online` pallet. Notice that the `ImOnlineId` has also been added to the tuple of keys, and it is also being used in the `keys` config for `session` pallet.

```rust
fn testnet_genesis(
	wasm_binary: &[u8],
	initial_authorities: Vec<(AccountId, AuraId, GrandpaId, ImOnlineId)>,
	root_key: AccountId,
	endowed_accounts: Vec<AccountId>,
	_enable_println: bool,
) -> GenesisConfig {
	GenesisConfig {
		system: SystemConfig {
			// Add Wasm runtime to storage.
			code: wasm_binary.to_vec(),
			changes_trie_config: Default::default(),
		},
		balances: BalancesConfig {
			// Configure endowed accounts with initial balance of 1 << 60.
			balances: endowed_accounts.iter().cloned().map(|k| (k, 1 << 60)).collect(),
		},
		validator_set: ValidatorSetConfig {
			initial_validators: initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
		},
		session: SessionConfig {
			keys: initial_authorities
				.iter()
				.map(|x| {
					(x.0.clone(), x.0.clone(), session_keys(x.1.clone(), x.2.clone(), x.3.clone()))
				})
				.collect::<Vec<_>>(),
		},
		aura: AuraConfig { authorities: vec![] },
		grandpa: GrandpaConfig { authorities: vec![] },
		im_online: ImOnlineConfig { keys: vec![] },
		sudo: SudoConfig {
			// Assign network admin rights.
			key: root_key,
		},
	}
}
```

### Types for Polkadot JS Apps/API

When using the `ImOnline` pallet, update the Polkadot JS custom types to the following:

```json
{
  "Keys": "SessionKeys3"
}
```

## Run

To run the node and network, follow the steps in [docs/local-network-setup.md](./local-network-setup.md).
