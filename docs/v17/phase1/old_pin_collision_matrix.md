# v17 Convergence — Fork-Feature × Toly-Refactor COLLISION MATRIX

> Phase-1 (a) deliverable. Built during the Phase-0 compute-wait from the source-verified recon files
> (`toly_engine_refactor_recon.md`, `toly_wrapper_refactor_recon.md`, `fork_engine_feature_inventory.md`,
> `fork_wrapper_programs_inventory.md`). Resolutions follow the STANDING DECISIONS: drop runtime-vec → single
> zero-copy/sparse path; adopt toly's restructured arch byte-faithfully; re-express fork features preserving
> their economic/security properties; keep matcher superset.
>
> **STATUS: RE-VERIFIED AGAINST FROZEN (2026-06-07).** Re-confirmed at source against frozen engine
> `5c72af3` (`~/toly-engine-frozen/src/v16.rs`) and frozen wrapper `0f87dcb` (`~/toly-wrapper-frozen/
> src/v16_program.rs`). The frozen pins are the CURRENT tips of toly origin/master in both repos. Three
> NEW frozen commits landed AFTER this draft was first built (matrix file mtime 2026-06-04 15:34) and
> CHANGE several resolutions — see the **§G FROZEN RE-VERIFICATION DELTA** appended at the end. Read §G
> FIRST; the body tables below are correct except where §G overrides them.

## Legend
- **SURVIVES-CLEAN**: fork feature lives on the zero-copy/ViewMut path already; no collision; zero/low v17 work.
- **REBUILD**: feature's logic survives but its runtime-vec mirror / Kani harness / host construction must be
  rewritten against the single zero-copy path.
- **RE-EXPRESS**: feature must be re-implemented onto the new arch (new auth model, new tag, new aggregate API).
- **COLLISION**: a hard conflict (tag, deleted field, ABI) requiring an explicit resolution.

---

## A. ENGINE FORK FEATURES × ENGINE TOLY REFACTORS

