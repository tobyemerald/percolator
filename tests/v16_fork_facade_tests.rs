//! fork-port A-4: runtime unit tests for `v16::fork_facade`.
//!
//! These tests live in a separate file so the v16 baseline test files
//! (`v16_spec_tests.rs`, `v16_fuzzing.rs`, `proofs_v16*.rs`) remain
//! untouched and the fork-facade surface can be verified independently.
//!
//! Coverage:
//!   - account-equity aliases (`account_equity_maint_raw`,
//!     `account_equity_net`, `account_equity_init_raw`,
//!     `account_equity_init_net`, `account_equity_withdraw_raw`,
//!     `account_equity_trade_open_raw`)
//!   - predicates (`is_above_initial_margin`, `is_terminal_ready`,
//!     `is_resolved`, `is_used`, `check_conservation`,
//!     `exact_solvency_envelope_ok`)
//!   - accessors (`haircut_ratio`, `set_owner`, `try_released_pos`,
//!     `try_notional`, `try_effective_matured_pnl`,
//!     `max_safe_flat_conversion_released`)
//!
//! Kani proofs for the same predicates live in `tests/proofs_v16_fork.rs`
//! (`proof_v16_is_terminal_ready_iff_counters_zero`,
//! `proof_v16_check_conservation_matches_vault_invariant`,
//! `proof_v16_set_owner_no_overwrite_no_zero`).

use percolator::v16::{
    fork_facade, MarketGroupV16, MarketModeV16, PortfolioAccountV16, ProvenanceHeaderV16,
    SideV16, V16Config,
};

fn baseline_config() -> V16Config {
    // `public_user_fund(1, 0, 1)` exercises the envelope early-return path
    // — matches the gating used by `proofs_v16_fork.rs::baseline_config`.
    V16Config::public_user_fund(1, 0, 1)
}

fn baseline_group() -> MarketGroupV16 {
    MarketGroupV16::new([1u8; 32], baseline_config()).unwrap()
}

fn baseline_account() -> PortfolioAccountV16 {
    let header = ProvenanceHeaderV16::new([1u8; 32], [2u8; 32], [7u8; 32]);
    let mut a = PortfolioAccountV16::empty(header);
    a.capital = 1_000_000;
    a
}

// ---------------------------------------------------------------------------
// Account-equity aliases.
// ---------------------------------------------------------------------------

#[test]
fn account_equity_aliases_match_underlying_formulas() {
    let mut a = baseline_account();
    a.pnl = 250_000;
    a.fee_credits = -10_000; // 10_000 owed.

    // _maint_raw = capital + pnl - fee_debt = 1_000_000 + 250_000 - 10_000.
    assert_eq!(
        fork_facade::account_equity_maint_raw(&a).unwrap(),
        1_240_000
    );

    // _net = max(0, _maint_raw).
    assert_eq!(fork_facade::account_equity_net(&a).unwrap(), 1_240_000);

    // _init_raw = capital + min(pnl, 0) - fee_debt = 1_000_000 + 0 - 10_000.
    assert_eq!(
        fork_facade::account_equity_init_raw(&a).unwrap(),
        990_000
    );

    // _init_net = max(0, _init_raw).
    assert_eq!(fork_facade::account_equity_init_net(&a).unwrap(), 990_000);

    // _withdraw_raw == _init_raw (v12 fork alias).
    assert_eq!(
        fork_facade::account_equity_withdraw_raw(&a).unwrap(),
        fork_facade::account_equity_init_raw(&a).unwrap()
    );
}

#[test]
fn account_equity_net_clamps_negative_to_zero() {
    let mut a = baseline_account();
    a.capital = 100;
    a.pnl = -1_000; // overall equity = -900.

    assert_eq!(fork_facade::account_equity_maint_raw(&a).unwrap(), -900);
    assert_eq!(fork_facade::account_equity_net(&a).unwrap(), 0);
}

#[test]
fn account_equity_trade_open_raw_uses_pnl_override() {
    let mut a = baseline_account();
    a.pnl = 500_000; // current pnl ignored; override drives the result.

    // Override negative pnl: equity = capital + override - 0 = 1_000_000 - 200_000.
    assert_eq!(
        fork_facade::account_equity_trade_open_raw(&a, -200_000).unwrap(),
        800_000
    );

    // Override positive pnl: equity = capital + min(override,0) = 1_000_000.
    assert_eq!(
        fork_facade::account_equity_trade_open_raw(&a, 500_000).unwrap(),
        1_000_000
    );
}

// ---------------------------------------------------------------------------
// Predicates.
// ---------------------------------------------------------------------------

#[test]
fn is_terminal_ready_true_on_fresh_group() {
    let group = baseline_group();
    assert!(fork_facade::is_terminal_ready(&group));
}

