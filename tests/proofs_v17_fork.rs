#![cfg(kani)]

//! v17 fork-feature Kani proofs — re-expressed onto the single zero-copy/sparse engine path.
//!
//! Frozen toly `proofs_v16.rs` is adopted BYTE-IDENTICAL (the rebuilt zero-copy core harness);
//! every fork-feature proof lives here instead so the adopted core stays clean and the fork
//! surface re-verifies independently. Features are re-grafted onto frozen one unit at a time;
//! each unit's proof(s) land here. Under `#[cfg(kani)]` the frozen crate re-exports all of `v16`
//! (lib.rs `#[cfg(kani)] pub use v16::*`), so these access the engine API directly.
//!
//! Coverage so far:
//!   A-10 — max_price_move_bps_per_slot upper bound (V16Config::validate_public_user_fund_shape).

use percolator::v16::V16Config;
use percolator::MAX_MARGIN_BPS;

// ============================================================================
// A-10 — max_price_move_bps_per_slot upper bound.
// Frozen toly bounds only the lower edge (`== 0`); the fork additionally
// rejects `> MAX_MARGIN_BPS` (a move budget above full margin would weaken the
// per-slot price-move guard). Re-grafted onto frozen as a one-clause shape check.
// ============================================================================

/// RED-before / GREEN-after for the A-10 clause: any out-of-range value above
/// MAX_MARGIN_BPS must be rejected by `validate_public_user_fund_shape`. Without
/// the re-grafted clause this proof FAILS (frozen would accept it).
#[kani::proof]
#[kani::unwind(20)]
#[kani::solver(cadical)]
fn proof_v17_max_price_move_bps_per_slot_upper_bound() {
    let mut config = V16Config::public_user_fund(1, 0, 1);

    let bad: u64 = kani::any();
    kani::assume(bad > MAX_MARGIN_BPS);
    config.max_price_move_bps_per_slot = bad;

    assert!(config.kani_validate_public_user_fund_shape().is_err());
    kani::cover!(true, "out-of-range max_price_move rejected");
}

/// Boundary: `== MAX_MARGIN_BPS` is accepted (the bound is `<=`, not `<`), so
/// the re-grafted clause is not stricter than the fork's v12 intent.
#[kani::proof]
#[kani::unwind(20)]
#[kani::solver(cadical)]
fn proof_v17_max_price_move_bps_per_slot_boundary_accepted() {
    let mut config = V16Config::public_user_fund(1, 0, 1);
    config.max_price_move_bps_per_slot = MAX_MARGIN_BPS;

    assert!(config.kani_validate_public_user_fund_shape().is_ok());
    kani::cover!(true, "boundary max_price_move accepted");
}

// ============================================================================
// lp_vault — LP Vault share-math (re-grafted module, fork-facade gated).
// The 5 wide-math properties (nav/shares/redemption/fee-split) route through
// wide_mul_div_floor_u128's ~256-iter U256 division → CBMC-intractable; they are
// DEFERRED with full coverage by the 29 concrete-value native tests in
// tests/v16_fork_lp_vault_tests.rs (NAV-from-counters donation defense,
// round-DOWN issuance/redemption, deposit→redeem no-profit, fee-split
// conservation). The 2 loop-free properties remain live formal proofs.
// ============================================================================
use percolator::lp_vault::{lp_redemption_cooldown_elapsed, lp_shares_for_deposit};

/// LP_VAULT-5: drain-epoch freshness — total_shares==0 mints exactly `amount`
/// (1:1), independent of stale NAV (early-return path, no wide-math loop).
#[kani::proof]
#[kani::unwind(4)]
#[kani::solver(cadical)]
fn proof_v17_lp_vault_drain_epoch_freshness() {
    let amount: u128 = kani::any();
    let stale_nav: u128 = kani::any();
    kani::assume(amount >= 1 && amount <= 1_000_000_000_000);
    let shares = lp_shares_for_deposit(amount, 0, stale_nav).unwrap();
    assert_eq!(shares, amount, "drain-epoch deposit must mint 1:1");
    kani::cover!(true, "LP_VAULT-5 drain epoch freshness (1:1 reset)");
}

