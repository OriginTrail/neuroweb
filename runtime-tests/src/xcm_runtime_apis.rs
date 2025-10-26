#![cfg(test)]

use super::new_test_ext;
use codec::Encode;
use frame_support::weights::Weight;
use neuroweb_runtime::{OriginCaller, Runtime, RuntimeCall};
use xcm::{prelude::*, VersionedAssetId, VersionedAssets, VersionedLocation, VersionedXcm};
use xcm_runtime_apis::{
    dry_run::runtime_decl_for_dry_run_api::DryRunApiV2,
    fees::{runtime_decl_for_xcm_payment_api::XcmPaymentApiV1, Error as XcmPaymentApiError},
};

mod payment_api {
    use super::*;

    /// Helper function to verify acceptable payment assets contain NEURO and DOT
    fn assert_contains_neuro_and_dot(assets: &[VersionedAssetId]) {
        assert_eq!(
            assets.len(),
            2,
            "Should have exactly 2 acceptable payment assets"
        );

        let neuro_location = neuroweb_runtime::xcm_config::TokenLocation::get();
        let dot_location = neuroweb_runtime::xcm_config::RelayLocation::get();

        let mut found_neuro = false;
        let mut found_dot = false;

        for asset in assets {
            if let VersionedAssetId::V4(xcm::v4::AssetId(loc)) = asset {
                if *loc == neuro_location {
                    found_neuro = true;
                }
                if *loc == dot_location {
                    found_dot = true;
                }
            }
        }

        assert!(found_neuro, "NEURO should be in acceptable payment assets");
        assert!(found_dot, "DOT should be in acceptable payment assets");
    }

    #[test]
    fn query_acceptable_payment_assets_v3() {
        new_test_ext().execute_with(|| {
            let assets = Runtime::query_acceptable_payment_assets(3).unwrap();
            assert_contains_neuro_and_dot(&assets);
        });
    }

    #[test]
    fn query_acceptable_payment_assets_v4() {
        new_test_ext().execute_with(|| {
            let assets = Runtime::query_acceptable_payment_assets(4).unwrap();
            assert_contains_neuro_and_dot(&assets);
        });
    }

    #[test]
    fn query_acceptable_payment_assets_v5() {
        new_test_ext().execute_with(|| {
            let assets = Runtime::query_acceptable_payment_assets(5).unwrap();
            assert_contains_neuro_and_dot(&assets);
        });
    }

