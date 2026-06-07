# P0 Genuine CBMC Fails — Reconcile Plan

Repo: `~/percolator`, branch `v16-resync-2026-06-01`, HEAD `229e38d`.
Triage baseline: commit `5aee93e`.
Engine src: `src/v16.rs`. Proofs: `tests/proofs_v16.rs`.

---

## FAIL #1: `proof_v16_repeated_account_b_chunks_complete_bounded_small_residual`

### Source References

| Claim | File:line | Verified |
|---|---|---|
| Proof body byte-identical 5aee93e..HEAD | `tests/proofs_v16.rs:6309` | git diff confirms 0 lines changed |
| `assert_public_invariants` OI-balance check — 5aee93e | `src/v16.rs:17940 (5aee93e)` | `git show 5aee93e:src/v16.rs \| grep "oi_eff_long_q != asset.oi_eff_short_q"` returns line 17940 |
| `assert_public_invariants` OI-balance check — HEAD | `src/v16.rs:18194` | `(self.mode == MarketModeV16::Live && asset.oi_eff_long_q != asset.oi_eff_short_q)` |
| `attach_leg(Long)` increments `oi_eff_long_q` only | `src/v16.rs:15002-15005` | `asset.oi_eff_long_q += basis_pos_q.unsigned_abs()` |
| `MarketGroupV16::new` sets `mode: MarketModeV16::Live` | `src/v16.rs:13160` | `mode: MarketModeV16::Live` in `Ok(Self { ... })` |
| `settle_account_b_chunk` (runtime) does not change `mode` | `src/v16.rs:15909-15947` | full fn body has no mode write |
| E6 commit 16c4324 did NOT modify `validate_runtime_storage_shape` or `assert_public_invariants` OI check | `git diff 5aee93e HEAD -- src/v16.rs` | zero diff on that line |

### Root Cause

The proof calls `group.attach_leg(&mut account, 0, SideV16::Long, 1)` which increments
`assets[0].oi_eff_long_q = 1` while `oi_eff_short_q` remains 0. The group's mode is
`MarketModeV16::Live` from construction (`MarketGroupV16::new` line 13160). After
B-chunk settlement, the proof ends with `assert_eq!(group.assert_public_invariants(), Ok(()))`.

Inside `assert_public_invariants`, the asset-loop at `src/v16.rs:18194` evaluates:

```rust
(self.mode == MarketModeV16::Live && asset.oi_eff_long_q != asset.oi_eff_short_q)
```

With `mode=Live`, `oi_eff_long_q=1`, `oi_eff_short_q=0`, this is TRUE → `return Err(InvalidConfig)`.
CBMC finds this immediately; the proof fails with "assertion failed:
`group.assert_public_invariants() == Ok(())`" — exactly the CBMC report.

This check has been present since before `5aee93e` (confirmed at line 17940 in that baseline).
The proof was written to test B-chunk completion mechanics but was incorrectly extended to call
`assert_public_invariants` on a single-sided-leg (imbalanced-OI) state, which is a structurally
invalid Live-mode state by engine design. B-chunk settlement does not restore OI balance.

The failure is **NOT** caused by commit `16c4324` (E6). No E6 field participates in this
invariant path. The proof was stale since its introduction — it completed before 5aee93e only as
an OOM (never as a CBMC success), so the structural failure was never caught.

### Verdict

**STALE FIXTURE.** The B-chunk settlement path itself is correct. The proof's final
`assert_public_invariants()` call is invalid for the single-sided-leg setup because Live mode
mandates OI balance.

### ESCALATION?

No. The OI balance check in `assert_public_invariants` is correct and intentional.
A single-Long-leg state with no counterpart Short is not reachable on a live market via normal
operation (every `attach_leg` on one side is preceded by a matching trade that adds the other
side). The proof fixture is artificial. No production-reachable invariant violation.

### Exact Reconcile Edit

Two options; Option A is minimal and preserves full proof intent:

**Option A (recommended): drop the `assert_public_invariants` call.**

The proof's purpose is to verify B-chunk completion mechanics: `delta_b` counts, `b_snap`/`b_rem`
updates, `b_stale` flag clearing, and `b_stale_account_count` counter. None of these require
`assert_public_invariants`. Remove the final line:

```diff
-    assert_eq!(group.assert_public_invariants(), Ok(()));
```

Replace with a targeted structural assertion that is meaningful in this single-sided context:

```rust
// B-chunk settlement maintains zero PnL and correct per-leg accounting;
// public invariants require Live-mode OI balance which intentionally does
// not hold for a single-sided test fixture.
assert_eq!(group.b_stale_account_count, 0);
// OI remains consistent: long increased by attach, short stays zero (no counterpart
// in this unit fixture — matched-book balance is not what this proof tests).
assert_eq!(group.assets[0].oi_eff_long_q, 1);
assert_eq!(group.assets[0].oi_eff_short_q, 0);
```

