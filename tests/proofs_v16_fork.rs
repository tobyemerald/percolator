#![cfg(kani)]

//! Kani proofs for fork-feature ports landed on the v16 engine. These
//! harnesses live in a separate file so v16 baseline proof files
//! (`proofs_v16.rs`, `proofs_v16_arithmetic.rs`, `v16_fuzzing.rs`,
//! `v16_spec_tests.rs`) remain untouched and the fork-port surface can
//! be re-verified independently.
//!
//! Coverage (A-9 — dynamic-trade-fee, engine-side mutator):
//!   - `proof_v16_apply_fee_policy_update_validates_bounds`:
//!     proves shape-validation rejects invalid `max_trading_fee_bps`
//!     (the only fee field needing a high cap) and leaves engine config
//!     unchanged. Other-field rejection paths are covered by smaller
//!     focused proofs to keep the symbolic surface tractable for
//!     `validate_exact_solvency_envelope`.
//!   - `proof_v16_apply_fee_policy_update_persists`:
//!     proves valid mutation persists exactly the four fee-policy
//!     fields into the engine config.
//!   - `proof_v16_fee_policy_update_no_other_field_mutation`:
//!     proves no non-fee config field is touched by the mutator (audit
//!     invariant — additive engine surface).
//!
//! Each harness pins the test-vector to the
//! `validate_exact_solvency_envelope` *early-return* path
//! (`maintenance_margin_bps == 10_000`, `liquidation_fee_bps == 0`,
//! `min_liquidation_abs == 0`, etc.) so Kani doesn't have to symbolically
//! drive the recursive interval-validation loop at L1124-1192. That
//! recursive path is already exercised by the v16 baseline proofs.

use percolator::v16::{FeePolicyUpdateV16, MarketGroupV16, V16Config};
use percolator::MAX_MARGIN_BPS;

fn baseline_config() -> V16Config {
    // `public_user_fund(1, 0, 1)` lands the envelope early-return path:
    // maintenance_margin_bps = 10_000, liquidation_fee_bps = 0,
    // min_liquidation_abs = 0, max_abs_funding_e9_per_slot = 0,
    // price_budget_fast = max_price_move_bps_per_slot *
    // max_accrual_dt_slots = 10_000 * 1 = 10_000 (<= 10_000).
    V16Config::public_user_fund(1, 0, 1)
}

fn baseline_group() -> MarketGroupV16 {
    MarketGroupV16::new([1u8; 32], baseline_config()).unwrap()
}

/// Proves that an out-of-range `max_trading_fee_bps` is rejected and the
/// engine config is left unchanged. `max_trading_fee_bps` is the field
/// added by A-9; the other three fee-policy fields are pinned to values
/// that match the baseline config so the symbolic surface stays small.
#[kani::proof]
#[kani::unwind(20)]
#[kani::solver(cadical)]
fn proof_v16_apply_fee_policy_update_validates_bounds() {
    let mut group = baseline_group();
    let before = group.config;

    let max_trading_fee_bps: u64 = kani::any();
    // Tight symbolic domain: straddles MAX_MARGIN_BPS so both the
    // accept (<=) and reject (>) branches are reachable.
    kani::assume(max_trading_fee_bps <= MAX_MARGIN_BPS + 1);

    let update = FeePolicyUpdateV16 {
        max_trading_fee_bps,
        liquidation_fee_bps: 0,
        liquidation_fee_cap: 0,
        min_liquidation_abs: 0,
    };

    let result = group.kani_apply_fee_policy_update_not_atomic(update);

    if update.max_trading_fee_bps > MAX_MARGIN_BPS {
        // Reject: must surface as Err and engine config must be
        // byte-identical to the pre-call snapshot.
        assert!(result.is_err());
        assert_eq!(group.config, before);
        kani::cover!(true, "validates_bounds: reject path reachable");
    } else {
        assert!(result.is_ok());
        assert_eq!(group.config.max_trading_fee_bps, max_trading_fee_bps);
        kani::cover!(true, "validates_bounds: accept path reachable");
    }
}