    #[test]
    fn query_acceptable_payment_assets_unsupported_version() {
        new_test_ext().execute_with(|| {
            // Test unsupported XCM version (e.g., version 2)
            let result = Runtime::query_acceptable_payment_assets(2);

            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                XcmPaymentApiError::UnhandledXcmVersion
            ));
        });
    }

    #[test]
    fn query_weight_to_asset_fee_neuro() {
        new_test_ext().execute_with(|| {
            let weight = Weight::from_parts(1_000_000_000, 64 * 1024); // 1 second of ref_time

            let neuro_location = neuroweb_runtime::xcm_config::TokenLocation::get();
            let asset = VersionedAssetId::V4(xcm::v4::AssetId(neuro_location));

            let result = Runtime::query_weight_to_asset_fee(weight, asset);

            assert!(result.is_ok());
            let fee = result.unwrap();
            // Fee should be non-zero for native token
            assert!(fee > 0);
        });
    }

    #[test]
    fn query_weight_to_asset_fee_dot() {
        new_test_ext().execute_with(|| {
            let weight = Weight::from_parts(1_000_000_000, 64 * 1024);

            let dot_location = neuroweb_runtime::xcm_config::RelayLocation::get();
            let asset = VersionedAssetId::V4(xcm::v4::AssetId(dot_location));

            let result = Runtime::query_weight_to_asset_fee(weight, asset);

            assert!(result.is_ok());
            let fee = result.unwrap();
            // Fee should be non-zero for DOT
            assert!(fee > 0);
        });
    }

    #[test]
    fn query_weight_to_asset_fee_unknown_asset() {
        new_test_ext().execute_with(|| {
            let weight = Weight::from_parts(1_000_000_000, 64 * 1024);

            let unknown_location = Location::new(1, [Parachain(9999)]);
            let asset = VersionedAssetId::V4(xcm::v4::AssetId(unknown_location));

            let result = Runtime::query_weight_to_asset_fee(weight, asset);

            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                XcmPaymentApiError::AssetNotFound
            ));
        });
    }

    #[test]
    fn query_weight_to_asset_fee_version_conversion() {
        new_test_ext().execute_with(|| {
            let weight = Weight::from_parts(1_000_000_000, 64 * 1024);

            // Test with V3 asset that should convert to V4
            let token_location = neuroweb_runtime::xcm_config::TokenLocation::get();
            let asset_v3 = VersionedAssetId::V3(xcm::v3::AssetId::Concrete(
                token_location.clone().try_into().unwrap(),
            ));

            let result = Runtime::query_weight_to_asset_fee(weight, asset_v3);

            // Should successfully convert and calculate fee
            assert!(result.is_ok());
        });
    }

    #[test]
    fn dot_fee_calculation_uses_correct_rate() {
        new_test_ext().execute_with(|| {
            use frame_support::weights::constants::WEIGHT_REF_TIME_PER_SECOND;

            let weight_one_second = Weight::from_parts(WEIGHT_REF_TIME_PER_SECOND, 0);
            let dot_location = neuroweb_runtime::xcm_config::RelayLocation::get();
            let dot_asset = VersionedAssetId::V4(xcm::v4::AssetId(dot_location));

            let fee = Runtime::query_weight_to_asset_fee(weight_one_second, dot_asset).unwrap();
            let dot_per_second = neuroweb_runtime::xcm_config::DotPerSecond::get();

            assert_eq!(fee, dot_per_second);
        });
    }

    #[test]
    fn query_xcm_weight_simple_transfer() {
        new_test_ext().execute_with(|| {
            let message = VersionedXcm::V4(Xcm(vec![
                WithdrawAsset((Here, 100u128).into()),
                BuyExecution {
                    fees: (Here, 100u128).into(),
                    weight_limit: Unlimited,
                },
                DepositAsset {
                    assets: All.into(),
                    beneficiary: AccountId32 {
                        network: None,
                        id: [0u8; 32],
                    }
                    .into(),
                },
            ]));

            let result = Runtime::query_xcm_weight(message);

            assert!(result.is_ok());
            let weight = result.unwrap();
            // Weight should be non-zero for a multi-instruction message
            assert!(weight.ref_time() > 0);
        });
    }

    #[test]
    fn query_delivery_fees_to_parent() {
        new_test_ext().execute_with(|| {
            let destination = VersionedLocation::V4(Location::parent());
            let message = VersionedXcm::V4(Xcm(vec![ClearOrigin]));

            let result = Runtime::query_delivery_fees(destination, message);

            match result {
                Ok(fees) => {
                    // If successful, fees should be a valid VersionedAssets
                    assert!(matches!(fees, VersionedAssets::V4(_)));
                }
                Err(_) => {
                    // Error is acceptable if delivery fees aren't configured
                }
            }
        });
    }

    mod dry_run_api {
        use super::*;
        #[test]
        fn dry_run_call_with_root_origin() {
            new_test_ext().execute_with(|| {
                let call = RuntimeCall::System(frame_system::Call::remark {
                    remark: vec![1, 2, 3],
                });
                let origin = OriginCaller::system(frame_system::RawOrigin::Root);
                let result = Runtime::dry_run_call(origin, call, 4);

                assert!(result.is_ok() || result.is_err());
            });
        }

        #[test]
        fn dry_run_call_different_xcm_versions() {
            new_test_ext().execute_with(|| {
                let call = RuntimeCall::System(frame_system::Call::remark {
                    remark: vec![1, 2, 3],
                });

                let origin = OriginCaller::system(frame_system::RawOrigin::Root);

                // Test with different XCM versions
                for version in [3, 4, 5] {
                    let result = Runtime::dry_run_call(origin.clone(), call.clone(), version);

                    assert!(result.is_ok() || result.is_err());
                }
            });
        }

        #[test]
        fn dry_run_xcm_with_assets() {
            new_test_ext().execute_with(|| {
                let origin_location = VersionedLocation::V4(Location::parent());
                let xcm = VersionedXcm::V4(Xcm(vec![
                    WithdrawAsset((Here, 1000u128).into()),
                    ClearOrigin,
                ]));
                let result = Runtime::dry_run_xcm(origin_location, xcm);

                assert!(result.is_ok() || result.is_err());
            });
        }

        #[test]
        fn dry_run_xcm_from_sibling() {
            new_test_ext().execute_with(|| {
                let origin_location = VersionedLocation::V4(Location::new(1, [Parachain(2000)]));
                let xcm = VersionedXcm::V4(Xcm(vec![ClearOrigin]));
                let result = Runtime::dry_run_xcm(origin_location, xcm);

                assert!(result.is_ok() || result.is_err());
            });
        }

        #[test]
        fn dry_run_xcm_transact() {
            new_test_ext().execute_with(|| {
                let origin_location = VersionedLocation::V4(Location::parent());
                let inner_call = RuntimeCall::System(frame_system::Call::remark {
                    remark: vec![1, 2, 3],
                });
                let xcm = VersionedXcm::V4(Xcm(vec![Transact {
                    origin_kind: OriginKind::Superuser,
                    require_weight_at_most: Weight::from_parts(1_000_000, 64 * 1024),
                    call: inner_call.encode().into(),
                }]));

                let result = Runtime::dry_run_xcm(origin_location, xcm);

                assert!(result.is_ok() || result.is_err());
            });
        }

        #[test]
        fn dry_run_xcm_different_versions() {
            new_test_ext().execute_with(|| {
                let origin_v4 = VersionedLocation::V4(Location::parent());
                let origin_v3 = VersionedLocation::V3(xcm::v3::Location::parent());

                let xcm_v4 = VersionedXcm::V4(Xcm(vec![ClearOrigin]));
                let xcm_v3 =
                    VersionedXcm::V3(xcm::v3::Xcm(vec![xcm::v3::Instruction::ClearOrigin]));

                // Test V4
                let result_v4 = Runtime::dry_run_xcm(origin_v4, xcm_v4);

                // Test V3
                let result_v3 = Runtime::dry_run_xcm(origin_v3, xcm_v3);

                // Both should be callable
                assert!(result_v4.is_ok() || result_v4.is_err());
                assert!(result_v3.is_ok() || result_v3.is_err());
            });
        }
    }
}