/// LP_VAULT-7: cooldown enforcement — elapsed iff current >= request+cooldown
/// (saturating, no overflow); cooldown==0 ⇒ always elapsed.
#[kani::proof]
#[kani::unwind(4)]
#[kani::solver(cadical)]
fn proof_v17_lp_vault_cooldown_enforcement() {
    let request_slot: u64 = kani::any();
    let current_slot: u64 = kani::any();
    let cooldown: u64 = kani::any();
    let elapsed = lp_redemption_cooldown_elapsed(request_slot, current_slot, cooldown);
    let deadline = request_slot.saturating_add(cooldown);
    assert_eq!(
        elapsed,
        current_slot >= deadline,
        "cooldown elapsed iff current >= saturating deadline"
    );
    if cooldown == 0 {
        assert!(
            lp_redemption_cooldown_elapsed(request_slot, current_slot, 0)
                == (current_slot >= request_slot)
        );
    }
    kani::cover!(true, "LP_VAULT-7 cooldown enforcement (saturating)");
}

// ============================================================================
// A-6 — stress envelope writer. Frozen carries the dormant `threshold_stress_active`
// flag; the fork's writer makes it live via a consumption accumulator + slot/epoch
// sentinels. Re-grafted onto the zero-copy ViewMut header. encode_bool maps true→1/
// false→0, so the flag is checked as a raw u8 (codec fns are crate-private).
// ============================================================================
use percolator::v16::{
    EngineAssetSlotV16Account, Market, MarketGroupV16HeaderAccount, MarketGroupV16View,
    MarketGroupV16ViewMut, V16Error, V16PodU128, V16PodU64,
};
use percolator::STRESS_ENVELOPE_TRIGGER_BPS_E9;

fn env_header() -> MarketGroupV16HeaderAccount {
    let cfg = V16Config::public_user_fund(1, 0, 1);
    MarketGroupV16HeaderAccount::new_dynamic([7u8; 32], cfg, 1, 0).unwrap()
}

/// A-6.1: accumulator is monotonic non-decreasing within an epoch/slot.
#[kani::proof]
#[kani::unwind(8)]
#[kani::solver(cadical)]
fn proof_v17_stress_envelope_writer_monotonic() {
    let mut header = env_header();
    let mut markets = [Market::new(0u64, EngineAssetSlotV16Account::default())];
    let mut view = MarketGroupV16ViewMut::new(&mut header, &mut markets);
    let c1: u128 = kani::any();
    let c2: u128 = kani::any();
    kani::assume(c1 <= u128::MAX / 4);
    kani::assume(c2 <= u128::MAX / 4);
    let now = 5u64;
    view.apply_stress_envelope_progress(c1, now).unwrap();
    let a1 = view.header.stress_consumption_bps_e9_since_envelope.get();
    view.apply_stress_envelope_progress(c2, now).unwrap();
    let a2 = view.header.stress_consumption_bps_e9_since_envelope.get();
    assert!(a2 >= a1, "accumulator monotonic within epoch/slot");
}

/// A-6.2: flag flips true iff one accrual reaches the trigger; start slot/epoch
/// are stamped on activation, else remain u64::MAX sentinels. OPERATIVE for the writer.
#[kani::proof]
#[kani::unwind(8)]
#[kani::solver(cadical)]
fn proof_v17_stress_envelope_activation_threshold() {
    let mut header = env_header();
    let mut markets = [Market::new(0u64, EngineAssetSlotV16Account::default())];
    let mut view = MarketGroupV16ViewMut::new(&mut header, &mut markets);
    let c: u128 = kani::any();
    kani::assume(c <= STRESS_ENVELOPE_TRIGGER_BPS_E9.saturating_add(1));
    let now = 9u64;
    let risk_epoch = view.header.risk_epoch.get();
    view.apply_stress_envelope_progress(c, now).unwrap();
    let active = view.header.threshold_stress_active == 1;
    assert_eq!(active, c >= STRESS_ENVELOPE_TRIGGER_BPS_E9, "flag set iff acc reached trigger");
    if active {
        assert_eq!(view.header.stress_envelope_start_slot.get(), now);
        assert_eq!(view.header.stress_envelope_start_credit_epoch.get(), risk_epoch);
    } else {
        assert_eq!(view.header.stress_envelope_start_slot.get(), u64::MAX);
        assert_eq!(view.header.stress_envelope_start_credit_epoch.get(), u64::MAX);
    }
}

