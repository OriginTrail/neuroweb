use super::*;
use crate::mock::*;
use frame_support::{
    assert_noop, assert_ok,
    traits::fungibles::Inspect,
};

#[test]
fn trac_wrap_works() {
    new_test_ext().execute_with(|| {
        let wrap_amount = 1000;

        // Check initial balances
        assert_eq!(Assets::balance(ForeignTracAssetId::get(), &ALICE), 1000000);
        assert_eq!(Assets::balance(LocalTracAssetId::get(), &ALICE), 0);

        // Perform wrap operation
        assert_ok!(Wrapper::trac_wrap(
            RuntimeOrigin::signed(ALICE),
            wrap_amount
        ));

        // Check balances after wrap
        assert_eq!(
            Assets::balance(ForeignTracAssetId::get(), &ALICE),
            1000000 - wrap_amount
        );
        assert_eq!(
            Assets::balance(LocalTracAssetId::get(), &ALICE),
            wrap_amount
        );

        // Check pallet foreign TRAC balance increased
        let pallet_account = Wrapper::pallet_account_id();
        assert_eq!(
            Assets::balance(ForeignTracAssetId::get(), &pallet_account),
            wrap_amount
        );

        // Check total supply of local TRAC
        assert_eq!(Wrapper::local_trac_total_supply(), wrap_amount);

        // Check event was emitted
        System::assert_last_event(RuntimeEvent::Wrapper(Event::TracWrapped {
            who: ALICE,
            amount: wrap_amount,
        }));
    });
}

#[test]
fn trac_wrap_with_insufficient_user_balance_fails() {
    new_test_ext().execute_with(|| {
        let wrap_amount = 2000000; // More than ALICE has

        // Should fail with insufficient balance
        assert_noop!(
            Wrapper::trac_wrap(RuntimeOrigin::signed(ALICE), wrap_amount),
            Error::<Test>::InsufficientUserBalance
        );
    });
}

#[test]
fn trac_wrap_with_zero_amount_fails() {
    new_test_ext().execute_with(|| {
        // Should fail with zero amount
        assert_noop!(
            Wrapper::trac_wrap(RuntimeOrigin::signed(ALICE), 0),
            Error::<Test>::ZeroAmount
        );
    });
}

#[test]
fn trac_unwrap_works() {
    new_test_ext().execute_with(|| {
        let wrap_amount = 1000;
        let unwrap_amount = 500;

        // First wrap some tokens
        assert_ok!(Wrapper::trac_wrap(
            RuntimeOrigin::signed(ALICE),
            wrap_amount
        ));

        // Check state after wrap
        assert_eq!(
            Assets::balance(LocalTracAssetId::get(), &ALICE),
            wrap_amount
        );
        assert_eq!(
            Assets::balance(ForeignTracAssetId::get(), &ALICE),
            1000000 - wrap_amount
        );

        // Now unwrap partial amount
        assert_ok!(Wrapper::trac_unwrap(
            RuntimeOrigin::signed(ALICE),
            unwrap_amount
        ));

        // Check balances after unwrap
        assert_eq!(
            Assets::balance(LocalTracAssetId::get(), &ALICE),
            wrap_amount - unwrap_amount
        );
        assert_eq!(
            Assets::balance(ForeignTracAssetId::get(), &ALICE),
            1000000 - wrap_amount + unwrap_amount
        );

        // Check pallet foreign TRAC balance decreased
        let pallet_account = Wrapper::pallet_account_id();
        assert_eq!(
            Assets::balance(ForeignTracAssetId::get(), &pallet_account),
            wrap_amount - unwrap_amount
        );

        // Check total supply of local TRAC decreased
        assert_eq!(
            Wrapper::local_trac_total_supply(),
            wrap_amount - unwrap_amount
        );

        // Check event was emitted
        System::assert_last_event(RuntimeEvent::Wrapper(Event::TracUnwrapped {
            who: ALICE,
            amount: unwrap_amount,
        }));
    });
}

#[test]
fn trac_unwrap_with_insufficient_user_balance_fails() {
    new_test_ext().execute_with(|| {
        let unwrap_amount = 1000;

        // ALICE has no local TRAC initially
        assert_eq!(Assets::balance(LocalTracAssetId::get(), &ALICE), 0);

        // Should fail with insufficient local balance
        assert_noop!(
            Wrapper::trac_unwrap(RuntimeOrigin::signed(ALICE), unwrap_amount),
            Error::<Test>::InsufficientUserBalance
        );
    });
}

#[test]
fn trac_unwrap_with_insufficient_pallet_balance_fails() {
    new_test_ext().execute_with(|| {
        let local_amount = 1000;

        // Mint local TRAC directly to ALICE without going through wrap
        // This creates a situation where there's local TRAC but no foreign TRAC in pallet
        assert_ok!(Assets::mint(
            RuntimeOrigin::signed(ALICE),
            LocalTracAssetId::get().into(),
            ALICE,
            local_amount
        ));

        // Check ALICE has local TRAC
        assert_eq!(
            Assets::balance(LocalTracAssetId::get(), &ALICE),
            local_amount
        );

        // Check pallet has no foreign TRAC
        let pallet_account = Wrapper::pallet_account_id();
        assert_eq!(
            Assets::balance(ForeignTracAssetId::get(), &pallet_account),
            0
        );

        // Should fail with insufficient pallet balance
        assert_noop!(
            Wrapper::trac_unwrap(RuntimeOrigin::signed(ALICE), local_amount),
            Error::<Test>::InsufficientPalletBalance
        );
    });
}