| Fork feature (loc) | drop-runtime-vec (f26a6f0) | sparse (945f2db) | O(1) aggs (4bd3d79/b01c8e0) | batch (5094322..) | fund fixes (051e268/c120fce) |
|---|---|---|---|---|---|
| **A-1 admit-threshold** — `TradeRequestV16` field v16.rs:2884; gates h_lock_lane ViewMut:9208 + runtime:15174 | TradeRequestV16 NOT cfg-gated → **SURVIVES**; only runtime h_lock_lane twin drops (ViewMut twin keeps gate) | field is on portfolio header path; sparse touches source_domains not legs → no collision | trade-path gate; O(1) refactor routes trade CU through aggregates → re-validate gate still fires pre-mutation | batch settles-once then N applies; admit gate must run per-leg inside batch → **RE-EXPRESS** into batch loop | none |
| **A-4 pub-lifts** — 8 in-place lifts (active_leg_slot_for_asset:13360, haircut_effective_support:20315, account_haircut_equity:20342/53, resolved_positive_payout_ready:20941, account_no_positive_credit_equity:21688, ensure_initial_margin:21720, ensure_no_positive_credit_initial_margin:21734) + 18-fn `fork_facade` mod:22172 | **REBUILD**: the 8 in-place lifts are on fns that survive (re-apply `pub` on the ViewMut twins); the entire `fork_facade` mod is cfg(runtime-vec-api)-gated → **rewrite against ViewMut types** | fork_facade signatures take runtime types → rewrite arg types to ViewMut/header refs | some lifted fns (account_no_positive_credit_equity, ensure_initial_margin) are aggregate consumers → ensure they read the new cached aggregates not a rescan | n/a | n/a |
| **A-6 stress-envelope** (+4 fields) — MarketGroupV16 runtime:2836-2847 MIRRORED in MarketGroupV16HeaderAccount POD:4398-4407; writers ViewMut:5232 + runtime:15244 | **REBUILD**: ViewMut path + the HeaderAccount POD fields SURVIVE intact (zero logic lost); drop only the runtime MarketGroupV16 mirror writer | 4 fields are in the market header POD, not source_domains → no sparse collision | **LAYOUT INTERACTION**: A-6's 4 header fields coexist with O(1)'s +5 aggregate fields in MarketGroupV16HeaderAccount — must reconcile the combined field order/offsets (both add to the same POD) | n/a | n/a |
| **A-9 fee mutator** — `FeePolicyUpdateV16` v16.rs:2904 (NOT cfg-gated); ViewMut mutator:4540; runtime mutator:14816; 3 Kani proofs in proofs_v16_fork.rs | **REBUILD**: struct + ViewMut mutator SURVIVE; drop runtime mutator:14816; rewrite the 3 fork Kani proofs onto ViewMut | no collision (fee policy is market-header scoped) | fee mutator interacts with insurance-flow aggregates (b01c8e0 added 6 pub insurance fns) → route fee→insurance through the aggregate-maintaining API | n/a | n/a |
| **A-10 max_price_move** — guard v16.rs:1657-1658 in V16Config::validate_public_user_fund_shape (NOT cfg-gated) | **SURVIVES-CLEAN** (zero-copy-compatible today; zero v17 work) | none | none | none | none |
| **engine lp_vault mod** — v16.rs:22482 (NOT cfg-gated; takes only primitive u128/u16 inputs, no MarketGroupV16/PortfolioAccountV16) | **SURVIVES-CLEAN** (fully zero-copy-clean; no v17 changes) | none (no source_domains touch) | none (pure NAV/share math on primitives) | none | none |
| **E6 genesis capital-at-risk counter** — DUAL-MIRROR: Vec<u128> in PortfolioAccountV16 SoA:2678-79 + V16PodU128 in PortfolioSourceDomainV16Account:12846-47 | **REBUILD**: the ViewMut/on-chain (PortfolioSourceDomainV16Account) field SURVIVES; drop the runtime Vec mirror; rewrite the E6 Kani proofs onto ViewMut | **SPARSE INTERACTION**: source domains become inline `[PortfolioSourceDomainV16Account; 32]` → the E6 fields ride along inside that inline array (good — they're already in the per-domain POD); verify the +32B is reflected in the new inline layout | E6 fields are per-source-domain; O(1) source_claim_bound_total aggregate must NOT double-count the capital-at-risk revenue | n/a | 051e268 terminal release touches source/insurance liens — confirm E6 crystallization still fires on the new terminal-release path |

## B. WRAPPER / SATELLITE-PROGRAM FORK FEATURES × WRAPPER TOLY REFACTORS

| Fork feature (loc) | auth overhaul (792256b/dba87a9/f64b7ee) | O(1) wrapper (479a84f) | sparse | batch (0fa25bd) | tag space |
|---|---|---|---|---|---|
| **LP Vault tags 65-71** (v16_program.rs:5324-6397) | LP vault writes vault/insurance/backing directly → with 479a84f's new totals, direct writes UNDERFLOW the aggregates (EngineCounterUnderflow class) → **RE-EXPRESS through the aggregate-maintaining APIs** (b01c8e0's 6 new insurance-flow fns). Auth gating: CreateLpVault admin-gate → marketauth | **RE-EXPRESS**: Deposit/Redeem/Crank mutate backing+insurance → route through aggregate APIs (479a84f) | LP vault reads source-credit + backing-domain ledger → confirm sparse source_domain inline layout | LP vault has no trade legs → batch n/a | **TAG-65 COLLISION** (see §C) |
| **LP-vault dual withdraw-gate** (5ebd136 class, v16_program.rs:6102-6133) | gate calls `expected_source_credit_rate_num(source_after)` — re-validate after the domain-auth/sparse redesign | reads source-credit watermark — confirm against O(1) aggregates | reads source_after on sparse domain | n/a | no tag |
| **B-3 NFT transfer** tags 72-73 (v16_program.rs:6514-6713) | tag 72 TransferPortfolioOwnership is CPI-from-NFT signed by mint_auth PDA; tag 73 SetNftProgramId admin→marketauth. NFT registry derivation unaffected by auth overhaul. **RE-EXPRESS** admin gate → marketauth | no direct aggregate write | portfolio.owner + provenance dual-write on sparse header | n/a | tags 72/73 — verify vs toly (no known toly claim there yet) |
| **B-11 oracle staleness cap** — MAX_ORACLE_STALENESS_SECS=86400, enforced v16_program.rs:1057,1179 | enforcement sits in UpdateConfig/per-asset-profile validation — **RE-APPLY** atop the restructured config + AssetOracleProfileV16 (LEN 368→400) | none | none | none | no tag (constant + guard) |
| **percolator-nft** (Token-2022, B-3 hook, NftRegistry) | NftRegistry owned by wrapper, derived under wrapper program-id; allowlist cpi_v16.rs:72-75 hardcodes wrapper id. Auth overhaul doesn't touch this UNLESS program-id changes at cutover → re-register via tag 73 | n/a | n/a | n/a | NFT tags 0-6 (own program) |
| **percolator-stake** (Bind/Rotate 19/20, two-step admin 5/6) | **STAKE-BINDING COLLISION** (see §C) — 792256b deletes cfg.insurance_authority + AUTHORITY_INSURANCE=2; Bind/Rotate CPI old 34B wire hard-fails. **REDESIGN onto per-asset tag-65 UpdateAssetAuthority** (Standing Decision 3) | n/a | n/a | n/a | stake tags own program; CPIs target wrapper tag 32→65 |
| **percolator-match superset** (vAMM insurance-fee-routing vamm.rs:488, skew-spread vamm.rs:534, asset_index echo, ABI v3) | logic-only, no wrapper wire dep → **KEEP** (Standing Decision 4); verify MatcherReturn offset 56 (asset_index echo) still read correctly by the restructured wrapper | n/a | n/a | matcher participates in batch trade? confirm batch path calls matcher per-leg unchanged | MATCHER_ABI v3 |

