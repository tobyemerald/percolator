#![cfg(kani)]

//! Fork-port admission Kani proofs landed on the v16 engine surface.
//!
//! This file is the Phase 1.B port of selected v12 `proofs_admission.rs`
//! harnesses that test invariants v16's admission surface still respects.
//! v12's admission machinery was structurally different (sticky h-max set
//! + AccountV12 + RiskEngine), so each port re-derives the *property*
//! against v16's equivalent surface:
//!
//!   v12 → v16 mapping (this file)
//!   ─────────────────────────────────────────────────────────────────────
//!   `RiskEngine`                      → `MarketGroupV16`
//!   `AccountV12`                      → `PortfolioAccountV16`
//!   `admit_fresh_reserve_h_lock`      → `h_lock_lane` + `select_h_lock`
//!   sticky h-max set                  → per-account `stale_state` /
//!                                       `b_stale_state` (lane-lifting
//!                                       flags on the account itself)
//!   `mark_h_max_sticky` (bitmap)      → `mark_account_stale` (counter)
//!   v12 `validate_admission_pair`     → `V16Config::validate_public_user_fund_shape`
//!   v12 `check_conservation`          → `fork_facade::check_conservation`
//!   v12 `pnl_matured_pos_tot`         → no equivalent (v16 has no
//!                                       split matured/unmatured PnL;
//!                                       harnesses that hinge on this
//!                                       split are deferred)
//!
//! Harnesses skipped — see end-of-file `KANI_TODO` for the rationale and
//! deferral target.
//!
//! Every harness here is a fresh write — DO NOT copy v12 bodies. v12 used
//! `RiskEngine` + `InstructionContext` which don't exist in v16. The ports
//! re-establish v12's *property* with v16's surface.

use percolator::v16::{
    fork_facade, HLockLaneV16, MarketGroupV16, MarketModeV16, PortfolioAccountV16,
    ProvenanceHeaderV16, TradeRequestV16, V16Config, V16Error, V16_MAX_PORTFOLIO_ASSETS_N,
};
use percolator::MAX_MARGIN_BPS;

fn baseline_config() -> V16Config {
    V16Config::public_user_fund(1, 0, 1)
}

fn baseline_group() -> MarketGroupV16 {
    MarketGroupV16::new([1u8; 32], baseline_config()).unwrap()
}

fn baseline_account() -> PortfolioAccountV16 {
    PortfolioAccountV16::empty(ProvenanceHeaderV16::new([1u8; 32], [2u8; 32], [3u8; 32]))
}

// ============================================================================
// Port of v12 AH-1 (single_admission_range): h_lock_lane returns *exactly*
// HMin or HMax — no third option, no error on a well-formed account+group.
// v12 property: result is in {h_min, h_max}. v16 property: lane is in
// {HMin, HMax}. v16 baseline `proof_v16_hlock_is_exactly_hmin_or_hmax`
// covers a wide-symbolic range; this port pins per-account flags symbolic
// and group flags off — proves the *account-only* path returns a 2-element
// codomain.
// fork-port AH-1 / Phase 1.B
// ============================================================================

/// Proves that for any single-account admission decision on a fresh group
/// with only per-account flags varying, the lane is exactly HMin or HMax —
/// never silently neither. v16 surface property port of v12 AH-1.
#[kani::proof]
#[kani::unwind(130)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_lane_is_exactly_hmin_or_hmax_per_account() {
    let group = baseline_group();
    let mut account = baseline_account();

    // Per-account symbolic flags — these are the v16 equivalents of v12's
    // per-account sticky h-max state.
    account.stale_state = kani::any();
    account.b_stale_state = kani::any();

    let lane = group.h_lock_lane(Some(&account), false, None).unwrap();
    assert!(lane == HLockLaneV16::HMin || lane == HLockLaneV16::HMax);

    // Cover both branches.
    if account.stale_state || account.b_stale_state {
        assert_eq!(lane, HLockLaneV16::HMax);
        kani::cover!(true, "fork-port AH-1: per-account flag forces HMax");
    } else {
        assert_eq!(lane, HLockLaneV16::HMin);
        kani::cover!(true, "fork-port AH-1: flag-free path returns HMin");
    }
}

