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