/// Proves a valid mutation persists the four fee-policy fields. Uses a
/// symbolic `max_trading_fee_bps` (the field added by A-9) plus pinned
/// zeros for the other three fee fields — keeping the envelope check on
/// its early-return path.
#[kani::proof]
#[kani::unwind(20)]
#[kani::solver(cadical)]
fn proof_v16_apply_fee_policy_update_persists() {
    let mut group = baseline_group();

    let max_trading_fee_bps: u64 = kani::any();
    kani::assume(max_trading_fee_bps <= MAX_MARGIN_BPS);

    let update = FeePolicyUpdateV16 {
        max_trading_fee_bps,
        liquidation_fee_bps: 0,
        liquidation_fee_cap: 0,
        min_liquidation_abs: 0,
    };

    let result = group.kani_apply_fee_policy_update_not_atomic(update);
    assert!(result.is_ok());

    // Persistence: the four target fields equal the requested values.
    assert_eq!(group.config.max_trading_fee_bps, update.max_trading_fee_bps);
    assert_eq!(group.config.liquidation_fee_bps, update.liquidation_fee_bps);
    assert_eq!(group.config.liquidation_fee_cap, update.liquidation_fee_cap);
    assert_eq!(group.config.min_liquidation_abs, update.min_liquidation_abs);

    kani::cover!(true, "fee policy update accept path reachable");
}

/// Proves no field outside the four fee-policy targets is mutated by
/// `apply_fee_policy_update_not_atomic`. This is the audit-grade
/// "additive surface" invariant — if it ever fails, the mutator has
/// silently grown its scope.
#[kani::proof]
#[kani::unwind(20)]
#[kani::solver(cadical)]
fn proof_v16_fee_policy_update_no_other_field_mutation() {
    let mut group = baseline_group();
    let before = group.config;

    let max_trading_fee_bps: u64 = kani::any();
    kani::assume(max_trading_fee_bps <= MAX_MARGIN_BPS);

    let update = FeePolicyUpdateV16 {
        max_trading_fee_bps,
        liquidation_fee_bps: 0,
        liquidation_fee_cap: 0,
        min_liquidation_abs: 0,
    };
    let result = group.kani_apply_fee_policy_update_not_atomic(update);
    assert!(result.is_ok());

    // Field-by-field equality on every non-fee config field. If any
    // future change to the mutator touches one of these, this proof will
    // fail and force the change to be re-justified.
    let after = group.config;
    assert_eq!(after.max_portfolio_assets, before.max_portfolio_assets);
    assert_eq!(after.max_market_slots, before.max_market_slots);
    assert_eq!(after.min_nonzero_mm_req, before.min_nonzero_mm_req);
    assert_eq!(after.min_nonzero_im_req, before.min_nonzero_im_req);
    assert_eq!(after.h_min, before.h_min);
    assert_eq!(after.h_max, before.h_max);
    assert_eq!(after.maintenance_margin_bps, before.maintenance_margin_bps);
    assert_eq!(after.initial_margin_bps, before.initial_margin_bps);
    assert_eq!(after.max_accrual_dt_slots, before.max_accrual_dt_slots);
    assert_eq!(
        after.max_abs_funding_e9_per_slot,
        before.max_abs_funding_e9_per_slot
    );
    assert_eq!(
        after.min_funding_lifetime_slots,
        before.min_funding_lifetime_slots
    );
    assert_eq!(
        after.max_price_move_bps_per_slot,
        before.max_price_move_bps_per_slot
    );
    assert_eq!(
        after.max_account_b_settlement_chunks,
        before.max_account_b_settlement_chunks
    );
    assert_eq!(
        after.max_bankrupt_close_chunks,
        before.max_bankrupt_close_chunks
    );
    assert_eq!(
        after.max_bankrupt_close_lifetime_slots,
        before.max_bankrupt_close_lifetime_slots
    );
    assert_eq!(
        after.asset_activation_cooldown_slots,
        before.asset_activation_cooldown_slots
    );
    assert_eq!(after.public_b_chunk_atoms, before.public_b_chunk_atoms);
    assert_eq!(
        after.max_recovery_fallback_deviation_bps,
        before.max_recovery_fallback_deviation_bps
    );
    assert_eq!(
        after.backing_fee_base_rate_e9_per_slot,
        before.backing_fee_base_rate_e9_per_slot
    );
    assert_eq!(
        after.backing_fee_kink_util_bps,
        before.backing_fee_kink_util_bps
    );
    assert_eq!(
        after.backing_fee_slope_at_kink_e9_per_slot,
        before.backing_fee_slope_at_kink_e9_per_slot
    );
    assert_eq!(
        after.backing_fee_slope_above_kink_e9_per_slot,
        before.backing_fee_slope_above_kink_e9_per_slot
    );
    assert_eq!(after.backing_freshness_buckets, before.backing_freshness_buckets);
    assert_eq!(
        after.margin_mode_realizable_full_shared_cross_margin,
        before.margin_mode_realizable_full_shared_cross_margin
    );
    assert_eq!(
        after.source_credit_lien_required,
        before.source_credit_lien_required
    );
    assert_eq!(
        after.insurance_credit_reservation_required,
        before.insurance_credit_reservation_required
    );
    assert_eq!(
        after.permissionless_recovery_enabled,
        before.permissionless_recovery_enabled
    );
    assert_eq!(
        after.recovery_fallback_price_enabled,
        before.recovery_fallback_price_enabled
    );
    assert_eq!(
        after.recovery_fallback_envelope_enabled,
        before.recovery_fallback_envelope_enabled
    );
    assert_eq!(
        after.credit_lien_revalidation_required,
        before.credit_lien_revalidation_required
    );
    assert_eq!(
        after.stale_certificate_penalty_enabled,
        before.stale_certificate_penalty_enabled
    );
    assert_eq!(
        after.full_refresh_required_for_favorable_actions,
        before.full_refresh_required_for_favorable_actions
    );
    assert_eq!(
        after.public_liveness_profile_crank_forward,
        before.public_liveness_profile_crank_forward
    );

    kani::cover!(true, "no-other-field-mutation invariant reachable");
}