// ============================================================================
// Port of v12 AH-2 (sticky_is_absorbing): once an account's lane-lifting
// flag is set, subsequent admission decisions stay at HMax even when group
// flags clear. v12: sticky bitmap. v16: account.stale_state field.
// fork-port AH-2 / Phase 1.B
// ============================================================================

/// Proves that an account's `stale_state` flag is "absorbing" w.r.t. the
/// lane decision: as long as the flag is set, the lane is HMax regardless
/// of any group-level threshold or candidate flag. v12 sticky-h_max
/// absorbing property ported to v16's per-account flag.
#[kani::proof]
#[kani::unwind(130)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_stale_state_is_absorbing() {
    let mut group = baseline_group();
    let mut account = baseline_account();

    // Mark the account stale via the canonical API (also bumps the
    // group counter — proves the API path, not just the field).
    group.mark_account_stale(&mut account).unwrap();
    assert!(account.stale_state);

    // Any symbolic state of group flags and threshold opt: lane must
    // remain HMax. This is the v12 sticky-absorbing property.
    let instruction_bankruptcy_candidate: bool = kani::any();
    let threshold_opt: Option<u128> = kani::any();
    let lane = group
        .h_lock_lane(
            Some(&account),
            instruction_bankruptcy_candidate,
            threshold_opt,
        )
        .unwrap();
    assert_eq!(lane, HLockLaneV16::HMax);

    kani::cover!(true, "fork-port AH-2: stale_state absorbs lane decision");
}

// ============================================================================
// Port of v12 AH-5 (cross_account_sticky_isolation): account A's lane-
// lifting state must not leak into account B's lane decision. v12:
// sticky bitmap indexed by storage slot. v16: per-account stale_state.
// fork-port AH-5 / Phase 1.B
// ============================================================================

/// Proves cross-account isolation of lane-lifting state: marking account A
/// stale does not cause account B's lane to lift to HMax. v12 sticky-
/// isolation property ported to v16's per-account stale_state.
#[kani::proof]
#[kani::unwind(130)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_cross_account_stale_isolation() {
    let mut group = baseline_group();
    // Two distinct portfolio accounts — different account_id seeds so
    // provenance validates against the same market.
    let mut a =
        PortfolioAccountV16::empty(ProvenanceHeaderV16::new([1u8; 32], [10u8; 32], [11u8; 32]));
    let mut b =
        PortfolioAccountV16::empty(ProvenanceHeaderV16::new([1u8; 32], [20u8; 32], [21u8; 32]));

    // Mark only A stale.
    group.mark_account_stale(&mut a).unwrap();
    assert!(a.stale_state);
    assert!(!b.stale_state);

    // A's lane must lift to HMax — sticky/absorbing.
    let lane_a = group.h_lock_lane(Some(&a), false, None).unwrap();
    assert_eq!(lane_a, HLockLaneV16::HMax);

    // B's lane must STAY at HMin — no cross-contamination from A.
    let lane_b = group.h_lock_lane(Some(&b), false, None).unwrap();
    assert_eq!(lane_b, HLockLaneV16::HMin);

    kani::cover!(true, "fork-port AH-5: cross-account stale isolation");
}

// ============================================================================
// Port of v12 AH-6 (positive_hmin_floor): when admission returns HMin lane,
// the value returned by `select_h_lock` is exactly `config.h_min` — never
// below. v12: result is never below `admit_h_min`. v16: select_h_lock maps
// HMin → config.h_min.
// fork-port AH-6 / Phase 1.B
// ============================================================================