---

## C. TAG-ALLOCATION RECONCILIATION (hard collisions)

Toly v17 wrapper tag claims (confirmed at source): 32 `UpdateAuthority{new_pubkey}` (simplified), **65
`UpdateAssetAuthority{asset_index,kind,new_pubkey}`** (f64b7ee), **66 `BatchTradeNoCpi`**, **67
`BatchTradeCpi`** (0fa25bd).

Fork tag claims: **65 CreateLpVault, 66 DepositToLpVault, 67 RequestRedeem, 68 ExecuteRedemption,
69 CrankFees, 70 SetPaused, 71 CloseLpVault**, 72 TransferPortfolioOwnership, 73 SetNftProgramId.

**COLLISIONS:** fork 65/66/67 (LP-vault) directly collide with toly 65 (UpdateAssetAuthority) / 66
(BatchTradeNoCpi) / 67 (BatchTradeCpi). Fork 68-71 may or may not collide depending on toly's full tag map
(verify the toly dispatch range 68+ in Phase 2).

**RESOLUTION (proposed):** Adopt toly's tags 65/66/67 byte-faithfully (Standing Decision 5). **Renumber the
fork LP-vault block to an unclaimed high range, e.g. 80-86** (Create 80, Deposit 81, RequestRedeem 82,
ExecuteRedemption 83, CrankFees 84, SetPaused 85, Close 86). Keep B-3 NFT at 72/73 IF toly hasn't claimed
them (verify); else shift to 87/88. Update all clients (SDK/keeper/indexer/frontend/mobile) + the NFT
program's CPI to the wrapper (it CPIs tag 72 — keep or shift in lockstep). This is a Phase-3 wrapper +
Phase-6 client change; recorded in the ABI delta (§E).

---

## D. DROP-RUNTIME-VEC PLAN (Phase-1 (b) — single zero-copy end-state)

**What is DELETED** (the runtime-vec-api mirror, per f26a6f0 + fork coupling):
- 28 `cfg(runtime-vec-api)` gates in the engine.
- The runtime `MarketGroupV16` struct's ~84 public methods that mirror the ViewMut twins (44 `*_not_atomic`
  verbs each have a ViewMut twin → keep ViewMut, drop runtime).
- The runtime `PortfolioAccountV16` SoA Vec fields (incl. the E6 Vec mirror) — replaced by the sparse inline
  `[PortfolioSourceDomainV16Account; 32]` zero-copy layout.