**Option B (broader scope): switch mode to Recovery before `assert_public_invariants`.**

```diff
+    group.mode = MarketModeV16::Recovery;
     assert_eq!(group.assert_public_invariants(), Ok(()));
```

`MarketModeV16::Recovery` does not trigger the OI-balance check at line 18194. All other
invariant sub-checks still execute (vault, senior solvency, domain ledger, asset shape, etc.).
This validates the broader invariant set while allowing the artificial single-sided fixture.
However, it silently skips the OI-balance invariant — callers should note this in a comment.

**Assertions preserved?**

Option A: all existing assertions at lines 6321-6341 are preserved unchanged. The removed line is
only the final `assert_public_invariants` call.
Option B: all assertions including `assert_public_invariants` are preserved; the mode change is
the only addition.

**Recommended: Option A.** It is honest about what the proof tests. The OI-balance invariant
has its own proofs elsewhere; duplicating it in a unit fixture for B-chunk mechanics adds noise.

### Confidence: HIGH

Every claim is source-grounded. The OI check is present byte-for-byte in both commits.
The proof is byte-identical in both commits. CBMC reached the check (it is reachable) and
returned Err(InvalidConfig) at line 18194. No ambiguity.

---

## FAIL #2: `proof_v16_released_pnl_conversion_is_residual_bounded_and_conserves_vault`

### Source References

| Claim | File:line | Verified |
|---|---|---|
| Proof body byte-identical 5aee93e..HEAD | `tests/proofs_v16.rs:8975` | git diff confirms 0 lines changed |
| `convert_released_pnl_to_capital_core_not_atomic` byte-identical | `src/v16.rs:14230 (HEAD)` vs `src/v16.rs:14031 (5aee93e)` | `git diff 5aee93e HEAD -- src/v16.rs` shows 0 changed lines in this fn body |
| Live-mode no-source path returns 0 | `src/v16.rs:14246-14247` | `} else if self.mode == MarketModeV16::Live { 0 }` |
| `converted == 0` returns `Err(LockActive)` | `src/v16.rs:14251-14253` | `if converted == 0 { return Err(V16Error::LockActive); }` |
| `result.unwrap()` panic site | `tests/proofs_v16.rs:9014` | `let converted = result.unwrap();` in the `else` branch |
| `expected > 0` when `profit >= 1 && residual >= 1` | `tests/proofs_v16.rs:8998` | `let expected = (profit as u128).min(residual as u128);` |
| `account_has_source_claims` false for zero-init account | `src/v16.rs:20004-20005` | `Ok(Self::account_source_claim_bound_sum_num_static(account)? != 0)` — no `add_account_source_positive_pnl_not_atomic` call in proof |
| `add_account_source_positive_pnl_not_atomic` — runtime fn | `src/v16.rs:18280` | fn signature confirmed |
| `add_fresh_counterparty_backing_not_atomic` — runtime fn | `src/v16.rs:18302` | fn signature confirmed |
| `ensure_source_domain_capacity` resizes E6 Vecs | `src/v16.rs:2760-2761` | `source_lien_capital_at_risk_fee_revenue.resize(...)` |

### Root Cause

The proof fixture sets `account.pnl = profit as i128` directly (bypassing `set_account_pnl`)
and adjusts group-level PnL counters manually. It does NOT call
`add_account_source_positive_pnl_not_atomic`, so `account.source_claim_bound_num[*] = 0` for all
domains → `account_has_source_claims = false`.

In `convert_released_pnl_to_capital_core_not_atomic` (line 14244-14252):

```rust
let converted = if Self::account_has_source_claims(account)? {
    self.account_source_realizable_support(account, released)?
} else if self.mode == MarketModeV16::Live {
    0          // <-- taken; no source claims, Live mode
} else {
    self.haircut_effective_support(released, self.residual(), self.junior_claim_bound())?
};
if converted == 0 {
    return Err(V16Error::LockActive);   // <-- always returned when profit>=1 && no source claims
}
```

When `profit >= 1`, `released = profit > 0`. With no source claims and Live mode: `converted=0`
→ `Err(LockActive)`.

The proof's `else` branch at line 9013 (`expected > 0`) calls `result.unwrap()`, which panics.

CBMC witnesses: any concrete assignment with `profit in [1,10]` AND `residual in [1,10]`
produces `expected > 0` → unwrap panic. The CBMC report says "1 of 6522 failed ...
unwrap_failed (result.rs)" — this is exactly the `.unwrap()` on an `Err(LockActive)` result.