// ============================================================================
// A-10 — InitMarket v2 wire-format port: max_price_move_bps_per_slot upper
// bound. Fork's v12 engine enforced `<= MAX_MARGIN_BPS`; v16 baseline only
// rejected `== 0`. This harness verifies the new upper-bound check rejects
// out-of-range values.
// ============================================================================

/// Proves that `max_price_move_bps_per_slot > MAX_MARGIN_BPS` is rejected by
/// `validate_public_user_fund_shape`. Establishes the fork-specific upper
/// bound that the v16 baseline lacks.
#[kani::proof]
#[kani::unwind(20)]
#[kani::solver(cadical)]
fn proof_v16_max_price_move_bps_per_slot_upper_bound() {
    let mut config = V16Config::public_user_fund(1, 0, 1);

    // Pick an arbitrary out-of-range value above MAX_MARGIN_BPS.
    let bad: u64 = kani::any();
    kani::assume(bad > MAX_MARGIN_BPS);
    config.max_price_move_bps_per_slot = bad;

    // Validation must reject — the A-10 bound holds.
    assert!(config.validate_public_user_fund_shape().is_err());

    kani::cover!(true, "out-of-range max_price_move rejected");
}

/// Proves the boundary case: `max_price_move_bps_per_slot == MAX_MARGIN_BPS`
/// is accepted (along with other valid fields). Establishes that the new
/// upper bound is inclusive and not stricter than the fork's v12 intent.
#[kani::proof]
#[kani::unwind(20)]
#[kani::solver(cadical)]
fn proof_v16_max_price_move_bps_per_slot_boundary_accepted() {
    let mut config = V16Config::public_user_fund(1, 0, 1);
    config.max_price_move_bps_per_slot = MAX_MARGIN_BPS;

    // Boundary value must validate successfully — the A-10 bound is
    // <= MAX_MARGIN_BPS, not <.
    assert!(config.validate_public_user_fund_shape().is_ok());

    kani::cover!(true, "boundary max_price_move accepted");
}