- The 18-fn `fork_facade` mod (v16.rs:22172) — rewrite the fork-only helpers against ViewMut types.
- The runtime mirror writers for A-6 (15244), A-9 (14816), A-1 (15174), E6 crystallization runtime path.

**What SURVIVES (the zero-copy/ViewMut half of E1–E6):** every ViewMut twin, the HeaderAccount POD fields
(A-6 stress-envelope, E6 per-domain field), TradeRequestV16/FeePolicyUpdateV16/V16Config (not cfg-gated),
A-10 guard, the engine lp_vault mod (primitive-only).

**Kani harness rebuild:** ~50+ harnesses currently construct runtime `MarketGroupV16`/`PortfolioAccountV16`
+ poke Vec fields. Rebuild them to construct the zero-copy `*Account` POD + ViewMut and exercise the ViewMut
verbs. This is the bulk of Phase-2 proof work. The A-9 fork proofs (3, proofs_v16_fork.rs) + E6 proofs +
the fork_facade-dependent proofs are the priority rewrites. (Note: the Phase-0 heavy-but-correct proofs that
pass on runtime today will be re-authored on ViewMut — their assertions carry over.)

## E. ABI / LAYOUT DELTA (Phase-1 (e) → Phase-6 client-regen spec)

| Change | Old | New | Source | Client impact |
|---|---|---|---|---|
| WrapperConfigV16 length | 624 | **432** | 792256b | SDK config decoder rewrite; drop 7 authority fields, add marketauth |
| AssetOracleProfileV16 length | 368 | **400** | f64b7ee | +asset_admin[32]; per-asset insurance_authority lives here now |
| MarketGroupV16HeaderAccount | base | **+72B** (5 aggregates) | 4bd3d79 | header decoder: add backing_provider_earnings_total, source_claim_bound_total_num, source_insurance_credit_reserved_total_atoms, insurance_domain_budget_remaining_total, resolved_payout_blocker_count (after pnl_matured_pos_tot) |
| PortfolioAccountV16Account.source_domains | external slice | **inline `[…;32]`** | 945f2db | account decoder: source_domains now fixed inline array; PortfolioV16View holds only `header:&…` |
| E6 per-domain | +32B (dual-mirror) | inline in sparse domain POD | 16c4324 + 945f2db | account decoder: 2 new u128 per source-domain (capital_at_risk + impaired twin) |
| UpdateAuthority tag 32 wire | `{kind,new_pubkey}` 34B | `{new_pubkey}` 33B | 792256b | SDK/stake CPI: drop kind byte |
| NEW tag 65 UpdateAssetAuthority | — | `{asset_index:u16,kind:u8,new_pubkey:[u8;32]}` | f64b7ee | SDK add; stake redesign target |
| NEW tags 66/67 batch | — | BatchTradeNoCpi/Cpi | 0fa25bd | SDK add batch builders |
| LP-vault tags 65-71 | 65-71 | **renumber 80-86** (proposed) | this matrix §C | SDK/keeper/frontend/mobile LP-vault tag update |
| Pre-existing carries (from prior cycle) | — | — | — | funding-wire drop, InvalidLeg→HiddenLeg, canonical-ATA, fee-on-mark |

## F. STAKE-REDESIGN SKETCH (Phase-1 (c) — detail in stake_redesign.md, Phase-4 impl)

**Problem:** 792256b deletes `cfg.insurance_authority` + `AUTHORITY_INSURANCE=2`; stake Bind/Rotate (tags
19/20) CPI the old `UpdateAuthority(tag32, kind=2, pubkey)` 34B wire → hard-fails decode. The bound custody
slot moves to PER-ASSET `AssetOracleProfileV16.insurance_authority`, rotated via tag 65 `UpdateAssetAuthority`.

**Redesign (preserves the property "no admin key can drain insurance; PDA controls inflow + terminal-reclaim"):**
- Stake `BindInsuranceAuthority` (tag 19) → CPI `UpdateAssetAuthority{asset_index, kind=insurance, new_pubkey
  = vault_auth_pda}` (toly tag 65) instead of the old tag-32 wire. Binds the per-asset insurance authority
  of the stake-pool's asset to the stake `vault_auth` PDA `["vault_auth", pool_pda]`.