#[test]
fn is_terminal_ready_false_on_nonzero_counters() {
    for case in 0..3 {
        let mut group = baseline_group();
        match case {
            0 => group.b_stale_account_count = 1,
            1 => group.stale_certificate_count = 1,
            2 => group.negative_pnl_account_count = 1,
            _ => unreachable!(),
        }
        assert!(
            !fork_facade::is_terminal_ready(&group),
            "case {} should disqualify terminal-ready",
            case
        );
    }
}

#[test]
fn is_terminal_ready_false_on_pending_loss_barrier() {
    let mut group = baseline_group();
    group.pending_domain_loss_barriers[0] = 42;
    assert!(!fork_facade::is_terminal_ready(&group));
}

#[test]
fn is_resolved_matches_market_mode() {
    let mut group = baseline_group();
    assert!(!fork_facade::is_resolved(&group)); // fresh group is Live.

    group.mode = MarketModeV16::Resolved;
    assert!(fork_facade::is_resolved(&group));

    group.mode = MarketModeV16::Recovery;
    assert!(!fork_facade::is_resolved(&group));
}

#[test]
fn check_conservation_holds_when_vault_covers_obligations() {
    let mut group = baseline_group();
    group.vault = 1000;
    group.c_tot = 600;
    group.insurance = 400;
    assert!(fork_facade::check_conservation(&group));

    // Below-budget vault: invariant breaks.
    group.vault = 999;
    assert!(!fork_facade::check_conservation(&group));

    // Exact equality is conservation-OK.
    group.vault = 1000;
    assert!(fork_facade::check_conservation(&group));
}

#[test]
fn check_conservation_overflow_safe() {
    let mut group = baseline_group();
    group.vault = u128::MAX;
    group.c_tot = u128::MAX;
    group.insurance = 1; // c_tot + insurance overflows → must return false.
    assert!(!fork_facade::check_conservation(&group));
}

#[test]
fn is_used_false_on_empty_account_true_after_active_leg() {
    use percolator::v16::active_bitmap_set;

    let mut a = baseline_account();
    assert!(!fork_facade::is_used(&a));

    active_bitmap_set(&mut a.active_bitmap, 0).unwrap();
    assert!(fork_facade::is_used(&a));
}

#[test]
fn is_above_initial_margin_requires_valid_cert() {
    let a = baseline_account();
    // Fresh account: cert.valid = false → IM check fails.
    assert!(!fork_facade::is_above_initial_margin(&a));
}

#[test]
fn is_above_initial_margin_true_when_certified_equity_meets_req() {
    let mut a = baseline_account();
    a.health_cert.valid = true;
    a.health_cert.certified_equity = 1_000;
    a.health_cert.certified_initial_req = 500;
    assert!(fork_facade::is_above_initial_margin(&a));

    // Equity below IM req fails.
    a.health_cert.certified_initial_req = 1_500;
    assert!(!fork_facade::is_above_initial_margin(&a));
}

#[test]
fn is_above_initial_margin_trade_open_counterfactual() {
    let mut a = baseline_account();
    a.health_cert.valid = true;
    a.health_cert.certified_initial_req = 800_000;

    // capital = 1_000_000, override pnl = 0 → equity = 1_000_000 >= IM_req.
    assert!(
        fork_facade::is_above_initial_margin_trade_open(&a, 0).unwrap()
    );

    // Override -300_000 → equity = 700_000 < IM_req (800_000).
    assert!(
        !fork_facade::is_above_initial_margin_trade_open(&a, -300_000).unwrap()
    );
}

#[test]
fn is_above_initial_margin_trade_open_errors_on_stale_cert() {
    let a = baseline_account();
    // cert.valid = false → must error (not just return false).
    assert!(
        fork_facade::is_above_initial_margin_trade_open(&a, 0).is_err()
    );
}

#[test]
fn exact_solvency_envelope_ok_accepts_safe_baseline_config() {
    // `public_user_fund(1, 0, 1)` lands the early-return safe envelope path.
    let cfg = baseline_config();
    assert!(fork_facade::exact_solvency_envelope_ok(&cfg));
}

// ---------------------------------------------------------------------------
// Accessors / mutators.
// ---------------------------------------------------------------------------

#[test]
fn haircut_ratio_zero_when_no_matured_pnl() {
    let group = baseline_group();
    assert_eq!(fork_facade::haircut_ratio(&group), (0, 0));
}

#[test]
fn haircut_ratio_clamps_numerator_at_denominator() {
    let mut group = baseline_group();
    group.pnl_matured_pos_tot = 1_000; // den
    group.pnl_pos_bound_tot = 5_000; // residual ceiling
    group.pnl_pos_tot = 5_000;

    // Residual >= den → num clamped at den.
    assert_eq!(fork_facade::haircut_ratio(&group), (1_000, 1_000));

    // Now starve residual: num drops.
    group.pnl_pos_bound_tot = 250;
    group.pnl_pos_tot = 250;
    assert_eq!(fork_facade::haircut_ratio(&group), (250, 1_000));
}

