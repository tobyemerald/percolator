# Percolator v17 — FULL CONVERGENCE Master Brief (autonomous, highest-quality, finish-it-all)

You are the execution lead for the **v17 convergence**: fully align the fork with toly's CURRENT
architecture, re-express every fork feature onto it, redesign stake, rebuild verification, and produce a
**cutover-ready, fully-validated bundle**. Run this **end-to-end, autonomously, to the highest standard**.
Take as long as it takes. Do NOT pester the orchestrator with questions you can resolve from the standing
decisions + the principles below. Escalate ONLY a genuine fund-critical fork with no in-policy resolution.
**Do NOT deploy** — mainnet cutover is human-gated; you produce the freeze-ready, audited bundle.

---

## STANDING DECISIONS (locked — do not re-litigate, do not ask)
1. **DROP runtime-vec-api.** Converge to toly's single zero-copy/sparse path. The dual-mirror discipline ends —
   every path is single. Rebuild the **Kani harness + LiteSVM + surfpool** onto the zero-copy path. The
   zero-copy/ViewMut half of the existing E1–E6 work survives; the runtime mirrors are deleted.
2. **Adopt toly's full current architecture:** sparse portfolio, the O(1)-in-N DoS refactor, the auth overhaul
   (`marketauth` + per-asset `asset_admin`), batch-trade API, per-asset cold-storage keys, and the two fund
   fixes (`051e268`, `c120fce`).
3. **Redesign stake.** Toly's auth overhaul deletes `cfg.insurance_authority`/`insurance_operator`. Rebind the
   stake-vault-PDA insurance custody onto the new per-asset/`marketauth` model (via `UpdateAssetAuthority`
   tag 65), **preserving the security property: no admin key can drain insurance; the PDA controls inflow +
   terminal-reclaim.** If you cannot preserve that property in-policy, THAT is an escalation.
4. **Keep our matcher superset** (content-ahead of toly, incl. the vAMM insurance-fee-routing/skew-spread —
   our guardrailed-vAMM substrate) unless toly's matcher gained something we need; reconcile, don't regress.
5. **Match toly exactly** for adopted code; verify by CONTENT not git hash. Re-express our fork features onto
   the new architecture preserving their economic/security properties.

## OPERATING DISCIPLINE
- **Autonomy:** decide everything you can from the standing decisions + "match toly's restructured arch,
  re-express our features onto it preserving their properties, verify everything." Document each non-trivial
  decision + rationale in the evidence pack. Escalate ONLY: (a) a fund-critical fork the standing decisions
  don't cover and that "match-toly + preserve-our-property" can't resolve, or (b) a fork feature that
  genuinely cannot be re-expressed and would have to be dropped (a feature-loss call). Otherwise proceed.
- **Continuous run, no mid-run stops** except a true blocker. The orchestrator reviews at phase boundaries and
  at the end via your artifacts — make that review possible.
- **Durable + resumable:** work in a dedicated worktree/branch off the Phase-0 rollback tag, separate target
  dir. Commit **per unit, audit-grade** (no squash, no `--no-verify`). Maintain a living evidence pack
  `~/wrapper-engine-deep-audit/V17_CONVERGENCE_EVIDENCE.md`. **Tag per phase** (`v17-phaseN-*`) so a context
  reset resumes from the brief + evidence pack + last phase tag. On resume: read this brief, the evidence
  pack, and `git log`/tags to reconstruct state, then continue.
- **No deploy.** No touching mainnet. Produce the cutover bundle + freeze; deploy is the human's call.

## QUALITY BAR (operationalized — not adjectives)
- **Collision-matrix first** (Phase 1): every fork feature × every toly refactor, verified at source, each with
  a re-expression plan + collision resolution. No code before the matrix is complete.
- **Byte-faithfulness:** every adopted toly change diffed against upstream (content, not hash).
- **Anti-hollow tests:** operative (RED-before / GREEN-after), exact error codes + wrong-reason guards, no
  vacuous proofs, no `#[ignore]`/`cfg(any())` masking, no value-flipping to go green.
- **Formal verification:** rebuild the FULL Kani harness on the single zero-copy path (incl. re-expressed
  fork-feature proofs). The gate is **literal 0-fail across every harness.** HARD LESSON from the last cycle:
  **timeouts MASK counterexamples** — run at a non-thrashing parallelism and **run every heavy proof to actual
  completion** (solo/`-j2`–`-j3`, generous `--harness-timeout`); never accept "timed out ⇒ probably passes."