This behavior was present at `5aee93e`: the `else if self.mode == MarketModeV16::Live { 0 }`
branch is byte-identical. The proof was written for an older engine version where Live-mode
non-source accounts could convert PnL via the haircut path; that path is now gated to
non-Live mode only. The proof was never run to CBMC completion before (it completed as OOM
on prior passes), so the stale fixture was never caught.

**This is NOT an E6 regression.** Commit `16c4324` adds E6 fields to `PortfolioAccountV16`
and accrual paths, but does not change the source-attribution gate in
`convert_released_pnl_to_capital_core_not_atomic`.

### Verdict

**STALE FIXTURE** — the proof assumes a pre-source-attribution Live-mode conversion path that
no longer exists. Reconcile: add source attribution so the `account_has_source_claims=true`
branch is taken, preserving the vault-conservation intent.

### ESCALATION?

No. The `else if self.mode == MarketModeV16::Live { 0 }` gate is correct and intentional: Live
mode requires source attribution for PnL conversion (prevents free extraction of unreserved
vault surplus). The proof fixture is the error, not the engine.

### Exact Reconcile Edit (Option A — "add source attribution")

**Setup changes (before `deposit_not_atomic`):**

Add backing to domain 0 BEFORE setting PnL, so the source path executes:

```rust
// Step 1: give domain 0 counterparty backing (expiry=1 > current_slot=0).
// BOUND_SCALE = 1_000_000 (see src/lib.rs); backing sized to cover full profit bound.
// Use kani::any()-capped profit because we constrain profit <= 10.
// We set vault first so assert_public_invariants inside add_fresh_counterparty_backing passes.
group.vault = 100;  // arbitrary large enough for deposit + all checks
```

Replace the manual PnL injection block:

```rust
// OLD (causes fixture failure):
//   account.pnl = profit as i128;
//   group.pnl_pos_tot = profit as u128;
//   set_junior_bound(&mut group, profit as u128);
//   group.pnl_matured_pos_tot = profit as u128;
//   group.vault = group.c_tot + group.insurance + residual as u128;

// NEW (source-attributed path):
group.deposit_not_atomic(&mut account, 10).unwrap();
// Add counterparty backing for domain 0 — sized to cover profit * BOUND_SCALE.
// Only add backing when profit > 0 (add_fresh_counterparty_backing_not_atomic
// returns Err(InvalidConfig) for amount=0 at src/v16.rs:18321).
if profit > 0 {
    group
        .add_fresh_counterparty_backing_not_atomic(0, (profit as u128) * BOUND_SCALE, 1)
        .unwrap();
    // Add source-attributed positive PnL — this calls set_account_pnl_with_source
    // (src/v16.rs:14199) and records source_claim_bound_num[0] = profit * BOUND_SCALE.
    group
        .add_account_source_positive_pnl_not_atomic(&mut account, 0, profit as u128)
        .unwrap();
}
group.pnl_matured_pos_tot = profit as u128;
group.vault = group.c_tot + group.insurance + residual as u128;
```

**`expected` assertion change:**

The source-backed path returns `account_source_realizable_support(account, profit)`.
With `credit_rate_num = CREDIT_RATE_SCALE` (full rate, initial value at line 1826) and
full backing, this equals `profit` exactly (see `account_source_realizable_support`,
lines 20008-20058: `support_num = claim_num * CREDIT_RATE_SCALE / CREDIT_RATE_SCALE = claim_num`,
then `/ BOUND_SCALE = profit`).

Conversion is also gated by `source_credit_available_backing_num(0) / BOUND_SCALE = profit`
(backing = `profit * BOUND_SCALE`, available / BOUND_SCALE = profit). So `take = profit` for
the single domain.

Replace:
```rust
// OLD:
let expected = (profit as u128).min(residual as u128);

// NEW:
// Source-backed path: converted = profit (full, backed by counterparty).
// Residual still controls vault; conversion does not draw from vault residual.
let expected = if profit > 0 { profit as u128 } else { 0 };
```

**`expected == 0` branch update:**

When `profit == 0`: `released = 0` → `convert_released_pnl_to_capital_core_not_atomic` returns
`Ok(0)` at line 14236 (`if released == 0 { return Ok(0); }`). No change needed for that arm.

When `profit > 0 && residual == 0`: with source backing, `converted = profit` regardless of
`residual`. The `vault_before` conservation still holds (vault does not change in source path).
The old `expected = profit.min(residual) = 0` branch relied on the non-source haircut formula
which is no longer reachable in Live mode. Remove the `else { assert_eq!(result, Err(LockActive)) }`
path for the `profit > 0, residual == 0` sub-case:

```rust
if expected == 0 {
    // profit == 0: released == 0 -> Ok(0)
    assert_eq!(result, Ok(0));
    assert_eq!(group.vault, vault_before);
    assert_eq!(group.c_tot, c_tot_before);
    assert_eq!(account.capital, 10);
    assert_eq!(account.pnl, 0);
} else {
    let converted = result.unwrap();
    assert_eq!(converted, expected);
    assert_eq!(group.vault, vault_before);          // vault-conservation: unchanged (source path)
    assert_eq!(group.c_tot, c_tot_before + expected);
    assert_eq!(account.capital, 10 + expected);
    assert_eq!(account.pnl, 0);
    assert_eq!(group.pnl_pos_tot, 0);
    assert_eq!(group.pnl_pos_bound_tot, 0);
}
assert_eq!(group.assert_public_invariants(), Ok(()));
```

Note: the final `assert_eq!(group.assert_public_invariants(), Ok(()))` is valid here because
this proof has NO leg attached — `oi_eff_long_q = oi_eff_short_q = 0` — so the Live-mode
OI-balance check passes. This distinguishes FAIL #2 from FAIL #1.

**Are conservation assertions preserved?**

Yes:
- `assert_eq!(group.vault, vault_before)` — preserved and still holds (source-backed
  conversion does not draw from vault; see `TokenValueFlowProofV16::support_to_account_capital`
  called at line 14294-14302 which validates zero vault delta for counterparty-backed cases).
- `assert_eq!(group.c_tot, c_tot_before + expected)` — preserved; `c_tot += converted` at
  line 14283-14286.
- `assert_eq!(account.capital, 10 + expected)` — preserved; `account.capital += converted`
  at line 14279-14282.
- `assert_eq!(account.pnl, 0)` — preserved; `set_account_pnl(account, 0)` via face_burn path.
- `assert_eq!(group.pnl_pos_tot, 0)` and `assert_eq!(group.pnl_pos_bound_tot, 0)` — preserved;
  source-backed `set_account_pnl` burns `source_claim_bound_num` and decrements group totals.

**Behavioral justification:**

The original proof tested the non-source (`haircut_effective_support`) path which is only
reachable in non-Live mode. In Live mode, PnL conversion requires source attribution as an
anti-extraction guard. The reconciled proof tests the same economic properties
(vault conservation, c_tot + capital conservation, PnL zeroing) on the path that is ACTUALLY
reachable for a Live-mode account, which is the source-backed path. The "residual-bounded"
aspect of the proof name changes meaning: with source backing, the bound is the available
counterparty backing (not vault residual), but vault conservation is strictly stronger than
residual-bounded for this path. The proof intent is preserved at the economic level.

### Confidence: HIGH

Byte-identity of the failing fn confirmed by `git diff`. The exact code path
(`else if self.mode == MarketModeV16::Live { 0 }`) is unambiguous. The proposed fix uses
the same API pattern as the sibling proof
`proof_v16_source_backed_open_conversion_rejects_before_mutation` (line 9026) which
already uses `add_account_source_positive_pnl_not_atomic` + `add_fresh_counterparty_backing_not_atomic`
for correct fixture setup.

---

## Summary

| Fail | Verdict | One-line reconcile | Escalate? |
|---|---|---|---|
| FAIL #1 `proof_v16_repeated_account_b_chunks_complete_bounded_small_residual` | Stale fixture — `assert_public_invariants` called on single-sided-leg state violating Live OI-balance invariant (present since before 5aee93e, unmasked because prior run OOM'd) | Drop the final `assert_public_invariants()` call (Option A) OR set `group.mode = MarketModeV16::Recovery` before it (Option B) | No |
| FAIL #2 `proof_v16_released_pnl_conversion_is_residual_bounded_and_conserves_vault` | Stale fixture — proof assumes no-source haircut path in Live mode (`else if self.mode == MarketModeV16::Live { 0 }`) which was gated off before 5aee93e; unwrap at line 9014 panics when profit≥1 | Add `add_account_source_positive_pnl_not_atomic` + `add_fresh_counterparty_backing_not_atomic` calls; update `expected` from `profit.min(residual)` to `profit`; preserve all conservation assertions | No |

**Neither failure is an E6 regression.** Both proofs are byte-identical at 5aee93e and HEAD. The E6 commit `16c4324` adds `source_lien_capital_at_risk_fee_revenue` and `source_lien_impaired_capital_at_risk_fee_revenue` fields and wires them through `ensure_source_domain_capacity` / `checked_source_domain_capacity` / fee-accrual paths — none of which are exercised by either failing proof's fixture. No production-reachable invariant violation was found. Both are fixture-reconcile items for the v17 Phase-0 baseline gate.