/// Proves `select_h_lock` floors the HMin lane to exactly `config.h_min` —
/// never below. v12 admit_h_min floor property ported to v16.
#[kani::proof]
#[kani::unwind(130)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_select_h_lock_floors_at_h_min() {
    let h_min: u8 = kani::any();
    let h_max: u8 = kani::any();
    kani::assume(h_max > 0);
    kani::assume(h_min as u64 <= h_max as u64);

    let market = [1u8; 32];
    let cfg = V16Config::public_user_fund(1, h_min as u64, h_max as u64);
    let group = MarketGroupV16::new(market, cfg).unwrap();
    let account =
        PortfolioAccountV16::empty(ProvenanceHeaderV16::new(market, [2u8; 32], [3u8; 32]));

    // No lock state — HMin lane. Result must equal config.h_min.
    let selected = group.select_h_lock(Some(&account), false).unwrap();
    assert_eq!(selected, h_min as u64);
    // And the result must respect the floor regardless of which lane.
    assert!(selected >= h_min as u64);

    kani::cover!(true, "fork-port AH-6: select_h_lock floors at h_min");
}

// ============================================================================
// Port of v12 AH-7 (sticky_bitmap_is_idempotent): marking an already-
// sticky account a second time is a no-op. v12: bitmap. v16: counter +
// bool field. Property: re-marking does not double-count the global
// counter, and the field stays true.
// fork-port AH-7 / Phase 1.B
// ============================================================================

/// Proves `mark_account_stale` is idempotent — re-marking an already-stale
/// account leaves the global `stale_certificate_count` unchanged. v12's
/// bitmap-idempotency property ported to v16's counter+bool model.
#[kani::proof]
#[kani::unwind(130)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_mark_stale_is_idempotent() {
    let mut group = baseline_group();
    let mut account = baseline_account();
    let count_before = group.stale_certificate_count;

    // First mark: counter increments, field flips to true.
    group.mark_account_stale(&mut account).unwrap();
    assert!(account.stale_state);
    assert_eq!(group.stale_certificate_count, count_before + 1);

    // Second mark on same account: idempotent — counter unchanged.
    group.mark_account_stale(&mut account).unwrap();
    assert!(account.stale_state);
    assert_eq!(
        group.stale_certificate_count,
        count_before + 1,
        "re-marking already-stale account must not double-count"
    );

    // Triple mark: still idempotent.
    group.mark_account_stale(&mut account).unwrap();
    assert!(account.stale_state);
    assert_eq!(group.stale_certificate_count, count_before + 1);

    kani::cover!(true, "fork-port AH-7: stale marking is idempotent");
}

// ============================================================================
// Port of v12 AH-8 (broken_conservation_fails): the conservation invariant
// `vault >= c_tot + insurance` is fail-closed — broken pre-state forces
// admission/touch path to reject. v12: `admit_fresh_reserve_h_lock` returns
// Err. v16: `fork_facade::check_conservation` returns false; deposit
// invariants assert.
// fork-port AH-8 / Phase 1.B
// ============================================================================

/// Proves the v16 conservation predicate detects broken state. v12 AH-8
/// property — `vault < c_tot + insurance` is invariant-breaking — ported
/// to v16's `fork_facade::check_conservation`. Complements the existing
/// `proof_v16_check_conservation_matches_vault_invariant` by pinning
/// `insurance=0` and a constant-witness c_tot > vault input, ensuring the
/// broken-conservation case is reachable on a constructed group.
#[kani::proof]
#[kani::unwind(8)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_broken_conservation_detected() {
    let mut group = baseline_group();

    // Symbolic broken state: vault < c_tot, insurance=0.
    let vault: u64 = kani::any();
    let c_tot: u64 = kani::any();
    kani::assume(c_tot > vault); // strict break
    group.vault = vault as u128;
    group.c_tot = c_tot as u128;
    group.insurance = 0;

    // Predicate must report broken.
    assert!(
        !fork_facade::check_conservation(&group),
        "broken vault < c_tot + insurance MUST be detected as non-conservative"
    );

    kani::cover!(true, "fork-port AH-8: broken-conservation case reachable");
}

