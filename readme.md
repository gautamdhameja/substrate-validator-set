# Substrate Validator Set

A Substrate pallet to add/remove validators in Substrate-based PoA networks.

## Usage

* Add the module's dependency in the `Cargo.toml` of your runtime directory. Make sure to enter the correct path or git url of the pallet as per your setup.

```toml
[dependencies.substrate_validator_set]
package = 'substrate-validator-set'
git = 'https://github.com/gautamdhameja/substrate-validator-set.git'
default-features = false
```

* Make sure that you also have the Substrate [session pallet](https://github.com/paritytech/substrate/tree/master/frame/session) as part of your runtime. This is because the validator-set pallet is based on the session pallet.

* Declare the pallet in your `runtime/src/lib.rs`.

```rust

```
