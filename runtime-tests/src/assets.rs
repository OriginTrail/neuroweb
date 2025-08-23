#![cfg(test)]

use frame_support::traits::Get;
use neuroweb_runtime::{
    assets::{FOREIGN_TRAC_UNIFIED_ASSET_ID, FOREIGN_TRAC_UNIFIED_ASSET_ID_SEPOLIA},
    ForeignTracAssetId, Params,
};
use super::{new_test_ext, run_to_block};

#[test]
fn foreign_trac_asset_id_defaults_to_mainnet() {
    new_test_ext().execute_with(|| {
        // Initially testnet_mode should be false, so we should get mainnet asset ID
        assert_eq!(Params::testnet_mode(), false);
        assert_eq!(ForeignTracAssetId::get(), FOREIGN_TRAC_UNIFIED_ASSET_ID);
    });
}

#[test]
fn foreign_trac_asset_id_switches_to_sepolia_in_testnet_mode() {
    new_test_ext().execute_with(|| {
        // Set testnet mode to true
        Params::set_testnet_mode(true);
        assert_eq!(Params::testnet_mode(), true);
        assert_eq!(ForeignTracAssetId::get(), FOREIGN_TRAC_UNIFIED_ASSET_ID_SEPOLIA);
    });
}

#[test]
fn foreign_trac_asset_id_switches_back_to_mainnet() {
    new_test_ext().execute_with(|| {
        // Start in testnet mode
        Params::set_testnet_mode(true);
        assert_eq!(ForeignTracAssetId::get(), FOREIGN_TRAC_UNIFIED_ASSET_ID_SEPOLIA);

        // Switch back to mainnet mode
        Params::set_testnet_mode(false);
        assert_eq!(Params::testnet_mode(), false);
        assert_eq!(ForeignTracAssetId::get(), FOREIGN_TRAC_UNIFIED_ASSET_ID);
    });
}

#[test]
fn foreign_trac_asset_id_consistency_across_blocks() {
    new_test_ext().execute_with(|| {
        // Set testnet mode
        Params::set_testnet_mode(true);
        let testnet_asset_id = ForeignTracAssetId::get();

        // Run several blocks and ensure consistency
        run_to_block(5);
        assert_eq!(ForeignTracAssetId::get(), testnet_asset_id);
        assert_eq!(ForeignTracAssetId::get(), FOREIGN_TRAC_UNIFIED_ASSET_ID_SEPOLIA);
    });
}

#[test]
fn foreign_trac_asset_ids_are_different() {
    new_test_ext().execute_with(|| {
        let mainnet_id = FOREIGN_TRAC_UNIFIED_ASSET_ID;
        let testnet_id = FOREIGN_TRAC_UNIFIED_ASSET_ID_SEPOLIA;

        // Ensure the two asset IDs are actually different
        assert_ne!(mainnet_id, testnet_id);

        // Test both modes return the expected different IDs
        Params::set_testnet_mode(false);
        assert_eq!(ForeignTracAssetId::get(), mainnet_id);

        Params::set_testnet_mode(true);
        assert_eq!(ForeignTracAssetId::get(), testnet_id);
    });
}

#[test]
fn local_assets_pallet_account() {
    use neuroweb_runtime::{LocalAssetsPalletId, local_assets_pallet_account, foreign_assets_pallet_account, AccountId};
    use sp_runtime::traits::AccountIdConversion;
    use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};
    
    new_test_ext().execute_with(|| {
        let local_assets_pallet_account_raw: AccountId = local_assets_pallet_account();
        let local_assets_pallet_account_formatted = local_assets_pallet_account_raw.to_ss58check_with_version(Ss58AddressFormat::custom(101));
        println!("{:?}", sp_core::sr25519::Public::from_raw(<[u8; 32]>::from(foreign_assets_pallet_account())).to_ss58check_with_version(Ss58AddressFormat::custom(101)));

        assert_eq!(local_assets_pallet_account_raw, LocalAssetsPalletId::get().into_account_truncating());
        assert_eq!(local_assets_pallet_account_formatted, "gJpDhAL2bdaUCfRYcXhCkNuH5HAsChPVwQVBzzuHVvw1otqvq");
    });
}

#[test]
fn foreign_assets_pallet_account() {
    use neuroweb_runtime::{ForeignAssetsPalletId, foreign_assets_pallet_account, AccountId};
    use sp_runtime::traits::AccountIdConversion;
    use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};
    
    new_test_ext().execute_with(|| {
        let foreign_assets_pallet_account_raw: AccountId = foreign_assets_pallet_account();
        let foreign_assets_pallet_account_formatted = foreign_assets_pallet_account_raw.to_ss58check_with_version(Ss58AddressFormat::custom(101));

        assert_eq!(foreign_assets_pallet_account_raw, ForeignAssetsPalletId::get().into_account_truncating());
        assert_eq!(foreign_assets_pallet_account_formatted, "gJpDhAL2bdaQbzP81kSMAaCh1xS7jGkw6ZzFKb4UcHYVCXfdz");
    });
}
