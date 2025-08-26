#![cfg(test)]

use crate::mock::*;

#[test]
fn testnet_mode_default_value() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Params::testnet_mode(), false);
    });
}

#[test]
fn testnet_mode_set_to_true() {
    ExtBuilder::default()
        .testnet_mode(true)
        .build()
        .execute_with(|| {
            assert_eq!(Params::testnet_mode(), true);
        });
}

#[test]
fn testnet_mode_set_to_false() {
    ExtBuilder::default()
        .testnet_mode(false)
        .build()
        .execute_with(|| {
            assert_eq!(Params::testnet_mode(), false);
        });
}

#[test]
fn testnet_mode_can_be_changed() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(Params::testnet_mode(), false);

        Params::set_testnet_mode(true);
        assert_eq!(Params::testnet_mode(), true);

        Params::set_testnet_mode(false);
        assert_eq!(Params::testnet_mode(), false);
    });
}

// #[test]
// fn storage_query_works() {
//     ExtBuilder::default().testnet_mode(true).build().execute_with(|| {
//         assert!(TestnetMode::<Runtime>::exists());
//         assert_eq!(TestnetMode::<Runtime>::get(), true);
//     });
// }
//
// #[test]
// fn storage_default_query() {
//     ExtBuilder::default().build().execute_with(|| {
//         assert_eq!(TestnetMode::<Runtime>::get(), false);
//     });
// }
