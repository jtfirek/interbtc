// Copyright 2021-2022 Zenlink.
// Licensed under Apache 2.0.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![feature(array_windows)]

#[cfg(test)]
mod mock;

#[cfg(any(feature = "runtime-benchmarks", test))]
pub mod benchmarking;
#[cfg(test)]
mod test;
pub mod weights;
pub use weights::WeightInfo;

use codec::{Decode, Encode};

use sp_runtime::traits::{AtLeast32BitUnsigned, One, Zero};
use sp_std::{fmt::Debug, vec::Vec};

use frame_support::{
    dispatch::{Codec, DispatchResult},
    pallet_prelude::*,
    transactional,
};

use dex_general::{AssetBalance, ExportDexGeneral};
use dex_stable::traits::StableAmmApi;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct StablePath<PoolId, CurrencyId> {
    pub pool_id: PoolId,
    pub base_pool_id: PoolId,
    pub mode: StableSwapMode,
    pub from_currency: CurrencyId,
    pub to_currency: CurrencyId,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub enum StableSwapMode {
    Single,
    FromBase,
    ToBase,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub enum Route<PoolId, CurrencyId> {
    Stable(StablePath<PoolId, CurrencyId>),
    Normal(Vec<CurrencyId>),
}

impl<PoolId, CurrencyId: Clone> Route<PoolId, CurrencyId> {
    fn input_currency(&self) -> Option<CurrencyId> {
        match self {
            Route::Stable(x) => Some(x.from_currency.clone()),
            Route::Normal(x) => x.first().cloned(),
        }
    }
    fn output_currency(&self) -> Option<CurrencyId> {
        match self {
            Route::Stable(x) => Some(x.to_currency.clone()),
            Route::Normal(x) => x.last().cloned(),
        }
    }
    fn number_of_swaps(&self) -> usize {
        match self {
            Route::Stable(_) => 1,
            Route::Normal(x) => x.len().saturating_sub(1),
        }
    }
}
pub use pallet::*;

#[allow(type_alias_bounds)]
type AccountIdOf<T: Config> = <T as frame_system::Config>::AccountId;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type StablePoolId: Parameter + Codec + Copy + Ord + AtLeast32BitUnsigned + Zero + One + Default;

        type Balance: Parameter
            + Member
            + AtLeast32BitUnsigned
            + Codec
            + Default
            + Copy
            + MaybeSerializeDeserialize
            + Debug
            + MaxEncodedLen
            + From<AssetBalance>
            + Into<AssetBalance>
            + TypeInfo;

        // The currency id used in both amms
        type CurrencyId: Parameter + Member + Copy + MaybeSerializeDeserialize + Ord + TypeInfo + MaxEncodedLen;

        type NormalAmm: ExportDexGeneral<AccountIdOf<Self>, Self::CurrencyId>;

        type StableAMM: StableAmmApi<Self::StablePoolId, Self::CurrencyId, AccountIdOf<Self>, Self::Balance>;

        /// The maximum number of swaps allowed in routes
        #[pallet::constant]
        type MaxSwaps: Get<u16>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    #[pallet::event]
    pub enum Event<T: Config> {}

    #[pallet::error]
    pub enum Error<T> {
        MismatchPoolAndCurrencyId,
        Deadline,
        InvalidRoutes,
        ConvertCurrencyFailed,
        AmountSlippage,
        InvalidPath,
        ExceededSwapLimit,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::WeightInfo::swap_exact_token_for_tokens_through_stable_pool())]
        #[transactional]
        pub fn swap_exact_token_for_tokens_through_stable_pool(
            origin: OriginFor<T>,
            amount_in: T::Balance,
            amount_out_min: T::Balance,
            routes: Vec<Route<T::StablePoolId, T::CurrencyId>>,
            to: T::AccountId,
            deadline: T::BlockNumber,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let now = frame_system::Pallet::<T>::block_number();
            ensure!(deadline > now, Error::<T>::Deadline);

            Self::validate_routes(&routes)?;

            let mut amount_out = amount_in;
            let mut receiver = who.clone();

            for (i, route) in routes.iter().enumerate() {
                if i == routes.len() - 1 {
                    receiver = to.clone();
                }
                match route {
                    Route::Stable(stable_path) => {
                        (amount_out) = Self::stable_swap(&who, stable_path, amount_out, &receiver)?;
                    }
                    Route::Normal(path) => {
                        let amounts = T::NormalAmm::get_amount_out_by_path(amount_out.into(), path)?;
                        Self::swap(&who, amount_out, path, &receiver)?;
                        amount_out = T::Balance::from(*amounts.last().ok_or(Error::<T>::InvalidPath)?);
                    }
                }
            }

            ensure!(amount_out >= amount_out_min, Error::<T>::AmountSlippage);

            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    fn stable_swap(
        who: &T::AccountId,
        path: &StablePath<T::StablePoolId, T::CurrencyId>,
        amount_in: T::Balance,
        to: &T::AccountId,
    ) -> Result<T::Balance, DispatchError> {
        let out_amount = match path.mode {
            StableSwapMode::Single => {
                let from_index = Self::currency_index_from_stable_pool(path.pool_id, path.from_currency)?;
                let to_index = Self::currency_index_from_stable_pool(path.pool_id, path.to_currency)?;
                T::StableAMM::swap(who, path.pool_id, from_index, to_index, amount_in, Zero::zero(), to)?
            }
            StableSwapMode::FromBase => {
                let from_index = Self::currency_index_from_stable_pool(path.base_pool_id, path.from_currency)?;
                let to_index = Self::currency_index_from_stable_pool(path.pool_id, path.to_currency)?;

                T::StableAMM::swap_pool_from_base(
                    who,
                    path.pool_id,
                    path.base_pool_id,
                    from_index,
                    to_index,
                    amount_in,
                    Zero::zero(),
                    to,
                )?
            }
            StableSwapMode::ToBase => {
                let from_index = Self::currency_index_from_stable_pool(path.pool_id, path.from_currency)?;
                let to_index = Self::currency_index_from_stable_pool(path.base_pool_id, path.to_currency)?;
                T::StableAMM::swap_pool_to_base(
                    who,
                    path.pool_id,
                    path.base_pool_id,
                    from_index,
                    to_index,
                    amount_in,
                    Zero::zero(),
                    to,
                )?
            }
        };
        Ok(out_amount)
    }

    fn swap(who: &T::AccountId, amount_in: T::Balance, path: &[T::CurrencyId], to: &T::AccountId) -> DispatchResult {
        T::NormalAmm::inner_swap_exact_assets_for_assets(who, amount_in.into(), Zero::zero(), path, to)
    }

    fn currency_index_from_stable_pool(
        pool_id: T::StablePoolId,
        currency_id: T::CurrencyId,
    ) -> Result<u32, DispatchError> {
        T::StableAMM::currency_index(pool_id, currency_id).ok_or_else(|| Error::<T>::MismatchPoolAndCurrencyId.into())
    }

    fn validate_routes(routes: &[Route<T::StablePoolId, T::CurrencyId>]) -> DispatchResult {
        let num_swaps = routes
            .iter()
            .map(|x| x.number_of_swaps())
            .fold(0usize, |a, b| a.saturating_add(b));
        ensure!(num_swaps <= T::MaxSwaps::get().into(), Error::<T>::ExceededSwapLimit);

        for [route_1, route_2] in routes.array_windows::<2>() {
            let output_1 = route_1.output_currency().ok_or(Error::<T>::InvalidPath)?;
            let input_2 = route_2.input_currency().ok_or(Error::<T>::InvalidPath)?;
            ensure!(output_1 == input_2, Error::<T>::InvalidRoutes);
        }
        Ok(())
    }
}