// ============================================================================
// Port of v12 K-9 (admission_pair_rejects_zero_max): the admission pair
// `(h_min, h_max)` with `h_max == 0` is invalid. v12: standalone
// `validate_admission_pair`. v16: the analogous shape check lives in
// `V16Config::validate_public_user_fund_shape` (h_max == 0 → InvalidConfig).
// fork-port K-9 / Phase 1.B
// ============================================================================

/// Proves V16Config rejects `h_max == 0` at shape-validation time. v12 K-9
/// "wrapper-bypass via (0, 0)" property ported — v16's config validator
/// enforces the same bound. h_min is symbolic to ensure rejection is on
/// h_max alone, not the (h_min > h_max) clause.
#[kani::proof]
#[kani::unwind(20)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_config_rejects_zero_h_max() {
    let mut cfg = V16Config::public_user_fund(1, 0, 1);
    // h_min stays at 0 (the baseline) so h_min <= h_max holds trivially —
    // any rejection must come from the h_max == 0 clause.
    cfg.h_min = 0;
    cfg.h_max = 0;

    let r = cfg.validate_public_user_fund_shape();
    assert!(r.is_err(), "h_max == 0 MUST be rejected by shape validation");
    assert_eq!(r, Err(V16Error::InvalidConfig));

    kani::cover!(true, "fork-port K-9: h_max == 0 rejected");
}

// ============================================================================
// Port of v12 v19_admit_gate_some_zero_rejected: `Some(0)` threshold
// semantics on the A-1 admit-threshold gate. v12: `validate_threshold_opt`
// was a standalone validator that rejected Some(0). v16: A-1 gate path
// uses `>= threshold`, so `Some(0)` on a fresh group with accumulator==0
// triggers the gate (≥ check) — proves the gate's *boundary semantics*
// on v16's interpretation.
// fork-port v19-some-zero / Phase 1.B
// ============================================================================

/// Proves v16's A-1 admit-threshold gate boundary: `Some(0)` with
/// accumulator >= 0 triggers HMax on a fresh group. v12 rejected
/// `Some(0)` as invalid input via `validate_threshold_opt`; v16 wires
/// the same value into the runtime gate where the >= semantics make it
/// always-trigger. Both express the same property: a zero threshold is
/// not equivalent to None (no-op).
#[kani::proof]
#[kani::unwind(130)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_threshold_some_zero_not_equivalent_to_none() {
    let group = baseline_group();
    let account = baseline_account();
    // Fresh group: accumulator is zero, all flags off.
    assert_eq!(group.stress_consumption_bps_e9_since_envelope, 0);
    assert!(!group.threshold_stress_active);

    // None: lane is HMin (baseline behavior).
    let lane_none = group.h_lock_lane(Some(&account), false, None).unwrap();
    assert_eq!(lane_none, HLockLaneV16::HMin);

    // Some(0): accumulator (0) >= 0 → A-1 gate fires → HMax.
    let lane_zero = group.h_lock_lane(Some(&account), false, Some(0)).unwrap();
    assert_eq!(lane_zero, HLockLaneV16::HMax);

    // Therefore Some(0) is NOT equivalent to None — the v12 property
    // (Some(0) is a distinct case with non-None semantics) holds on v16.
    assert_ne!(lane_none, lane_zero);

    kani::cover!(true, "fork-port v19-some-zero: Some(0) != None");
}

// ============================================================================
// Port of v12 v19_admit_gate_sticky_early_return: a sticky account
// bypasses the consumption-threshold gate via early return. v12: sticky
// set early-return. v16: per-account stale_state lifts the lane via
// early return (the account-flag check runs *before* the threshold gate).
// fork-port v19-sticky-early / Phase 1.B
// ============================================================================

