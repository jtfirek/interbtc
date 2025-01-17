//! Autogenerated weights for clients-info
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2021-12-13, STEPS: `100`, REPEAT: 10, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// target/release/interbtc-standalone
// benchmark
// --chain
// dev
// --execution=wasm
// --wasm-execution=compiled
// --pallet
// clients-info
// --extrinsic
// *
// --steps
// 100
// --repeat
// 10
// --output
// crates/clients-info/src/default_weights.rs
// --template
// .deploy/weight-template.hbs


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use sp_std::marker::PhantomData;

/// Weight functions needed for vault_registry.
pub trait WeightInfo {
	fn set_current_client_release() -> Weight;
	fn set_pending_client_release() -> Weight;
}

/// Weights for vault_registry using the Substrate node and recommended hardware.
pub struct SubstrateWeight<T>(PhantomData<T>);
impl<T: frame_system::Config> WeightInfo for SubstrateWeight<T> {
	fn set_current_client_release() -> Weight {
		Weight::from_ref_time(4_130_000 as u64)
			.saturating_add(T::DbWeight::get().writes(1 as u64))
	}

	fn set_pending_client_release() -> Weight {
		Weight::from_ref_time(4_130_000 as u64)
	}
}

// For backwards compatibility and tests
impl WeightInfo for () {
	fn set_current_client_release() -> Weight {
		Weight::from_ref_time(4_130_000 as u64)
			.saturating_add(RocksDbWeight::get().writes(1 as u64))
	}

	fn set_pending_client_release() -> Weight {
		Weight::from_ref_time(4_130_000 as u64)
	}
}
