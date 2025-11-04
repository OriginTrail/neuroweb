#![cfg(test)]

use super::new_test_ext;
use neuroweb_runtime::{AccountId, TracWrapperPalletId, Wrapper};
use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};
use sp_runtime::traits::AccountIdConversion;

#[test]
fn wrapper_pallet_account() {
    new_test_ext().execute_with(|| {
        let wrapper_pallet_account_raw: AccountId = Wrapper::pallet_account_id();
        let wrapper_pallet_account_formatted =
            wrapper_pallet_account_raw.to_ss58check_with_version(Ss58AddressFormat::custom(101));

        assert_eq!(
            wrapper_pallet_account_raw,
            TracWrapperPalletId::get().into_account_truncating()
        );

        assert_eq!(
            wrapper_pallet_account_formatted,
            "gJpDhAL2bdaamftxXHXTbNxL6EZcXrgSmeLJzn2ficD3n4oWV"
        );
    });
}