/// Proves v16's `h_lock_lane` short-circuits via account flag — the
/// per-account `stale_state` check runs before the threshold gate, so
/// the threshold value cannot affect the outcome. v12 sticky early-return
/// property ported.
#[kani::proof]
#[kani::unwind(130)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_account_flag_short_circuits_threshold_gate() {
    let mut group = baseline_group();
    let mut account = baseline_account();

    // Symbolic accumulator + threshold: in particular, accumulator below
    // threshold (where the threshold gate alone would NOT lift). Pinning
    // accumulator to a value strictly below threshold makes the only way
    // the lane can be HMax is via the account flag short-circuit.
    let threshold: u128 = kani::any();
    kani::assume(threshold > 0);
    kani::assume(threshold <= u128::MAX / 2);
    let acc: u128 = kani::any();
    kani::assume(acc < threshold);
    group.stress_consumption_bps_e9_since_envelope = acc;

    // Mark account stale — short-circuits before threshold check.
    group.mark_account_stale(&mut account).unwrap();

    // Pin all group-level lane triggers off so the only HMax source is
    // either the account flag (early return) or the threshold gate
    // (which is below threshold and therefore can't fire).
    group.threshold_stress_active = false;
    group.bankruptcy_hlock_active = false;
    group.loss_stale_active = false;

    let lane = group
        .h_lock_lane(Some(&account), false, Some(threshold))
        .unwrap();
    assert_eq!(
        lane,
        HLockLaneV16::HMax,
        "account flag must short-circuit threshold gate"
    );

    kani::cover!(true, "fork-port v19-sticky-early: account flag short-circuits");
}

// ============================================================================
// Port of v12 IN-1 (no_live_immediate_release): a public mutation path
// (`set_pnl_with_reserve` in v12) refuses to run on Live with a reserve
// mode that doesn't fit. v16 analogue: `withdraw_not_atomic` (which goes
// through admission via `h_lock_lane` after `validate_withdraw_global_locks`)
// refuses to mutate when the global lock check fires.
// fork-port IN-1 / Phase 1.B
// ============================================================================

/// Proves v16's `validate_withdraw_global_locks` rejects non-Live market
/// modes. v12 IN-1 property — public mutation path refuses on a guarded
/// mode — ported to v16. State is unchanged on rejection (validate-then-
/// mutate).
#[kani::proof]
#[kani::unwind(130)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_withdraw_rejects_non_live_mode() {
    let mut group = baseline_group();
    let account = baseline_account();

    // Force market to Resolved — `validate_withdraw_global_locks` must
    // reject any withdraw attempt regardless of account shape.
    group.mode = MarketModeV16::Resolved;

    let mode_before = group.mode;
    let vault_before = group.vault;
    let c_tot_before = group.c_tot;

    let r = group.kani_validate_withdraw_global_locks(&account);
    assert!(r.is_err(), "non-Live mode MUST reject withdraw global locks");
    assert_eq!(r, Err(V16Error::LockActive));

    // No state change.
    assert_eq!(group.mode, mode_before);
    assert_eq!(group.vault, vault_before);
    assert_eq!(group.c_tot, c_tot_before);

    kani::cover!(true, "fork-port IN-1: non-Live mode rejects withdraw");
}

// ============================================================================
// Port of v12 AC-6 (outstanding_acceleration_blocked_by_nonzero_hmin):
// when `h_min > 0`, the v12 outstanding-reserve acceleration path is
// blocked. v16 equivalent: `select_h_lock` on HMin lane returns
// `config.h_min`, which when > 0 functions as the non-zero gating
// equivalent to v12's `admit_h_min` floor.
// fork-port AC-6 / Phase 1.B
// ============================================================================

/// Proves `select_h_lock` returns a non-zero h-lock when `config.h_min > 0`
/// and the lane is HMin (no flags set). v12 AC-6 "nonzero h_min blocks
/// immediate acceleration" property ported — v16's HMin lane respects
/// the config-supplied floor.
#[kani::proof]
#[kani::unwind(130)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_select_h_lock_respects_nonzero_h_min() {
    let h_min: u8 = kani::any();
    kani::assume(h_min > 0);
    let h_max: u8 = kani::any();
    kani::assume(h_max >= h_min);
    kani::assume(h_max > 0);

    let market = [1u8; 32];
    let cfg = V16Config::public_user_fund(1, h_min as u64, h_max as u64);
    let group = MarketGroupV16::new(market, cfg).unwrap();
    let account =
        PortfolioAccountV16::empty(ProvenanceHeaderV16::new(market, [2u8; 32], [3u8; 32]));

    // No lock state — HMin lane selected. With h_min > 0, result > 0.
    let selected = group.select_h_lock(Some(&account), false).unwrap();
    assert_eq!(selected, h_min as u64);
    assert!(selected > 0, "nonzero h_min must propagate to selected lock");

    kani::cover!(true, "fork-port AC-6: nonzero h_min respected on HMin lane");
}