- Stake `RotateInsuranceAuthority` (tag 20) → same tag-65 CPI with the PDA signing as the CURRENT per-asset
  insurance authority (escape: rotate to admin, redeploy, re-bind). Preserves the no-lockout property.
- `TopUpInsurance` (tag 9 in wrapper) gating already moved (dba87a9) to read
  `domain_authorities_from_view(&group,&cfg,asset_index)?.insurance_authority` (per-asset) → the PDA bound
  via tag 65 is the gate; no admin key sits on that path.

  **THREAT MODEL — RESOLVED (NOT an escalation): PRESERVABLE-WITH-GUARD.** Full analysis +
  source quotes in `phase1/stake_threat_model.md`. Findings (toly origin/main @5081495):
  - tag 65 `kind=insurance` is rotatable by (A) `asset_admin` WITHOUT the current authority's consent
    (`v16_program.rs:9033`), or (B) the current authority self-rotating when `admin_signed==false` (:9044).
  - `asset_admin` is BOOTSTRAPPED to `config.marketauth` at asset-0 genesis (:1311) → so by default the
    marketauth holder CAN displace a bound PDA insurance authority (the drain path EXISTS).
  - BUT authorities are burn-only-no-revival: burning `asset_admin`→0 makes `admin_signed` always false →
    only path B (PDA self-rotation) survives → marketauth can no longer rotate it away (`live_authority_
    matches` rejects zero unconditionally :10422; f64b7ee "a burned admin can't be revived").
  - SECONDARY drain surface: tag 57 `handle_withdraw_insurance_domain` shutdown-drain bypass gated on
    `marketauth` as `insurance_operator` (:8300-8308) — NOT gated by insurance_authority → must also close.
  **STAKE REDESIGN (Phase-4) operational sequence to preserve "no-admin-drain":**
  (1) Bind: stake tag 19 → CPI toly tag 65 `kind=insurance, new_pubkey=vault_auth_pda` (was tag 32).
  (2) Burn `asset_admin`→0 (tag 65 `kind=admin, new_pubkey=0`, signed by current asset_admin) — NON-NEGOTIABLE,
      locks out path A permanently.
  (3) Rotate/burn `insurance_operator` off marketauth — closes the tag-57 shutdown-drain.
  (4) Rotate (stake tag 20) → tag 65 with the PDA self-signing (path B) — the no-lockout escape survives.
  ALTERNATIVE (cleaner, needs a wrapper change we'd add in Phase 3): in `handle_update_asset_authority`,
  when `kind==insurance && admin_signed`, ALSO require the current insurance_authority holder to sign (PDA
  consent) — then step (2) burn becomes unnecessary. Toly hasn't added this guard; decide in Phase 3/4 whether
  to add it (cleaner, diverges from toly by one guard) or use the pure-operational burn sequence (matches toly
  byte-for-byte). Either preserves the property → in-policy, no escalation.

---

## OPEN ITEMS to close before Phase-2 code (finalize in Phase 1 proper)
1. ~~Verify toly's FULL wrapper tag map ≥68~~ **RESOLVED (§G).** Frozen toly claims tags 65/66/67/**68**/**69**;
   LP-vault renumber target **80-86 is fully unclaimed** (70-88 all free at frozen). Fork tags 68/69 ALSO
   collide (not just 65/66/67 as drafted).
2. ~~Stake no-admin-drain threat~~ **RE-RESOLVED + TIGHTENED (§G).** Frozen `087a404` now (a) REJECTS burning
   any non-admin asset authority to 0 (`v16_program.rs:8840`) — the old "burn insurance_operator" step is
   ILLEGAL; rotate to a live key instead; (b) REJECTS burning `marketauth` to 0 (`:8774`); (c) the tag-57
   marketauth shutdown-drain bypass is `asset_index != 0`-gated (`:8257`) → UNREACHABLE for the stake asset-0
   pool, so that secondary drain surface is already closed for asset 0. Net: stake redesign is SIMPLER for
   asset-0 but the burn-sequence steps change.
3. Confirm batch-trade calls the matcher per-leg unchanged (matcher superset KEEP, Standing Decision 4).
   Frozen `MATCHER_ABI_VERSION = 3` (`:69`) == fork ABI; asset_index echo at bytes[56..64] read+validated
   (`:3865/:3886`) — KEEP holds. NEW: frozen `8690121` adds `derive_matcher_authorization` `matcher-auth`
   PDA the wrapper requires (`:8690121 diff`) — adopt the PDA binding wrapper-side (§G).
4. Reconcile A-6 (+4) and O(1) (+5) fields' combined order in MarketGroupV16HeaderAccount (§A). CONFIRMED
   real at frozen: O(1) aggregates occupy `MarketGroupV16HeaderAccount:4669-4674` (5 fields) with a hard
   rescan-consistency check (`v16.rs:5183-5191`); A-6's 4 header fields must append AFTER them.
5. ~~Confirm B-3 NFT tags 72/73 don't collide~~ **RESOLVED (§G).** Frozen uses neither 72 nor 73; NFT program
   CPIs wrapper tag 72 (`percolator-nft/src/transfer_hook.rs:77`). KEEP NFT-B3 at 72/73 — no renumber needed.

---

## G. FROZEN RE-VERIFICATION DELTA (2026-06-07) — what CHANGED vs the draft

Re-verified every row at source against frozen engine `5c72af3` + frozen wrapper `0f87dcb`. The draft was
built against an OLDER toly snapshot. Material deltas:

**G-0 Frozen pins ARE the toly tips.** `5c72af3` (engine) and `0f87dcb` (wrapper) are the current
origin/master heads. All draft-cited refactor commits verified present and dated ≤2026-06-04:
`f26a6f0`(runtime-vec drop) `945f2db`(sparse) `4bd3d79`+`b01c8e0`(O(1)) `5094322`/`0fa25bd`(batch)
`792256b`/`dba87a9`/`f64b7ee`(auth) `051e268`/`c120fce`(fund). `16c4324`(E6) NOT a toly commit (it is a
FORK commit — draft mis-attributed it; the E6 fields are fork-only, see G-5).

**G-1 NEW toly wrapper commits the draft missed (all in frozen `0f87dcb`):**
- `cc91b07` "Gate live insurance withdraw on exposed oracle lag" (2026-06-04 23:45) — NEW oracle-lag runtime
  gate. **OVERLAPS B-11** (see G-2).
- `8690121` "Bind matcher authorization to canonical PDA" — adds `derive_matcher_authorization(["matcher-auth",
  market, maker_account, maker_owner, matcher_program, matcher_context], program_id)` required by the wrapper's
  matcher CPI path. **Affects matcher KEEP** — adopt the PDA binding wrapper-side (matcher logic still KEEP).
- `087a404` "Reject market authority burns" — **changes the stake threat model** (see G-3).
- The remaining ~60 new commits are test-coverage / engine bumps (no new collision surface).

**G-2 B-11 oracle cap × cc91b07 oracle-lag gate — DISTINCT mechanisms, partial intent-overlap.**
- B-11 (fork) = a CONFIG bound: `config/profile.max_staleness_secs > MAX_ORACLE_STALENESS_SECS(86_400)` is
  rejected at the hybrid-oracle validation sites (`percolator-prog/src/v16_program.rs:1057,1179` etc.). Hook
  point SURVIVES at frozen — `max_staleness_secs` still in the frozen config/profile (`:712,:747`) and the
  validation has `== 0` (`:1163,:1284`) but LACKS the `> MAX` upper cap. B-11 = re-insert the upper cap. SEV
  mechanical RE-APPLY.
- cc91b07 (toly) = a RUNTIME gate on the LIVE INSURANCE WITHDRAW drain path only: rejects when an asset has
  exposed OI and `raw_oracle_target_price != effective_price` (`reject_exposed_target_effective_lag_view`,
  wrapper `:4805`/`:8547` in the cc91b07 diff). It does NOT bound config staleness; B-11 does NOT gate the
  live-withdraw path. **They are complementary, not redundant.** Adopt cc91b07 byte-faithfully (it is in
  frozen); re-apply B-11's config-cap on top. No conflict. Note the LP-vault redeem (re-expressed in §B) must
  ALSO honor cc91b07's lag gate since it is a same-class insurance/backing drain.

**G-3 STAKE REDESIGN — burn semantics TIGHTENED (supersedes §F steps 2-3).** Frozen `handle_update_asset_
authority` (`v16_program.rs:8791`):
- Co-sign of incoming key unchanged (`:8806`).
- **NEW HARD RULE `:8840`: `new_pubkey == 0 && kind != ASSET_AUTH_ADMIN` → InvalidInstruction.** Only the
  cold-storage `asset_admin` may be burned to 0; insurance / insurance_operator / backing / oracle authorities
  can be ROTATED but NEVER burned. Comment `:8824-8826` documents it.
- Frozen `handle_update_authority` (tag 32, marketauth) `:8774`: **burning marketauth to 0 is now rejected
  unconditionally** (draft's mode-gated burn-guard removed by `087a404`).
- Tag-57 shutdown-drain bypass is `asset_index != 0 && shutdown_drain && marketauth` (`:8257-8259`) → for the
  stake asset-0 pool the marketauth insurance-drain bypass is UNREACHABLE (already closed for asset 0).
- ASSET_AUTH kinds at frozen `:4661-4665`: ADMIN=0, INSURANCE=1, INSURANCE_OPERATOR=2, BACKING_BUCKET=3,
  ORACLE=4. (Stake CPI's `AUTHORITY_INSURANCE=2` in `percolator-stake/src/cpi.rs:33` is the OLD tag-32 kind
  selector — now WRONG: per-asset insurance is kind=1 via tag 65.)
**Revised stake no-admin-drain sequence (Phase-4):** (1) Bind: stake tag 19 → CPI toly **tag 65**
`UpdateAssetAuthority{asset_index, kind=ASSET_AUTH_INSURANCE(1), new_pubkey=vault_auth_pda}` (was tag-32-with-
kind 34B → now 35B `{u16,u8,[32]}`). (2) Burn `asset_admin`→0 (tag 65 kind=ADMIN(0)) — STILL LEGAL, locks
out admin-rotation path A. (3) **DROP the old "burn insurance_operator" step — illegal now**; for asset 0 the
marketauth operator-bypass is already unreachable (`:8257`), so no action needed; for assets 1..N rotate the
operator to a controlled live key if used. (4) Rotate (stake tag 20) → tag 65 kind=INSURANCE with the PDA
self-signing (path B) — no-lockout escape survives. The "cleaner one-guard consent" alternative still applies.

**G-4 STAKE CPI WIRE COLLISION — confirmed at source.** `percolator-stake/src/cpi.rs:129-133` emits
`tag(32)+kind(2)+pubkey(32)=34B` and targets the DELETED `cfg.insurance_authority`. Frozen tag 32 takes
`{new_pubkey}` only (33B, no kind, marketauth-only). HARD decode/semantic fail → REDESIGN onto tag 65 (G-3).
Both `cpi_bind_insurance_authority` and the rotate twin (`cpi.rs:152+`) must be rewritten.

**G-5 ENGINE FORK FEATURES — re-confirmed at frozen engine `5c72af3`:**
- A-1 admit-threshold: frozen `TradeRequestV16:3162-3169` has NO `admit_h_max_consumption_threshold_bps_opt`
  (and `size_q` is `i128` at frozen vs `u128` in fork — extra ABI delta). **Still a genuine fork ADD.**
- A-4 fork_facade(`fork.v16.rs:22172`) + the 8 pub-lifts: frozen has NEITHER. **Still fork ADDs**; rebuild
  against ViewMut.
- A-6 stress-envelope (4 fields: `threshold_stress_active` + 3 envelope fields, fork `v16.rs:2836-2847` /
  POD `:4398-4407`): frozen has NONE. **Still fork ADD; layout-interaction with O(1) aggregates CONFIRMED**
  (frozen `MarketGroupV16HeaderAccount:4669-4674` + rescan check `:5183-5191`).
- A-9 fee mutator `FeePolicyUpdateV16` (4 fields, fork `:2904-2909`): frozen has the 4 fields only on tag-0
  InitMarket (`v16_program.rs:3082-3086`); toly's POST-init fee mutators (tags 37/49/51/55) are NARROWER
  (cranker_share / trade_fee_base only) → A-9's post-init 4-field mutation is NOT obviated. **Fork ADD; new
  tag in renumber range.** (Partial overlap: tag-55 `trade_fee_base_bps` vs A-9 `max_trading_fee_bps`.)
- A-10 max_price_move: **PARTIALLY CONVERGED.** Frozen engine NOW has `max_price_move_bps_per_slot` + the
  `== 0` lower guard (`v16.rs:1462,:1950`). Only the fork's UPPER cap `> MAX_MARGIN_BPS` (`fork v16.rs:1658`)
  is fork-only → re-insert one clause. SEV mechanical (downgraded from the draft's "SURVIVES-CLEAN").
- engine `lp_vault` mod (fork `:22482`): primitive-only (u128/u16 ledger counters), uses
  `wide_mul_div_floor_u128`; frozen lacks it but it has NO runtime-vec/sparse dependency. **SURVIVES-CLEAN.**
- E6 capital-at-risk: **CONVERGED — supersedes the §A E6 row.** Fork commit `16c4324` is a PORT of toly's
  own `a57a408` "Add genesis capital-at-risk fee-revenue counter (per-source-domain)". Frozen engine HAS the
  two POD fields `source_lien_capital_at_risk_fee_revenue` + `source_lien_impaired_capital_at_risk_fee_revenue`
  as `V16PodU128` in the per-source-domain account (`v16.rs:14486-14487`), already on the sparse zero-copy path
  (crystallization at `:6742-6749`, terminal-clear at `:9460-9463`). So E6 is NOT a fork-only re-express — only
  the fork's runtime `Vec<u128>` mirror (`fork v16.rs:2678-2679`) drops with runtime-vec; the on-chain fields
  MATCH toly. **SEV mechanical (verify field byte-equality on adopt; drop the Vec mirror + its Kani twin).**

**G-6 WRAPPER FORK FEATURES — re-confirmed at frozen wrapper `0f87dcb`:**
- LP-Vault tags 65-71: **tag collision now wider** — fork 65↔toly 65(UpdateAssetAuthority), 66↔66(BatchTrade
  NoCpi), 67↔67(BatchTradeCpi), **68↔68(SetMatcherConfig)**, **69↔69(RestartAssetOracle)**; fork 70/71 FREE.
  Renumber the whole 65-71 block → 80-86 (target range fully free, G-1). The aggregate-bypass collision is
  CONFIRMED: frozen engine maintains `source_claim_bound_total_num` / `backing_provider_earnings_total` /
  `insurance_domain_budget_remaining_total` via `apply_total_delta` (`v16.rs:5529,:5550`) + a rescan-equality
  check (`:5183-5191`). LP-vault's direct `*source_acc = …from_runtime(&source)` + raw `group.header.vault`
  writes BYPASS that maintenance → re-express through the aggregate-maintaining mutators / view path that
  `handle_withdraw_backing_bucket` uses at frozen (`v16_program.rs:7787`, via `domain_authorities_from_view`).
- LP-vault dual withdraw-gate (5ebd136 class, fork `:6102-6133`): logic SURVIVES (`expected_source_credit_
  rate_num` still at fork `:7952` and is the canonical gate); re-attach it to the re-expressed redeem AND add
  the cc91b07 lag gate (G-2). Re-validate after the aggregate-API rewrite.
- B-3 NFT tags 72/73 (fork `:2821-2829`): FREE at frozen → KEEP. NFT program CPIs tag 72 (transfer_hook.rs:77).
- B-11 oracle cap: G-2.
- WrapperConfig 624→432, AssetOracleProfile 368→400 CONFIRMED (frozen `:49-50`; fork `:52-53`). Frozen profile
  carries `asset_admin:[u8;32]` (`:762`) bootstrapped to `config.marketauth` (`:1409`); fork profile carries
  `insurance_authority` (fork `:302,:355`). ABI delta in §E holds.