#[test]
fn trac_unwrap_with_zero_amount_fails() {
    new_test_ext().execute_with(|| {
        // Should fail with zero amount
        assert_noop!(
            Wrapper::trac_unwrap(RuntimeOrigin::signed(ALICE), 0),
            Error::<Test>::ZeroAmount
        );
    });
}

#[test]
fn multiple_wrap_unwrap_operations_work() {
    new_test_ext().execute_with(|| {
        let wrap_amount_1 = 1000;
        let wrap_amount_2 = 2000;
        let unwrap_amount = 1500;

        // First wrap
        assert_ok!(Wrapper::trac_wrap(
            RuntimeOrigin::signed(ALICE),
            wrap_amount_1
        ));
        assert_eq!(
            Assets::balance(LocalTracAssetId::get(), &ALICE),
            wrap_amount_1
        );

        // Second wrap
        assert_ok!(Wrapper::trac_wrap(
            RuntimeOrigin::signed(ALICE),
            wrap_amount_2
        ));
        assert_eq!(
            Assets::balance(LocalTracAssetId::get(), &ALICE),
            wrap_amount_1 + wrap_amount_2
        );

        // Partial unwrap
        assert_ok!(Wrapper::trac_unwrap(
            RuntimeOrigin::signed(ALICE),
            unwrap_amount
        ));
        assert_eq!(
            Assets::balance(LocalTracAssetId::get(), &ALICE),
            wrap_amount_1 + wrap_amount_2 - unwrap_amount
        );

        // Check pallet balance consistency
        let pallet_account = Wrapper::pallet_account_id();
        assert_eq!(
            Assets::balance(ForeignTracAssetId::get(), &pallet_account),
            wrap_amount_1 + wrap_amount_2 - unwrap_amount
        );

        // Check total supply consistency
        assert_eq!(
            Wrapper::local_trac_total_supply(),
            wrap_amount_1 + wrap_amount_2 - unwrap_amount
        );
    });
}

#[test]
fn cross_user_wrap_operations_work() {
    new_test_ext().execute_with(|| {
        let alice_wrap = 1000;
        let bob_wrap = 2000;

        // ALICE wraps
        assert_ok!(Wrapper::trac_wrap(RuntimeOrigin::signed(ALICE), alice_wrap));

        // BOB wraps
        assert_ok!(Wrapper::trac_wrap(RuntimeOrigin::signed(BOB), bob_wrap));

        // Check individual balances
        assert_eq!(Assets::balance(LocalTracAssetId::get(), &ALICE), alice_wrap);
        assert_eq!(Assets::balance(LocalTracAssetId::get(), &BOB), bob_wrap);

        // Check pallet total
        let pallet_account = Wrapper::pallet_account_id();
        assert_eq!(
            Assets::balance(ForeignTracAssetId::get(), &pallet_account),
            alice_wrap + bob_wrap
        );

        // Check total supply
        assert_eq!(Wrapper::local_trac_total_supply(), alice_wrap + bob_wrap);

        // ALICE unwraps all
        assert_ok!(Wrapper::trac_unwrap(
            RuntimeOrigin::signed(ALICE),
            alice_wrap
        ));
        assert_eq!(Assets::balance(LocalTracAssetId::get(), &ALICE), 0);

        // BOB still has local TRAC, pallet still has BOB's foreign TRAC
        assert_eq!(Assets::balance(LocalTracAssetId::get(), &BOB), bob_wrap);
        assert_eq!(
            Assets::balance(ForeignTracAssetId::get(), &pallet_account),
            bob_wrap
        );
        assert_eq!(Wrapper::local_trac_total_supply(), bob_wrap);
    });
}

#[test]
fn pallet_account_id_helper_is_correct() {
    new_test_ext().execute_with(|| {
        let pallet_account = Wrapper::pallet_account_id();
        assert_eq!(pallet_account, 7021806093762588525);
    });
}

#[test]
fn pallet_foreign_trac_balance_helper_is_correct() {
    new_test_ext().execute_with(|| {
        // Initially pallet should have no foreign TRAC
        assert_eq!(Wrapper::pallet_foreign_trac_balance(), 0);

        let wrap_amount = 1000;

        // After wrap, pallet should have foreign TRAC
        assert_ok!(Wrapper::trac_wrap(
            RuntimeOrigin::signed(ALICE),
            wrap_amount
        ));
        assert_eq!(Wrapper::pallet_foreign_trac_balance(), wrap_amount);

        // After unwrap, pallet balance should decrease
        let unwrap_amount = 300;
        assert_ok!(Wrapper::trac_unwrap(
            RuntimeOrigin::signed(ALICE),
            unwrap_amount
        ));
        assert_eq!(
            Wrapper::pallet_foreign_trac_balance(),
            wrap_amount - unwrap_amount
        );
    });
}

#[test]
fn local_trac_total_supply_helper_is_correct() {
    new_test_ext().execute_with(|| {
        // Initially should be zero
        assert_eq!(Wrapper::local_trac_total_supply(), 0);

        let wrap_amount = 1000;

        // After wrap, total supply should increase
        assert_ok!(Wrapper::trac_wrap(
            RuntimeOrigin::signed(ALICE),
            wrap_amount
        ));
        assert_eq!(Wrapper::local_trac_total_supply(), wrap_amount);

        // After unwrap, total supply should decrease
        let unwrap_amount = 400;
        assert_ok!(Wrapper::trac_unwrap(
            RuntimeOrigin::signed(ALICE),
            unwrap_amount
        ));
        assert_eq!(
            Wrapper::local_trac_total_supply(),
            wrap_amount - unwrap_amount
        );
    });
}