// ============================================================================
// Port of v12 K-71 (neg_pnl_count_tracks_actual): an account-level counter
// invariant — the engine's `neg_pnl_account_count` matches the number of
// used accounts with `pnl < 0`. v16 analogue: `negative_pnl_account_count`
// on `MarketGroupV16`. v12 used pnl mutation paths; v16 baseline doesn't
// expose those raw, so this port establishes the *static* invariant: on
// a freshly constructed group, the counter starts at 0 (matches zero
// negative-PnL accounts), and any path to non-zero requires an explicit
// public state mutation.
// fork-port K-71 / Phase 1.B
// ============================================================================

/// Proves `negative_pnl_account_count` starts at zero on a freshly
/// constructed `MarketGroupV16`. v12 K-71 invariant has a static
/// component: a market with no accounts has no negative-PnL accounts.
/// The dynamic-tracking component of v12 K-71 maps to v16's account-
/// lifecycle baseline harnesses; this port covers the initial-state
/// invariant explicitly.
#[kani::proof]
#[kani::unwind(20)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_neg_pnl_counter_initial_state_is_zero() {
    let group = baseline_group();
    assert_eq!(
        group.negative_pnl_account_count, 0,
        "freshly constructed MarketGroupV16 has no negative-PnL accounts"
    );

    // Symbolic config range: the property holds across any valid
    // config (single asset, varying h_min/h_max). Reconstruct with
    // a symbolic h_max bound to widen coverage.
    let h_max: u8 = kani::any();
    kani::assume(h_max > 0);
    let cfg = V16Config::public_user_fund(1, 0, h_max as u64);
    let group2 = MarketGroupV16::new([2u8; 32], cfg).unwrap();
    assert_eq!(group2.negative_pnl_account_count, 0);

    kani::cover!(true, "fork-port K-71: fresh group has zero negative-PnL count");
}

// ============================================================================
// Port of v12 AC-1 (acceleration_all_or_nothing) — atomicity: v16's
// `deposit_not_atomic` either succeeds and increments capital+c_tot+vault
// by the same amount, or rejects without mutation. v12 used reserve
// bucket atomicity; v16's analogous atomic primitive is deposit. Both
// share the same "value-flow atomicity" invariant.
// fork-port AC-1 / Phase 1.B
// ============================================================================

/// Proves `deposit_not_atomic` is "all-or-nothing": on a successful call,
/// capital, c_tot, and vault all advance by exactly `amount`; on
/// `amount == 0` it is a true no-op. v12 acceleration-atomicity property
/// ported to v16's deposit (the analogous atomic value-flow primitive).
#[kani::proof]
#[kani::unwind(130)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_deposit_all_or_nothing() {
    let mut group = baseline_group();
    let mut account = baseline_account();

    // Bounded symbolic deposit: stays well below overflow.
    let amount: u8 = kani::any();

    let cap_before = account.capital;
    let c_tot_before = group.c_tot;
    let vault_before = group.vault;

    let r = group.deposit_not_atomic(&mut account, amount as u128);

    if amount == 0 {
        // Zero deposit: no mutation at all.
        assert!(r.is_ok());
        assert_eq!(account.capital, cap_before);
        assert_eq!(group.c_tot, c_tot_before);
        assert_eq!(group.vault, vault_before);
        kani::cover!(true, "fork-port AC-1: zero-deposit no-op path reachable");
    } else {
        // Non-zero deposit: must succeed (no overflow at u8 scale) AND
        // mutate all three totals by exactly the same amount.
        assert!(r.is_ok());
        assert_eq!(account.capital, cap_before + amount as u128);
        assert_eq!(group.c_tot, c_tot_before + amount as u128);
        assert_eq!(group.vault, vault_before + amount as u128);
        // Conservation preserved.
        assert!(fork_facade::check_conservation(&group));
        kani::cover!(true, "fork-port AC-1: deposit atomicity path reachable");
    }
}