- **Dual validation:** LiteSVM + the surfpool 3B mainnet-fork differential (incl. the 3B.4 self-differential),
  both rebuilt against the converged single-path binaries.
- **Adversarial security review** of every fund-critical path (auth, custody, settlement, source-credit,
  LP-vault, stake, oracle) — the percolator-security discipline; conservation/solvency invariants hold end to
  end.

---

## FORK-FEATURE INVENTORY (everything that MUST survive the convergence — verify each at source in Phase 1)
**Whole programs toly lacks:** Position-NFT (`percolator-nft`: Token-2022 NFT, B-3 transfer hook, per-market
NftRegistry); Stake (`percolator-stake`: StakePool, Bind/Rotate insurance custody, two-step admin).
**Engine features** (`v16.rs`): A-1 admit-threshold; A-4 ~26 pub-lifts; A-6 stress-envelope (+ its fields);
A-9 fee mutator; A-10 max_price_move; engine-side `lp_vault` module (NAV/share-math).
**Wrapper features** (`v16_program.rs`): LP Vault (tags 65–71); B-3 NFT ownership transfer (tags 72–73);
B-11 oracle cap; the LP-vault source-credit watermark mirror (5ebd136 class).
**Matcher** (`percolator-match`): the content-ahead superset incl. vAMM insurance-fee-routing + skew-spread.
**Off-chain:** keeper, SDK, indexer, frontend, API, mobile (Phase 6 regen).

## TOLY REFACTOR SET (what we're adopting)
**FROZEN v17 TARGET (re-pinned 2026-06-07, before Phase 2 — DO NOT chase further; converge to these exact SHAs):**
engine **`5c72af3`** (`5c72af32506ca10f7e923e2be4f824ea559dec6d`) / wrapper **`0f87dcb`** (`0f87dcb4099e56b308462215e2b7c377536a5590`) / matcher keep-ours. Superseded the original `051e268`/`70294cb` pin (toly shipped ~150 more commits: a security/fund-fix cluster + cross-market-isolation + matcher-auth hardening + authority-burn rejection + a proof-widening sweep — all now IN the target). Re-pin valid because the build hadn't started; FROZEN for the program duration now.
Original-pin commit families (still the architecture being adopted):
- Sparse portfolio (945f2db cluster, built on f26a6f0).
- O(1)-in-N DoS refactor: engine `4bd3d79` (eliminate market-size scans) + `b01c8e0` (tighten wrapper API);
  wrapper `479a84f` (route mutations through aggregate-maintaining APIs; 5 new total fields). Closes O(N)
  trade-CU DoS.
- Auth overhaul: `792256b` (collapse to `marketauth`, delete insurance/operator/backing/mark authorities,
  WRAPPER_CONFIG_LEN 624→432) + `dba87a9` (unify asset-0) + `f64b7ee` (per-asset cold-storage, tag 65).
- Batch trade: engine `5094322`/`2366af0`/`3c4bdc3`/`a180483`/`05e6e98`; wrapper `0fa25bd`.
- Fund fixes: `051e268` (release impaired insurance liens in terminal wind-down), `c120fce` (source-claim
  persistence + stale trade CU).

## KNOWN COLLISIONS (seed the matrix — find the rest in Phase 1)
- **Auth overhaul → Stake** (deletes the bound fields → redesign, Standing Decision 3); likely also **NFT B-3
  authority checks** and **LP-vault authority gating**.
- **O(1) aggregates → LP Vault:** our LP vault writes vault/insurance directly → will underflow toly's new
  totals (his `EngineCounterUnderflow` class) → must route through the aggregate-maintaining APIs.
- **Sparse portfolio → LP-vault source-credit usage + the E6 genesis counters + wrapper deserializers.**
- **Drop runtime-vec → LP-vault NAV host construction + the wrapper deserializers** (rebuild on zero-copy).

---

## PHASES (each ends with: commits + evidence-pack section + a phase tag)