#[test]
fn set_owner_rejects_zero_owner() {
    let mut a = baseline_account();
    let r = fork_facade::set_owner(&mut a, [0u8; 32]);
    assert!(r.is_err());
}

#[test]
fn set_owner_rejects_overwrite_with_different_owner() {
    let mut a = baseline_account();
    // baseline_account starts with owner = [7u8; 32].
    let r = fork_facade::set_owner(&mut a, [9u8; 32]);
    assert!(r.is_err());
    assert_eq!(a.owner, [7u8; 32], "owner must be unchanged on reject");
}

#[test]
fn set_owner_accepts_idempotent_same_owner() {
    let mut a = baseline_account();
    let r = fork_facade::set_owner(&mut a, [7u8; 32]);
    assert!(r.is_ok());
}

#[test]
fn set_owner_accepts_claim_on_empty_slot() {
    let header = ProvenanceHeaderV16::new([1u8; 32], [2u8; 32], [0u8; 32]);
    let mut a = PortfolioAccountV16::empty(header);
    // PortfolioAccountV16::empty propagates header.owner = [0u8; 32].
    assert_eq!(a.owner, [0u8; 32]);

    let claimer = [42u8; 32];
    fork_facade::set_owner(&mut a, claimer).unwrap();
    assert_eq!(a.owner, claimer);
}

#[test]
fn try_released_pos_is_positive_pnl_minus_reserved() {
    let mut a = baseline_account();
    a.pnl = 1_000;
    a.reserved_pnl = 300;
    assert_eq!(fork_facade::try_released_pos(&a).unwrap(), 700);

    // Negative pnl → released-pos is 0 (saturated at max(0, pnl)).
    a.pnl = -100;
    a.reserved_pnl = 0;
    assert_eq!(fork_facade::try_released_pos(&a).unwrap(), 0);
}

#[test]
fn try_released_pos_underflow_surfaces_as_error() {
    let mut a = baseline_account();
    a.pnl = 100;
    a.reserved_pnl = 200; // reserved > released → underflow.
    assert!(fork_facade::try_released_pos(&a).is_err());
}

#[test]
fn try_notional_zero_when_no_active_leg() {
    let group = baseline_group();
    let a = baseline_account();
    // No legs active → must return Ok(0) (not error).
    let r = fork_facade::try_notional(&group, &a, 0, 1_000_000);
    assert_eq!(r.unwrap(), 0);
}

#[test]
fn try_notional_with_active_leg_calls_risk_notional_ceil() {
    use percolator::v16::active_bitmap_set;
    use percolator::POS_SCALE;

    let group = baseline_group();
    let mut a = baseline_account();
    // Wire up a single active leg with non-zero basis.
    a.legs[0].active = true;
    a.legs[0].asset_index = 0;
    a.legs[0].side = SideV16::Long;
    a.legs[0].basis_pos_q = POS_SCALE as i128; // 1 unit position.
    active_bitmap_set(&mut a.active_bitmap, 0).unwrap();

    // notional = ceil(1 * price * POS_SCALE / POS_SCALE) = price.
    let n = fork_facade::try_notional(&group, &a, 0, 1_234).unwrap();
    assert_eq!(n, 1_234);
}

#[test]
fn try_effective_matured_pnl_zero_for_account_without_source_claims() {
    let group = baseline_group();
    let mut a = baseline_account();
    // pnl >= 0 with no source claims → support = 0 → matured = 0.
    a.pnl = 500_000;
    let m = fork_facade::try_effective_matured_pnl(&group, &a).unwrap();
    assert_eq!(m, 0);
}

#[test]
fn max_safe_flat_conversion_released_no_haircut_returns_cap() {
    let a = baseline_account();
    let out = fork_facade::max_safe_flat_conversion_released(&a, 1_000, 0, 1);
    assert_eq!(out, 1_000);
}

#[test]
fn max_safe_flat_conversion_released_impossible_returns_zero() {
    let a = baseline_account();
    // h_num > h_den → impossible per spec §4.12.
    let out = fork_facade::max_safe_flat_conversion_released(&a, 1_000, 5, 1);
    assert_eq!(out, 0);
}

#[test]
fn max_safe_flat_conversion_released_applies_inverse_haircut() {
    let a = baseline_account();
    // x_cap=100, h=(1,2) → floor(100 * 2 / 1) = 200.
    let out = fork_facade::max_safe_flat_conversion_released(&a, 100, 1, 2);
    assert_eq!(out, 200);
}