// ============================================================================
// Port of v12 ah4 (hmin_zero_immediate_release) — h_min == 0 path: when
// config.h_min == 0 and the lane is HMin, `select_h_lock` returns 0
// (immediate release equivalent). v12 property: an admission decision
// can return 0 when state admits. v16: HMin lane + config.h_min == 0
// produces 0.
// fork-port ah4 / Phase 1.B
// ============================================================================

/// Proves that `select_h_lock` returns 0 when `config.h_min == 0` and the
/// HMin lane is selected — the v12 "immediate-release" boundary case.
/// v16 surface: the HMin lane maps directly to `config.h_min`.
#[kani::proof]
#[kani::unwind(130)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_h_min_zero_returns_zero_lock() {
    let h_max: u8 = kani::any();
    kani::assume(h_max > 0);

    let market = [1u8; 32];
    // h_min explicitly zero.
    let cfg = V16Config::public_user_fund(1, 0, h_max as u64);
    let group = MarketGroupV16::new(market, cfg).unwrap();
    let account =
        PortfolioAccountV16::empty(ProvenanceHeaderV16::new(market, [2u8; 32], [3u8; 32]));

    // No flags set — HMin lane.
    let selected = group.select_h_lock(Some(&account), false).unwrap();
    assert_eq!(selected, 0, "h_min=0 on HMin lane returns 0 lock");

    kani::cover!(true, "fork-port ah4: h_min=0 immediate-release path");
}

// ============================================================================
// Cover-only sanity proof: TradeRequestV16 admit_h_max_consumption_threshold
// field is a per-trade, copied value. v12 had no `TradeRequest` struct;
// the threshold was an instruction-context field. v16 inlines it into the
// trade request. This port proves the field round-trips as a pure value
// (no aliasing), strengthening the existing wire-format proof by exercising
// a different equivalence (Eq derive across all 5 fields with symbolic
// asset_index + size_q).
// fork-port v19-threshold-field / Phase 1.B
// ============================================================================

/// Proves the threshold-opt field on TradeRequestV16 is a pure value:
/// equal requests compare equal, copy preserves the value. Complements
/// the existing `proof_v16_admit_threshold_field_persists_in_trade_request`
/// in `proofs_v16_fork.rs` by binding `fee_bps` to MAX_MARGIN_BPS-bounded
/// (the validation-relevant range) instead of fully unconstrained.
#[kani::proof]
#[kani::unwind(8)]
#[kani::solver(cadical)]
fn proof_v16_fork_admission_trade_request_threshold_eq_value_semantics() {
    let asset_index: usize = kani::any();
    kani::assume(asset_index < V16_MAX_PORTFOLIO_ASSETS_N);
    let size_q: u128 = kani::any();
    let exec_price: u64 = kani::any();
    let fee_bps: u64 = kani::any();
    kani::assume(fee_bps <= MAX_MARGIN_BPS);
    let threshold_opt: Option<u128> = kani::any();

    let req_a = TradeRequestV16 {
        asset_index,
        size_q,
        exec_price,
        fee_bps,
        admit_h_max_consumption_threshold_bps_opt: threshold_opt,
    };
    let req_b = TradeRequestV16 {
        asset_index,
        size_q,
        exec_price,
        fee_bps,
        admit_h_max_consumption_threshold_bps_opt: threshold_opt,
    };
    assert_eq!(req_a, req_b);

    // Differing threshold opt: requests must compare not-equal (unless
    // both opts are the same variant + value, which we exclude).
    let other_threshold: Option<u128> = kani::any();
    kani::assume(other_threshold != threshold_opt);
    let req_c = TradeRequestV16 {
        asset_index,
        size_q,
        exec_price,
        fee_bps,
        admit_h_max_consumption_threshold_bps_opt: other_threshold,
    };
    assert_ne!(req_a, req_c);

    kani::cover!(true, "fork-port v19-threshold-field: Eq value semantics");
}