**Phase 0 — Close the current v16 snapshot as the rollback baseline (FAST — do NOT grind Kani).**
REVISED 2026-06-07 (hardware + drop-runtime-vec reality): do NOT gate this baseline on a literal full-420 Kani
run. ~397/420 already PASS; the engine diff vs `5aee93e` is **additive-only (no regressions)**; the remaining
~23 are heavy unwind-130 proofs that **OOM on 64GB even solo** and/or exercise the **runtime-vec path we DELETE
in Phase 2** — verifying them here has near-zero forward value. So: (a) commit the genuine fixture-fails that
are investigated-as-not-bugs (byte-identity vs `5aee93e`, assertions faithful — `phase1/p0_genuine_fails_reconcile.md`);
(b) bank the ~397 PASS; (c) DOCUMENT the ~23 deferred proofs (OOM/runtime-vec) → resolved at the Phase-5
zero-copy gate, with rationale, in the evidence pack; (d) run the FAST real checks: `cargo build` both, wrapper
`cargo test`, surfpool 3B green; (e) tag `v16-{engine,wrapper}-sync-rolling/2026-06-03-catchup` +
`v16-surfpool-phase3b-resync`. **This tag = the v17 branch point + rollback.** The baseline is a fallback, not
the ship target — the LITERAL full Kani 0-fail gate runs ONCE, at Phase 5, on the rebuilt zero-copy harness.

**Phase 1 — Convergence recon + design (NO code).** Produce, verified at source: (a) the complete **fork-feature ×
toly-refactor collision matrix** (every feature above × sparse/O(1)/auth/runtime-drop, with the resolution for each);
(b) the **drop-runtime-vec plan** (what's deleted, the single zero-copy end-state, harness rebuild plan);
(c) the **stake-redesign design doc** (new binding via tag 65 preserving PDA-custody, with the threat model);
(d) the sparse/auth/O(1)/batch adoption plans; (e) the **layout/ABI delta** (runtime-vec drop, sparse, config
624→432, O(1) aggregates, +32B E6, batch) → the Phase-6 client-regen spec. Tag `v17-phase1-design`.

**Phase 2 — Engine convergence.** Drop runtime-vec; adopt sparse + O(1) aggregates + fund fixes + batch API;
re-express A-1/A-4/A-6/A-9/A-10 + engine `lp_vault` onto the single zero-copy path; rebuild the Kani harness on
zero-copy. Per-unit operative proofs. Tag `v17-phase2-engine`.

**Phase 3 — Wrapper convergence.** Adopt the auth overhaul (marketauth + per-asset asset_admin), O(1) integration,
sparse conversion, batch trade, cold-storage keys; re-express LP-Vault (tags 65–71, routed through the aggregate
APIs), NFT-B3 (tags 72–73, under the new auth), B-11, the watermark mirror. Operative regression tests with exact
codes. Tag `v17-phase3-wrapper`.

**Phase 4 — Stake redesign.** Rebind insurance custody onto the new auth model per the Phase-1 design; preserve
PDA-custody; assembled-LiteSVM e2e proving no-admin-drain + no-lockout (rotate/re-bind) under the new model. Update
NFT/B-3 authority interactions if the matrix flagged them. Tag `v17-phase4-stake`.

**Phase 5 — Validation gate.** Consolidate; both single-path engine builds + wrapper build-sbf clean. **Full Kani
audit on the rebuilt zero-copy harness → literal 0-fail (no timeout-masking).** Rebuild LiteSVM + the surfpool 3B
mainnet-fork differential (incl. 3B.4 self-differential) on the converged binaries. **Adversarial security review of
all fund-critical paths.** Tag `v17-phase5-validated`.

**Phase 6 — Client regen.** From the Phase-1 ABI/layout delta, regenerate/update SDK (3.0.0), keeper, indexer,
frontend, mobile to the new layouts/auth/batch surface (incl. the pre-existing carries: funding-wire drop,
InvalidLeg→HiddenLeg, canonical-ATA, fee-on-mark). Their test suites green against the converged programs. Tag
`v17-phase6-clients`.

**Phase 7 — Freeze (NO deploy).** Assemble the cutover bundle, freeze, and produce the **audit-ready package** +
the final report. **Do not deploy** — hand off for human authorization. Tag `v17-freeze-candidate`.

## DEFINITION OF DONE
All toly refactors adopted; runtime-vec fully removed; every fork feature re-expressed onto the single zero-copy
path with its collision resolved; stake redesigned with PDA-custody preserved + proven; full Kani audit literal
0-fail on the rebuilt harness (heavies completed, not timed out); LiteSVM + surfpool 3B green; security review
clean; clients regenerated + green; cutover bundle frozen and audit-ready. **Not deployed.**

## FINAL REPORT (one, at the end)
The evidence pack + per-phase tags; the collision matrix with every resolution; the stake-redesign threat model +
proof; the full Kani 0-fail tally (every harness named, none dropped/ignored/timed-out-unresolved); the surfpool
self-differential result; the security-review findings; the ABI/layout delta + client-regen status; and the
remaining human-gated step (cutover authorization). Escalations (if any) listed with options + your recommendation.