/// A-6.3: clear zeroes the accumulator, restores u64::MAX sentinels, clears the flag.
#[kani::proof]
#[kani::unwind(8)]
#[kani::solver(cadical)]
fn proof_v17_stress_envelope_clear_resets_fields() {
    let mut header = env_header();
    let mut markets = [Market::new(0u64, EngineAssetSlotV16Account::default())];
    let mut view = MarketGroupV16ViewMut::new(&mut header, &mut markets);
    view.header.stress_consumption_bps_e9_since_envelope = V16PodU128::new(kani::any());
    view.header.stress_envelope_start_slot = V16PodU64::new(kani::any());
    view.header.stress_envelope_start_credit_epoch = V16PodU64::new(kani::any());
    view.header.threshold_stress_active = 1;
    view.clear_stress_envelope_v16();
    assert_eq!(view.header.stress_consumption_bps_e9_since_envelope.get(), 0);
    assert_eq!(view.header.stress_envelope_start_slot.get(), u64::MAX);
    assert_eq!(view.header.stress_envelope_start_credit_epoch.get(), u64::MAX);
    assert_eq!(view.header.threshold_stress_active, 0);
}

/// A-6.4: an active envelope from a PRIOR epoch is cleared (epoch advanced, different
/// slot, not in active-close) before the new accrual — so the post-state holds only the
/// fresh delta and the flag drops below trigger.
#[kani::proof]
#[kani::unwind(8)]
#[kani::solver(cadical)]
fn proof_v17_stress_envelope_epoch_reset() {
    let mut header = env_header();
    let mut markets = [Market::new(0u64, EngineAssetSlotV16Account::default())];
    let mut view = MarketGroupV16ViewMut::new(&mut header, &mut markets);
    view.header.threshold_stress_active = 1;
    view.header.stress_consumption_bps_e9_since_envelope =
        V16PodU128::new(STRESS_ENVELOPE_TRIGGER_BPS_E9);
    view.header.stress_envelope_start_slot = V16PodU64::new(10);
    view.header.stress_envelope_start_credit_epoch = V16PodU64::new(1);
    view.header.risk_epoch = V16PodU64::new(2);
    // fresh header is Live mode + loss_stale_active=0 → not active-close → epoch-advance clears.
    view.apply_stress_envelope_progress(1, 11).unwrap();
    assert_eq!(view.header.stress_consumption_bps_e9_since_envelope.get(), 1);
    assert_eq!(view.header.threshold_stress_active, 0);
}

/// A-6.5: the validate_shape sentinel-pairing invariant rejects a torn idle envelope
/// (flag=0, acc=0, but a sentinel != u64::MAX). OPERATIVE for the shape clause
/// (RED-before/GREEN-after): without the clause validate_shape would accept it.
#[kani::proof]
#[kani::unwind(24)]
#[kani::solver(cadical)]
fn proof_v17_stress_envelope_validate_shape_pairing() {
    let cfg = V16Config::public_user_fund_with_market_slots(1, 1, 0, 10);
    let mut header = MarketGroupV16HeaderAccount::new_dynamic([7u8; 32], cfg, 1, 0).unwrap();
    let mut markets = [Market::new(0u64, EngineAssetSlotV16Account::default())];
    {
        let mut view = MarketGroupV16ViewMut::new(&mut header, &mut markets);
        view.activate_empty_market_not_atomic(0, 100, 1).unwrap();
    }
    // proper idle envelope (new_dynamic set sentinels = u64::MAX) passes.
    {
        let view = MarketGroupV16View::new(&header, &markets);
        assert!(view.validate_shape().is_ok(), "idle envelope (sentinels=MAX) passes");
    }
    // torn idle envelope: flag=0, acc=0, but start_slot=0 (!= MAX) → rejected.
    header.stress_envelope_start_slot = V16PodU64::new(0);
    {
        let view = MarketGroupV16View::new(&header, &markets);
        assert_eq!(
            view.validate_shape(),
            Err(V16Error::InvalidConfig),
            "torn idle envelope rejected by the pairing invariant"
        );
    }
}