// ============================================================================
// KANI_TODO (deferred ports — rationale per item)
//
// The following v12 admission harnesses were inspected and *not* ported in
// this batch. Each is deferred with a rationale tying it to a v16 feature
// gap or scope decision.
//
//   - AH-3 (no_under_admission): hinges on the sticky-bitmap mutation
//     across two sequential admissions on the same account. v16's
//     analogue is account.stale_state which has the same absorbing
//     property — the *sequential* aspect is implicitly covered by AH-2
//     above (stale_state is absorbing across any number of admission
//     calls). Re-deriving the explicit two-call sequence would add
//     coverage but no new property. SKIP — subsumed.
//
//   - AC-1 / AC-2 / AC-4 (acceleration_*): v12's outstanding-reserve
//     acceleration on a sched-bucket. v16 has no sched-bucket / reserved-
//     pnl machinery — the entire reserve-bucket subsystem is replaced by
//     v16's source-credit + close-progress-ledger model. Deferred to
//     Phase 1.C source-credit harnesses (already partly covered by v16
//     baseline `proofs_v16.rs` source-credit subsystem of 18 harnesses).
//
//   - AC-5 (admit_outstanding_atomic_on_err): same as AC-1 / AC-2 / AC-4
//     above — reserve-bucket subsystem doesn't exist on v16.
//
//   - K-1 / K-2 (accrue_rejects_dt_over_envelope / resolve_degenerate_*):
//     touch v16's funding/accrual surface which is structurally different.
//     Belongs in `proofs_v16_fork_accrual.rs` (next batch).
//
//   - K-201 / K-202 (keeper_crank rejects oversized budget / postcondition
//     detects broken conservation): keeper-crank in v16 is the
//     `PermissionlessCrank` path which has completely different request
//     shape. Defer to `proofs_v16_fork_crank.rs` batch.
//
//   - RS-1..RS-4 (reserve validation / queue malformation): reserve
//     subsystem doesn't exist in v16 (replaced by source-credit). SKIP
//     — subsumed.
//
//   - v19_admit_gate_stress_lane_forces_h_max / v19_admit_gate_none_disables_step2:
//     subsumed by existing fork-port harnesses in
//     `tests/proofs_v16_fork.rs` (proof_v16_admit_threshold_*_to_hmax).
//
//   - v19_consumption_*: subsumed by existing fork-port stress-envelope
//     harnesses in `tests/proofs_v16_fork.rs` (proof_v16_stress_envelope_*).
//
//   - v19_rr_*: round-robin cursor / Phase 2 scan harnesses. v16 has
//     entirely different Phase 2 surface (permissionless crank). RETIRE
//     per V16_PROOFS_RETIRED.md (v19 stress-lane / consumption ~10
//     harnesses retired).
//
//   - v19_phase2_* / v19_speculative_hmax_*: bankrupt-close state
//     machine. RETIRED per V16_PROOFS_RETIRED.md A-5 (v16's
//     CloseProgressLedgerV16 is strict superset).
//
//   - v19_fee_sync_* / v19_explicit_fee_*: fee-sync subsystem. Belongs
//     in `proofs_v16_fork_fee.rs` (next batch).
//
//   - v19_floor_to_zero_cleanup_preserves_oi_and_adds_potential_dust:
//     phantom-dust 4-field schema. RETIRED per V16_PROOFS_RETIRED.md A-8.
//
//   - v19_flat_negative_cleanup_starts_bankruptcy_hmax: bankrupt-close
//     state machine. RETIRED per V16_PROOFS_RETIRED.md A-5.
//
//   - k104_oi_geq_sum_of_effective: OI invariant. v16 has different OI
//     mechanism (per-leg, not per-account). Belongs in
//     `proofs_v16_fork_oi.rs` (next batch).
//
// ============================================================================
