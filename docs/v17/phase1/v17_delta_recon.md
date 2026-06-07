# v17 Convergence — MASTER Phase-1 Design Delta to the FROZEN Target

> **Phase-1 synthesis lead deliverable.** Master design delta from the FORK
> (engine `~/percolator` @ `07fe138`, wrapper `~/percolator-prog` @ `07fe138`) TO the FROZEN
> toly target (engine `~/toly-engine-frozen` = `5c72af3` "Strengthen v16 batch projection proof";
> wrapper `~/toly-wrapper-frozen` = `0f87dcb` "Bump engine and cover residual reward counters").
> Built from 9 source-grounded concern recons; every load-bearing fact below was re-verified at
> `file:line` against the actual frozen and fork trees on 2026-06-07. Git lineage repos:
> `~/toly-percolator` (ENGINE history; lib.rs/v16.rs/wide_math.rs) and `~/toly-percolator-prog`
> (WRAPPER history). Supersedes the OLD-PIN `collision_matrix.md` draft.
>
> **Standing decisions in force:** (1) DROP runtime-vec → single zero-copy/sparse path.
> (2) Adopt toly's full frozen arch. (3) Redesign stake PDA insurance custody onto per-asset
> tag-65 auth, preserving "no admin key drains insurance; PDA controls inflow + terminal-reclaim".
> (4) Keep our matcher superset. (5) Match toly exactly by CONTENT for adopted code; re-express
> fork features onto the new arch preserving economic/security properties.

---

## (a) EXECUTIVE SUMMARY

v17 is ONE big convergence cycle to toly's frozen architecture. Three structural truths anchor it:

1. **The fork is a DUAL-MIRROR, not a runtime-only engine.** The fork ALREADY carries the full
   zero-copy `View`/`ViewMut` production path identical in structure to frozen (fork
   `MarketGroupV16View`@1978 / `ViewMut`@1983 / `PortfolioV16View`@2007 / `ViewMut`@2012; all 16
   `*Account` POD structs present). It ADDITIONALLY keeps the heap-`Vec` runtime path behind
   `cfg(any(kani, feature="runtime-vec-api"))` (fork `PortfolioAccountV16`@2661 with 13 `Vec`
   fields; `MarketGroupV16`@2805 with 5 `Vec` fields). **Dropping runtime-vec deletes the SECOND
   mirror only; the production twin already exists and matches toly by structure.** Runtime-vec is
   100% absent at frozen (`grep -c 'runtime-vec-api' ~/toly-engine-frozen/src/v16.rs = 0`;
   `grep -cE '\bVec<' = 0`).

2. **Most of the engine "convergence" is ADOPT-BY-CONTENT, not re-port.** The fund fixes
   (`051e268`, `c120fce`), sparse portfolio (`945f2db`), O(1) DoS refactor (`4bd3d79`+`b01c8e0`),
   batch trade (`0fa25bd`/engine `05e6e98`), residual reward counters (`58dc118`), and E6
   capital-at-risk (`a57a408`) ARE the frozen baseline we converge TO. The genuine fork-only ENGINE
   adds that must be RE-EXPRESSED onto the zero-copy/sparse path are a short list: A-1 admit
   threshold, A-4 fork_facade + pub-lifts, A-6 stress envelope, A-9 fee mutator, A-10 upper cap
   (one clause), and the `lp_vault` module (survives clean).

3. **The hard collisions are concentrated in the WRAPPER tag space + the STAKE CPI wire.** Toly now
   OWNS tags 65 (UpdateAssetAuthority), 66 (BatchTradeNoCpi), 67 (BatchTradeCpi), 68 (SetMatcherConfig),
   69 (RestartAssetOracle). The fork's LP-vault block 65-71 and tag-32-with-kind collide head-on, and
   the stake program's Bind/Rotate CPI is hard-pinned to the deleted tag-32-with-kind 34-byte wire.

**Fund-critical escalations: NONE blocking.** The single previously-escalated item — whether the
"no-admin-drain" insurance invariant is preservable for the PDA-bound authority at match-toly — is
**DE-ESCALATED**: toly's own `0cf5134` added an `asset_index != 0` clause to the tag-57 marketauth
shutdown-drain (`~/toly-wrapper-frozen/src/v16_program.rs:8257`), closing the exact hole **in-policy**
for the stake asset-0 pool. The invariant holds at match-toly with a known constraint (bind MUST
target asset_index==0; binding a nonzero asset re-opens the drain). See §(g).

**One material correction to the OLD-PIN docs (flagged, not silently changed):** the frozen matcher-auth
mechanism is the INLINE `PortfolioMatcherConfigV16` (104B appended to the portfolio account; `7144d9b`
"Bind matcher config to LP portfolios"), NOT the separate `matcher-auth` PDA / `KIND_MATCHER_AUTH=5` /
`SetMatcherAuthorization` that `collision_matrix.md` §G-1 and the fork-feature-collision-pass concern
attributed to `8690121`. At frozen, `grep -nE 'matcher-auth|KIND_MATCHER_AUTH|SetMatcherAuthorization'`
= 0 hits; the frozen struct is `PortfolioMatcherConfigV16`@828 {matcher_program, matcher_context,
matcher_delegate, enabled:u64}; tag 68 = `SetMatcherConfig{enabled:u8}`. `517a55a`/`8690121` were
SUPERSEDED by `7144d9b`. The fund-fixes concern captures the correct final form; this synthesis adopts it.

**Second material note — RETRACTED (was a false escalation; corrected by execution lead 2026-06-07):**
The synthesis draft claimed the "live toly tips" were AHEAD of the frozen pins on Findings C/D/E/F/G.
**This is FALSE.** It mistook the STALE detached HEADs of the local reference checkouts
(`~/toly-percolator`@`b6e23b3`, `~/toly-percolator-prog`@`4ee339d`) for toly's live upstream tips.
Verified via `git merge-base --is-ancestor` against the frozen pins:
- frozen engine `5c72af3` CONTAINS `b6e23b3` (Finding D), `7188eec` (Finding C), `f9af174` (Finding E),
  `0bee8ef` (resolved wind-down source liens), `a57a408` (E6 counter) — `5c72af3` is **225 commits
  AHEAD** of `b6e23b3` (`5c72af3..b6e23b3 = 0`).
- frozen wrapper `0f87dcb` CONTAINS `4ee339d` (Findings F/G) — **327 commits AHEAD** (`0f87dcb..4ee339d
  = 0`).
The frozen target is COMPLETE and contains every fund-relevant Finding C–G. **NO re-pin.** This also
honors the brief's standing instruction: target FROZEN at `5c72af3`/`0f87dcb` for the program duration;
do not chase toly's daily ships. §(h) escalation #1 is correspondingly DISMISSED.

---

## (b) DROP-RUNTIME-VEC PLAN

Canonical toly drop event: `f26a6f06` (2026-05-28 "Remove v16 runtime engine path"); `git show
--numstat f26a6f0` in `~/toly-percolator`: `src/v16.rs +114/-8870`, `tests/proofs_v16.rs +151/-12194`
(total 563 ins / 31690 del). Cargo.toml diff removes `test = ["runtime-vec-api"]`,
`runtime-vec-api = []`, sets `fuzz=[]`, drops `required-features=["test"]`. Sparse + persistence
follow-ons: `945f2db` (source_domains → embedded `[PortfolioSourceDomainV16Account;
PORTFOLIO_SOURCE_DOMAIN_CAP=32]`, ABI break) → `64c665a` (persisted sparse) → `c120fce`
(persistence + CU) → `4bd3d79` (O(1) totals, header ABI +72B).

### DELETION LIST (engine `~/percolator/src/v16.rs`)

1. `struct PortfolioAccountV16` (2661) + `impl PortfolioAccountV16` (2695-2804): 4 methods (`empty`,
   `ensure_source_domain_capacity`, `source_domain_capacity`, `checked_source_domain_capacity`). 13
   heap-`Vec` fields.
2. `struct MarketGroupV16` (2805) + the entire runtime-gated `impl MarketGroupV16` (13102-21611) =
   ~255 non-kani methods + ~85 `kani_` shims. 5 `Vec` fields.
3. The 11 heap-vec CONVERTERS (the literal "dual-mirror tax", all gated): `from_runtime_group_slot`
   (4106), `write_runtime_group_slot` (4148), `from_runtime_with_capacity` (4591),
   `try_to_runtime_with_market_slots` (4648/4652), `try_to_runtime_with_slots` (4810), `from_runtime`
   (12852), `write_runtime` (12886), `from_runtime` (12979), `source_domains_from_runtime` (13010),
   `try_to_runtime_with_source_domains` (13024).
   **DO NOT TOUCH** the 12 ungated VALUE-codec `from_runtime`/`try_to_runtime` pairs
   (`V16ConfigAccount`@3536/3569, `HealthCertV16Account`, `SourceCreditStateV16Account`, etc.) — those
   are the surviving zero-copy codec layer present in frozen too (frozen has 12 of each, all ungated).
4. Runtime shape/capacity validators (`validate_runtime_storage_shape`, `source_domain_capacity`,
   `ensure_source_domain_capacity`, `ensure_account_source_domain_capacity`,
   `validate_account_source_domain_capacity`, `storage_domain_count`; ~33 grep hits). Frozen = 0;
   replaced by sparse slot-search helpers `source_domain_slot()` / `source_domain_slot_or_insert()` /
   `compact_source_domains()` over CAP=32.
5. All 28 `cfg(any(kani, feature="runtime-vec-api"))` sites (verified `grep -c` = 28 at fork).
6. Cargo.toml: delete `runtime-vec-api` feature, the `test = ["runtime-vec-api"]` mapping,
   `required-features=["test"]` on `v16_spec_tests`; set `fuzz=[]`. Mirror toly exactly.
7. The fork E6 runtime `Vec<u128>` mirror (fork `v16.rs:2678-2679`) — the on-chain POD fields stay
   (CONVERGED, see §(d) E6).

### RE-EXPRESS SET (keep, re-point to zero-copy — NOT deleted)

- `fork_facade` module (26 fns, fork `v16.rs:22172`-EOF). Re-point every fn from `&PortfolioAccountV16`
  (heap) → `&PortfolioAccountV16Account` / `&PortfolioV16View<'_>`. Wrapper (`~/percolator-prog`) is the
  consumer — this is the wrapper-facing break. No analog in frozen (`grep fork_facade` = 0).
- `account_equity` family + A-4 visibility lifts: `account_equity` (21679),
  `account_no_positive_credit_equity` (21689), `account_no_positive_credit_equity_with_capital`,
  `ensure_initial_margin`, `ensure_no_positive_credit_initial_margin` + ~44 free fns (21678-22171).
  Change arg from `&PortfolioAccountV16` (heap) → `&PortfolioAccountV16Account` (POD) / scalar parts.
  Bodies already funnel through `account_equity_from_parts` (frozen UNGATED primitive @14663). Remove
  the runtime cfg gate. **Note:** in frozen these are PRIVATE (`fn` not `pub fn`); re-lift under
  `#[cfg(feature="fork-facade")]` — see §(d) api-encapsulation rows.

### SINGLE ZERO-COPY END-STATE

The surviving production surface = the 16 zero-copy `#[repr(C)]` `*Account` POD structs + `View`/`ViewMut`
wrappers, exactly as frozen:

- View wrappers (frozen): `MarketGroupV16View`@2310 / `MarketGroupV16ViewMut`@2315 wrap
  `&MarketGroupV16HeaderAccount` + `&[Market<T>]`; `PortfolioV16View`@2339 / `PortfolioV16ViewMut`@2343
  wrap `&PortfolioAccountV16Account`. Trait surface `MarketSlotV16View`@4453 / `MarketSlotV16ViewMut`@4457
  impl'd for `EngineAssetSlotV16Account`@4461/4467.
- VALUE-codec `from_runtime`/`try_to_runtime` survive in BOTH engines (encode/decode `#[repr(C)] Copy`
  VALUE structs like `V16Config`, `HealthCertV16`, `AssetStateV16` — NOT heap structs).
- Kani proofs run DIRECTLY against zero-copy/value types (frozen has 84 `fn kani_` + 93 `#[cfg(kani)]`
  blocks; sample: `kani_add_open_interest_for_new_position(asset:&mut AssetStateV16,…)`@378;
  `kani_validate_dynamic_market_slot_shape_at<S:MarketSlotV16View>`@4922;
  `activate_empty_market_slot_not_atomic<S:MarketSlotV16ViewMut>`@4930).
- Production batch-trade twin sig:
  `execute_batch_with_fee_in_place_not_atomic(&mut self, long_account:&mut PortfolioV16ViewMut<'_>,
  short_account:&mut PortfolioV16ViewMut<'_>, requests:&[TradeRequestV16])`.

### HARNESS-REBUILD SURFACE MAP (fork runtime fn → frozen zero-copy/ViewMut twin)

`impl MarketGroupV16` methods → `impl MarketGroupV16ViewMut<'a,T>` (fork already has the struct @1983;
frozen big impl @5377-14194, 314 methods). Specific renames the rebuild targets:

| Fork runtime fn | Frozen zero-copy/ViewMut twin |
|---|---|
| `execute_trade_with_fee[_in_place]_not_atomic` | `execute_batch_with_fee_loss_stale_scoped_not_atomic` (single routes through batch via `core::slice::from_ref`; frozen @12423/12441) |
| `liquidate_account_core_not_atomic` | `liquidate_account_not_atomic` (frozen @11890) |
| `permissionless_crank_core_not_atomic` | `permissionless_crank_not_atomic` (frozen @9940) |
| `deposit_core_not_atomic` / `withdraw_core_not_atomic` | `deposit_fresh_counterparty_backing_not_atomic`@5911 / `withdraw_fresh_counterparty_backing_not_atomic`@5940 + `deposit_domain_insurance_not_atomic`@7298 / `withdraw_domain_insurance_not_atomic`@7378 |
| `apply_position_delta_inner` | `apply_position_delta_with_lookup_inner` (frozen @11107) |
| `account_*` per-portfolio helpers (haircut_equity_with_capital, health_leg_requirements, compute_account_health_cert, certify_account_after_local_settlement, full_account_refresh, mark_account_stale, create/close_portfolio_account, validate_account_*, build_account_health_cert_from_*) | the `PortfolioV16View`/`ViewMut` impls (frozen view @2490 / viewmut @2853) taking `&PortfolioV16View` |
| ~50 fork runtime-type `kani_` harnesses | the ~84 frozen `kani_` shims (`kani_add_open_interest_for_new_position(&mut AssetStateV16)`, `kani_health_cert_after_capital_debit(HealthCertV16)`, `kani_validate_dynamic_market_slot_shape_at<S:MarketSlotV16View>`, `activate_empty_market_slot_not_atomic<S:MarketSlotV16ViewMut>`) |

**APPLY ORDER:** `f26a6f0` (drop) → `945f2db` (sparse, ABI) → `64c665a` (persisted sparse) → `c120fce`
(persistence + CU) → `4bd3d79` (O(1) totals, header ABI add). After convergence the fork's kani harness
count should fall from **113 → ~84** (drop the runtime-exposure shims). Verify the adopted engine core
byte-for-byte vs frozen `5c72af3`; verify re-expressed `fork_facade`/`account_equity` preserve economic
outputs via the existing fork LiteSVM/Kani gate.

**ABI impact of the DROP ITSELF: NONE** — the heap-Vec form was never serialized; the surviving POD
layout is byte-identical. ABI breaks come from the SPARSE + O(1) follow-ons (§(f)).

---

## (c) PER-CONCERN FROZEN-STATE FACTS (source-grounded)

All re-verified at source 2026-06-07.

### Drop-runtime-vec
- `grep -c 'runtime-vec-api' ~/toly-engine-frozen/src/v16.rs` = **0**; `grep -cE '\bVec<'` = **0**;
  no runtime structs at frozen. Fork: 28 cfg sites, `PortfolioAccountV16`@2661, `MarketGroupV16`@2805,
  `lp_vault` mod @22482, **113** `fn kani_` (vs frozen **84**).

### Sparse portfolio (frozen)
- `source_domains: [PortfolioSourceDomainV16Account; PORTFOLIO_SOURCE_DOMAIN_CAP]` INLINE in
  `PortfolioAccountV16Account` (frozen `v16.rs:2353` accessor confirms the array form).
- `PORTFOLIO_SOURCE_DOMAIN_CAP` = `2 * V16_MAX_PORTFOLIO_ASSETS_N` = **32** (non-kani; frozen
  `v16.rs:20,24`); **4** under kani (`:22`). `V16_MAX_PORTFOLIO_ASSETS_N = 16` (`:20`).
- `PortfolioSourceDomainV16Account` gains a leading `pub domain: V16PodU32` tag (frozen `:14470`);
  measured 196B (fork 192B; +4B for the tag). Inline block = 32×196 = 6272B.
- `PortfolioAccountV16Account` measured 9227B (fork 2907B; +6320B = 6272 inline + 48 residual).
- Sparse lookup is a domain-TAGGED linear scan: `source_domain_slot` (`:2357`),
  `source_domain_slot_or_insert` (`:2422`), `compact_source_domains` (`:2469`); ViewMut::new compacts
  on construction (`:2391`); full-slots insert → `Err(LockActive)` (`:2442`).
- `PortfolioV16View`/`ViewMut` are HEADER-ONLY (frozen `:2339-2345`, single field); fork is a 2-field
  fat view (`{header, source_domains slice}`, fork `:2007-2010`).

### O(1)-in-N DoS refactor (frozen)
- 5 cached aggregates inserted in `MarketGroupV16HeaderAccount` between `pnl_matured_pos_tot` and
  `materialized_portfolio_count` (frozen `:4670-4674`, re-verified): `backing_provider_earnings_total`
  (V16PodU128), `source_claim_bound_total_num` (V16PodU128),
  `source_insurance_credit_reserved_total_atoms` (V16PodU128), `insurance_domain_budget_remaining_total`
  (V16PodU128), `resolved_payout_blocker_count` (V16PodU64). Header **+72B**; shifts every dynamic
  asset-slot offset +72.
- Production O(1) check = `validate_header_aggregate_totals` (frozen @5146): senior =
  `c_tot+insurance+backing_provider_earnings_total <= vault`; the 3 insurance/budget totals `<= insurance`;
  `pnl_pos_bound_tot >= source_claim_bound_total_num`. Full scan gated behind
  `#[cfg(any(test,kani,feature="audit-scan"))]` (`:5180/5200`); rescan-equality check (`:5183-5191`).
- 6 new pub APIs (`b01c8e0`): `credit_backing_provider_earnings_not_atomic` (@6547),
  `credit_domain_insurance_budget_not_atomic` (@7285), `deposit_domain_insurance_not_atomic` (@7298),
  `withdraw_domain_insurance_not_atomic` (@7378), `credit_account_from_insurance_not_atomic` (@7458),
  `set_domain_insurance_spent` (@7210); + `charge_account_backing_fee_not_atomic` (@6583).
- ONE legal remaining direct `header.insurance +=` in toly: the backing-fee capital-slack→insurance
  split (no engine entry point yet). Fork engine has NEITHER the 5 fields nor the 6 APIs; still runs the
  O(N) scan (`backing_provider_earnings_total()`@5334 looped in `residual()`@5351) — the live DoS vector.

### Auth overhaul (frozen wrapper)
- `WRAPPER_CONFIG_LEN` **432** (frozen `:49`; fork **624** `:52`); collapse via `792256b`.
- `ASSET_ORACLE_PROFILE_LEN` **400** (frozen `:50`; fork **368** `:53`); +asset_admin via `f64b7ee`.
- `WrapperConfigV16` first field = `marketauth:[u8;32]` (frozen `:686`); the 6 former domain keys
  deleted. `AssetOracleProfileV16.asset_admin:[u8;32]` (frozen `:762`) bootstrapped to
  `config.marketauth` (`:1409`). Per-asset insurance/operator/backing/oracle authorities pre-exist on
  the profile in BOTH (frozen `:743-746`; fork `:355-358`).
- Tag 32 `UpdateAuthority{new_pubkey:[u8;32]}` (NO kind; frozen `:3179`); handler unconditionally rejects
  burn-to-zero (`:8774`, from `087a404`).
- Tag 65 `UpdateAssetAuthority{asset_index:u16, kind:u8, new_pubkey:[u8;32]}` (frozen decode `:3182`);
  kinds `ASSET_AUTH_ADMIN=0, INSURANCE=1, INSURANCE_OPERATOR=2, BACKING_BUCKET=3, ORACLE=4`
  (`:4661-4665`); handler `:8791`. Burn-to-zero rejected for all kinds EXCEPT `ASSET_AUTH_ADMIN`
  (`:8840`, from `c37d3e4`).

### Batch trade (frozen)
- Engine entry `execute_batch_with_fee_loss_stale_scoped_not_atomic` (frozen `:12441`); single-trade
  wrapper `execute_trade_with_fee_loss_stale_scoped_not_atomic` (`:12423`) routes through it via
  `core::slice::from_ref`. Settle-once + N-applies + ONE end-state margin check in
  `execute_batch_with_fee_after_tail_validation_not_atomic` (`:12483-12549`); `recertify_after_fill =
  (requests.len()==1)`. Fork engine has ZERO batch internals (greenfield adopt).
- Wrapper tags 66 `BatchTradeNoCpi{Vec<BatchTradeLeg{asset_index:u16,size_q:i128,exec_price:u64,
  fee_bps:u64}>}` (frozen `:3125`), 67 `BatchTradeCpi{Vec<BatchTradeCpiLeg{asset_index:u16,size_q:i128,
  fee_bps:u64,limit_price:u64}>}` (`:3138`); wire = tag, u8 leg count, fixed legs (size_q SIGNED i128).
  `MATCHER_BATCH_MAX_LEGS=16`. Commit `0fa25bd` (engine pin `05e6e98`).

### Fund fixes + new functional (frozen)
- `051e268` PRESENT (terminal insurance-lien release: `prepare_insurance_lien_terminal_release_delta`
  @1034, `release_source_credit_lien_from_insurance_terminal_not_atomic` @6355).
- `c120fce` PRESENT (source-claim persistence + O(1)/stale-trade CU; `!has_default_sparse_tag()` guard
  @2429; `is_sparse_tail_default()` fast-path @9167).
- `58dc118` PRESENT (3 monotonic u128 residual reward counters on `PortfolioAccountV16Account`
  @14535-14537; spent<=crystallized guard @2506/5474; transfer-on-trade @8341). NON-margin-affecting.
- Wrapper `c2019a8` accessors + `0f87dcb` mirrors the +48B (3 u128) in pack/unpack.
- Matcher-auth FINAL form = inline `PortfolioMatcherConfigV16` (104B, `7144d9b`); tag 68
  `SetMatcherConfig`; dual signed/unsigned-LP tail (`matcher_tail_start_or_verify_lp_config` @6711).
  **The separate `matcher-auth` PDA / KIND_MATCHER_AUTH=5 / SetMatcherAuthorization is GONE at frozen**
  (`grep` = 0). `PORTFOLIO_MATCHER_CONFIG_LEN=104`, `PORTFOLIO_ACCOUNT_LEN = engine_portfolio + 104`
  (frozen `:65-67`).
- `03873e8` PRESENT (secondary-vault recovery on close slab; removes
  `verify_withdrawable_vault_token_account`).
- `cc91b07` PRESENT but relocated by `0cf5134` into `live_domain_withdraw_health_or_shutdown_view`
  (`reject_exposed_target_effective_lag_view` @4851, reached from `handle_withdraw_insurance_asset` @8253).
- `0cf5134` PRESENT (major ABI consolidation): REMOVES tag 23 `WithdrawInsuranceLimited`, tag 33
  `UpdateInsurancePolicy`, const `MIN_INSURANCE_WITHDRAW_FLOOR_UNITS`; CHANGES tag 57
  `WithdrawInsuranceDomain{domain:u8}` → `WithdrawInsuranceAsset{asset_index:u16}` (frozen `:3293`,
  handler `:8200`); `long_domain = asset_index*2`.
- `0f87dcb` widened domain u8→u16 on tags 56/24/50/51/52/53.

### API-encapsulation sweep (frozen engine)
- Frozen `lib.rs:40-43`: `#[cfg(kani)] pub mod v16` + `pub use v16::*` vs `#[cfg(not(kani))] mod v16`
  (private) + curated 60-symbol allow-list (`pub use v16::{…}` @52-73). `wide_math` private under
  not(kani) (`:44-47`). Fork `lib.rs:40,43`: unconditional `pub mod v16; pub use v16::*` (blanket).
- 27 sweep-hidden symbols (24 fns + 3 proof structs + the `wide_math` module privatization). Removed
  trade API replaced by the scoped variants (`b75d352`). `TokenValueFlowProofV16` pub-under-kani /
  private-under-not(kani) (frozen `:3279/3291`). A-4-class fns are PRIVATE in frozen.

### Stake re-validation (frozen wrapper)
- `0cf5134` added `asset_index != 0` to the marketauth shutdown-drain predicate (frozen `:8257-8259`):
  `admin_shutdown_authorized = asset_index != 0 && shutdown_drain &&
  live_authority_matches(&cfg.marketauth, operator.key)`. For asset 0, marketauth can NEVER take the
  tag-57 shutdown-drain branch. (Re-confirmed: `grep 'asset_index != 0'` → `:4894`, `:8257`.)
- `asset_admin`-burn carve-out (`c37d3e4`) unchanged at frozen (`:8840`). Self-rotate path for the
  PDA survives (`!admin_signed => expect_live_authority(current_value)`).
- Fork stake CPI emits OLD wire: `cpi.rs:131-132` `TAG_UPDATE_AUTHORITY=32` + `AUTHORITY_INSURANCE=2`
  + pubkey = 34B (canary `:207-221`); fork tags 19/20 = Bind/Rotate (`instruction.rs:300-301`).

### Fork-feature collision pass (frozen)
- Fork engine adds confirmed absent at frozen: A-1 (no admit field; frozen `TradeRequestV16` `size_q`
  is i128 vs fork u128), A-4 fork_facade + lifts, A-6 stress envelope (4 fields), A-9 fee mutator.
  A-10 PARTIALLY converged (field + `==0` guard in frozen `v16.rs:1462/1950`; only `> MAX_MARGIN_BPS`
  upper cap is fork-only). E6 CONVERGED (`a57a408`; fork `16c4324` was a PORT). `lp_vault` survives clean.

---

## (d) COMPLETE REFRESHED COLLISION MATRIX

> Severity legend: **fund-critical** (silent drain / lockout / aggregate-drift risk if mishandled) ·
> **breaking** (compile/decode/ABI break needing explicit re-expression) · **mechanical** (rename /
> renumber / one-clause / verbatim port) · **none** (survives clean). The full table is also written
> standalone to `v17_collision_matrix_refreshed.md`.

| # | Fork feature (loc) | sparse | O(1) | auth+burn | drop-runtime-vec | batch | api-hide | tag-renumber | new-functional | Severity | Resolution |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | **Runtime heap-Vec structs** `PortfolioAccountV16`@2661 + `MarketGroupV16`@2805 + impls | — | — | — | DELETED by `f26a6f0` (frozen has none) | — | — | — | — | mechanical | DELETE both defs + impls (255+85 methods). Production runs the View/ViewMut twin the fork already has. No economics lost. |
| 2 | **Heap-vec converters** (11, gated) | — | — | — | no analog in frozen | — | — | — | — | mechanical | DELETE all 11. Keep the 12 ungated VALUE-codec pairs (present in frozen). |
| 3 | **fork_facade mod** (26 fns @22172) | reads source_domains | reads aggregates | — | imports heap types | — | private in frozen | — | — | breaking | RE-EXPRESS: re-point `&PortfolioAccountV16`→`&PortfolioAccountV16Account`/`&PortfolioV16View`; re-lift under `#[cfg(feature="fork-facade")]`. Wrapper-facing break. |
| 4 | **account_equity family + A-4 lifts** (`account_equity`@21679, `account_no_positive_credit_equity`@21689 + ~44 free fns) | — | aggregate consumers | — | heap arg type + runtime gate | — | PRIVATE in frozen | — | — | breaking | RE-EXPRESS: arg→POD/parts; bodies funnel through `account_equity_from_parts` (frozen @14663); re-lift verbatim under fork-facade. |
| 5 | **~85 kani_ shims in impl MarketGroupV16** + ~28 free kani_ fns (113 total vs frozen 84) | — | — | — | runtime-gated | — | — | — | — | mechanical | REBUILD onto View/ViewMut twin; drop ~29 pure runtime-exposure shims after rebuild. |
| 6 | **Runtime shape/capacity validators** (~33 hits) | obviated by sparse slot-search | — | — | runtime-only | — | — | — | — | mechanical | DELETE; callers use `source_domain_slot()`/`_or_insert()`/`compact_source_domains()` (CAP=32). |
| 7 | **Cargo.toml runtime-vec feature** (test/runtime-vec-api/required-features) | — | — | — | removed by `f26a6f0` | — | — | — | — | mechanical | Mirror toly: delete feature + mappings, `fuzz=[]`. |
| 8 | **A-1 admit-threshold** (`TradeRequestV16.admit_h_max_consumption_threshold_bps_opt`; h_lock_lane gate) | header path; no source_domain touch | re-validate gate fires pre-mutation | — | drop runtime twin, keep ViewMut | gate must run **per-leg inside batch loop** | — | — | size_q i128(frozen) vs u128(fork) reconcile | breaking | RE-EXPRESS: re-add `Option<u128>` onto frozen `TradeRequestV16`; gate on ViewMut h_lock_lane + per-leg in batch path; reconcile size_q type. |
| 9 | **A-4 pub-lifts** (8 in-place) | — | aggregate readers | — | rebuild on ViewMut | — | toly narrows surface → re-lift behind fork-facade | — | — | breaking | RE-LIFT verbatim under `#[cfg(feature="fork-facade")] pub fn` on frozen private bodies (they already take `&PortfolioV16View`); verify by CONTENT. |
| 10 | **A-6 stress-envelope** (4 fields: threshold_stress_active + 3 envelope; POD @4398-4407) | header POD, not source_domains | **APPEND fields AFTER the 5 O(1) aggregates; exclude from rescan-equality set (`:5183`)** | — | drop runtime mirror writer | — | — | — | — | breaking | REBUILD + layout-reconcile: keep 4 header POD fields appended after aggregates; drop runtime writer; confirm not in rescan set. |
| 11 | **A-9 fee mutator** (`FeePolicyUpdateV16` 4 fields; ViewMut mutator + 3 Kani proofs) | market-header scoped | route fee→insurance through aggregate API | — | drop runtime mutator | — | — | **new tag in renumber range** | toly post-init mutators (37/49/51/55) NARROWER → A-9 not obviated | breaking | RE-EXPRESS as a new wrapper tag (renumber range) → engine config writer; keep ViewMut mutator; rewrite 3 Kani proofs onto ViewMut. Partial overlap tag-55 trade_fee_base vs A-9 max_trading_fee. |
| 12 | **A-10 max_price_move upper cap** (`> MAX_MARGIN_BPS`) | — | — | — | zero-copy compatible | — | — | — | field CONVERGED (frozen `:1462`+`==0`@1950) | mechanical | Re-insert ONE clause `|| self.max_price_move_bps_per_slot > MAX_MARGIN_BPS` in frozen validate. |
| 13 | **engine lp_vault mod** (@22482; primitive NAV/share math) | no source_domain touch | pure primitive math | — | survives clean | — | reaches `crate::wide_math` even when private (crate-visible) | — | absent in frozen | none | SURVIVES-CLEAN: re-add as engine-internal `#[cfg(feature="fork-facade")]` module, byte-identical; re-run NAV/donation-defense Kani. |
| 14 | **E6 capital-at-risk** (2 per-domain V16PodU128; runtime Vec mirror + POD) | rides inline sparse domain POD (+32B carrier change) | aggregate must not double-count | — | drop ONLY the Vec mirror | — | — | — | **CONVERGED** (toly `a57a408`; fork `16c4324` = port; frozen `:14486-14487`) | mechanical | Adopt toly's POD fields verbatim; drop fork Vec mirror + Kani twin; verify byte-equality; confirm crystallize (`:6742`)/terminal-clear (`:9460`). |
| 15 | **Residual reward counters** (portfolio-level, 3 u128) | rides portfolio header | non-margin | — | engine-owned, rides zero-copy automatically | transfer-on-trade in fill path | — | — | **MISSING in fork** (toly `58dc118`) | breaking | ADOPT: add 3 fields to `PortfolioAccountV16Account` (+48B) + `record_account_residual_crystallized_loss` + spent<=crystallized guard; wrapper mirrors in pack/unpack. |
| 16 | **Wrapper portfolio deserializers** (`portfolio_source_domain_*` + dynamic 2N tail) | frozen reads single fixed inline blob | — | — | runtime-vec hydration helpers drop | — | — | — | — | breaking | DELETE fork tail helpers + split view builder; adopt frozen single-blob `portfolio_wire` + one-arg `PortfolioV16ViewMut::new`. |
| 17 | **Portfolio account rent/size** (`PORTFOLIO_ACCOUNT_LEN`, dynamic 2N tail) | fixed 9243B inline vs dynamic tail | — | — | — | — | — | — | +104B inline matcher config | **fund-critical** | Hard ABI/byte-size break. Fresh-start cutover (decision). New fixed portfolio = HEADER(16)+engine(9227)+matcher(104). All init/rent/decoders switch to fixed constant. Verify init zeroing. |
| 18 | **Wrapper local budget helpers** `set_domain_budget_view`/`add_to_domain_budget_view` (@4616-4652) | per-domain slot | bypass `insurance_domain_budget_remaining_total` | — | mutate Vec mirror directly | — | — | — | — | **fund-critical** | DELETE; route through `credit_domain_insurance_budget_not_atomic`@7285. Raw writes drift the total → `validate_header_aggregate_totals` rejects. |
| 19 | **Wrapper fee-credit helpers** (`credit_market_insurance_budget_view`@4664, `credit_fee_to_domain_budget_view`@4804, +2; hot path @7139) | per-domain | DoS hot path | — | Vec mirror | per-leg in batch | — | — | — | **fund-critical** | Re-express as thin wrappers over `credit_domain_insurance_budget_not_atomic`, PRESERVING fork `fee_redirect_to_market_0_bps` split + market-0 redirect. |
| 20 | **handle_withdraw_insurance_domain inline triple** (insurance/vault checked_sub + budget; @8978-9001) | — | bypass budget total | tag 57 now WithdrawInsuranceAsset (u16) | — | — | — | tag 57 wire change | `0cf5134` | **fund-critical** | Replace triple with `withdraw_domain_insurance_not_atomic`@7378 (folds vault−/insurance−/budget− atomically). Keep insurance-ledger sync as post-step. |
| 21 | **handle_withdraw_insurance / _limited** (terminal/market-wide debit) | — | bypass | tag 23 DELETED; 57→asset | — | — | — | — | `0cf5134` | **fund-critical** | Loop per-domain debit through `withdraw_domain_insurance_not_atomic`; keep amount-computation views. |
| 22 | **Crank backing-fee split** (`header.insurance += insurance_fee`; bucket += provider_fee; budget credit; @12096 + zero-copy @11526) | — | bucket/total maintenance | — | zero-copy crank path | — | — | — | one legal inline `header.insurance +=` at frozen | **fund-critical** | provider_fee → `credit_backing_provider_earnings_not_atomic`@6547; insurance_fee budget → `credit_domain_insurance_budget_not_atomic`; KEEP the single inline `header.insurance +=` (matches frozen). |
| 23 | **LP-vault DepositToLpVault vault increment** (@5665-5685) | — | vault-increase relaxes senior<=vault; 4 totals untouched | — | re-verify on zero-copy | — | — | renumber | — | mechanical | Keep direct vault write (SAFE — relaxes the only check it touches). Re-verify backing-bucket write bypasses `utilization_fee_earnings`. |
| 24 | **LP-vault ExecuteRedemption vault decrement** (@6137) + **resolved-payout topup** (@11286) | — | **bare vault− risks senior<=vault** | — | — | — | — | renumber | — | breaking | FLAG: PROVE redeemed atoms are junior/unliened backing (senior stays <= new_vault), OR add a fork engine entry point analogous to `withdraw_domain_insurance_not_atomic`. Highest-risk re-expression item. |
| 25 | **Insurance top-ups** (`handle_top_up_insurance`@7765, `_domain`@7906) | — | budget total | tag 56 domain u16 | — | — | — | — | `0f87dcb` u16 | breaking | Route through `deposit_domain_insurance_not_atomic`@7298 (vault+insurance+budget atomic). Backing-bucket vault top-up (@8378) stays vault-only (verify untouched earnings). |
| 26 | **Crank reward to insurance** (`handle_sync_maintenance_fee`@9460/9522; zero-copy @11526) | — | unbudgeted credit safe; payout must respect budget | — | zero-copy crank | — | — | — | — | breaking | Per-site classify: pure insurance-CREDIT = safe (raises insurance, relaxes both checks); account PAYOUT must use `credit_account_from_insurance_not_atomic`@7458. Cranker-reward economics are a fork KEEP (by-design). |
| 27 | **WrapperConfigV16 7 authority keys** (admin/base_unit/insurance/operator/backing/asset/mark) | — | — | **COLLAPSED to marketauth (624→432)** | — | — | — | — | `792256b` | breaking | ADOPT frozen collapsed layout; re-express every LP-vault/fork authority check that read a deleted key → `cfg.marketauth` (market-scoped) or per-asset profile via `domain_authorities_from_profile` (domain-scoped, frozen `:7657-7670`). |
| 28 | **AssetOracleProfileV16 +asset_admin** (368→400) | — | — | per-asset cold-storage admin | — | — | — | — | `f64b7ee` | breaking | ADOPT verbatim (+32B). asset_admin bootstrapped to marketauth (`:1409`). |
| 29 | **Fork tag 32 UpdateAuthority{kind,new_pubkey}** (7-value config kind enum) | — | — | tag 32 → marketauth-only (no kind, burn rejected) | — | — | — | same-tag/diff-wire | `792256b`/`087a404` | breaking | DROP the kind byte; adopt frozen tag 32 verbatim; map config-level authority rotations onto tag 65 (TOLY kind values; note insurance=1 not fork's 2). |
| 30 | **NEW tag 65 UpdateAssetAuthority** (fork lacks; toly owns) | — | — | per-asset rotation, 5-kind enum, burn-rejection carve-out | — | — | — | — | `f64b7ee`/`c37d3e4` | (adopt) | ADOPT verbatim (kinds @4661-4665, handler @8791, burn guard @8840). This is the host for D-STAKE-1. |
| 31 | **LP-vault tags 65-71** (Create/Deposit/RequestRedeem/Execute/CrankFees/SetPaused/Close) | reads source-credit/backing ledger | RE-EXPRESS thru aggregate APIs | gate → marketauth/profile | rebuild on zero-copy | LP is counterparty in batch | — | **collide 65-69** | — | **fund-critical** | RENUMBER 65-71 → **74-80**. Route every backing/insurance/vault write through aggregate-maintaining APIs (rows 18-26). Gate CreateLpVault on marketauth. |
| 32 | **LP-vault dual withdraw-gate** (5ebd136; @6102-6133) | reads source_after on sparse | re-validate vs O(1) | — | rebuild | — | — | — | `cc91b07` lag gate | breaking | Re-attach `expected_source_credit_rate_num` gate (fork `:7952`); ADD `cc91b07` `reject_exposed_target_effective_lag_view` (LP redeem is same-class drain). |
| 33 | **NFT-B3 tags 72/73** (TransferPortfolioOwnership, SetNftProgramId) | portfolio.owner + provenance on sparse header | — | SetNftProgramId admin→marketauth | — | — | — | **FREE at frozen** (top=69) | — | mechanical | KEEP at 72/73 (toly tops at 69; no collision). Re-base SetNftProgramId(73) admin gate → marketauth. NFT program CPIs tag 72 (`transfer_hook.rs:77`) — keep in lockstep. |
| 34 | **B-11 oracle staleness cap** (`MAX_ORACLE_STALENESS_SECS=86400`; @1057/1179) | — | — | re-apply atop restructured config/profile | — | — | — | — | `cc91b07` DISTINCT/complementary | mechanical | RE-APPLY `> MAX_ORACLE_STALENESS_SECS` upper-cap clause at frozen hybrid-oracle validation (frozen has `==0` @1163/1284 but no cap). cc91b07 adopted independently. |
| 35 | **Fork tags 23 WithdrawInsuranceLimited + 33 UpdateInsurancePolicy** | — | — | old global-insurance model | — | — | — | freed at frozen | DELETED by `0cf5134` | breaking | REMOVE both (no tag collision; old model dropped). Re-express any still-wanted intent onto per-asset auth (tag 65 + UpdateAssetLifecycle 40). Frees 23/33. |
| 36 | **Fork tag 57 WithdrawInsuranceDomain{domain:u8}** + u8-domain on 24/50/51/52/53/56 | — | per-asset budget view | — | — | — | — | wire change | `0cf5134`/`0f87dcb` | breaking | ADOPT toly: tag 57 → WithdrawInsuranceAsset{asset_index:u16}; domain u8→u16 on all backing/insurance-domain tags. |
| 37 | **Wrapper trade calls + manual loss-stale toggle** (@7111-7125, calls @7116/7120/7483/7487) | — | — | — | — | scoped API absorbs the toggle | removed trade API (`b75d352`) | — | — | breaking | ADOPT `execute_*_loss_stale_scoped_not_atomic`; DELETE the manual toggle (engine absorbs it, frozen @12459-12468). Loss-stale gate now engine-enforced + Kani-proven. |
| 38 | **Wrapper wide_math cross-crate calls** (7 sites @4253/4322/7963) | — | — | — | — | — | `wide_math` privatized (`24d334d`) | — | — | breaking | Re-export `pub mod wide_math` under `#[cfg(any(kani, feature="fork-facade"))]` (mirror toly's kani pattern), OR move the 2 helpers into engine as proven public APIs. Verify body byte-identical. |
| 39 | **Wrapper TokenValueFlowProofV16 ctor** (@11291) | — | — | — | — | — | kani-only struct (`2fa4561`) | — | — | breaking | Prefer re-express onto the PUBLIC ResolvedClose/ResolvedPayout API (on allow-list) which performs the conservation proof internally; only re-lift the witness under fork-facade if no public seam suffices. |
| 40 | **Wrapper has_pending_residual call** (@11737) | — | — | — | reads `try_to_runtime` (runtime-vec) | — | demoted private (`44cfd06`) | — | — | mechanical | Inline over public `CloseProgressLedgerV16` fields (`active && !finalized && !canceled && residual_remaining != 0`); read the POD account directly post-runtime-vec drop. |
| 41 | **Crate public API** (blanket `pub use v16::*`) | — | — | — | — | — | frozen exports 60 curated symbols | — | — | breaking | ADOPT frozen `lib.rs` (private mod + allow-list); add `fork-facade` feature re-export block for fork-needed extras. Do NOT revert to blanket export. Keeper/SDK-codegen likely also need fork-facade. |
| 42 | **percolator-match superset** (vAMM, asset_index echo bytes[56..64], ABI v3) | — | — | adopt inline matcher-config dual signed/unsigned tail | — | matcher per-leg in batch | — | — | inline `PortfolioMatcherConfigV16` (`7144d9b`) | mechanical | KEEP matcher logic (decision 4; ABI v3 matches, echo @56 read+validated). ADOPT wrapper-side inline `PortfolioMatcherConfigV16` + tag 68 + dual-path tail (signed→[8..], unsigned→[7..]). NOT the abandoned separate-PDA form. |
| 43 | **percolator-stake Bind/Rotate** (tags 19/20; CPI tag32+kind=2 34B) | — | — | **tag 32 marketauth-only; per-asset → tag 65; `0cf5134` asset_index!=0 gate** | — | — | — | CPI re-target | `792256b`/`087a404`/`0cf5134` | **fund-critical** | REWIRE CPI → tag 65 `{asset_index=0, kind=ASSET_AUTH_INSURANCE(1), new_pubkey}` 36B; update canary (`cpi.rs:207-221`). D-STAKE-1; see §(g). |
| 44 | **percolator-nft whole program** (Token-2022 hook, NftRegistry) | — | — | NftRegistry unaffected | — | — | — | depends on wrapper id stability | — | mechanical | KEEP. Re-register via tag 73 if wrapper program-id changes at cutover. CPI tag 72 stays in lockstep. |
| 45 | **CloseSlab handler** (single primary vault drain) | — | — | — | — | — | — | account-list +2 | `03873e8` | mechanical | ADOPT secondary-vault drain+close (frozen `:8369-8447`); removes `verify_withdrawable_vault_token_account`. CloseSlab reads secondary_vault@6 + secondary_dest@7. |
| 46 | **Wrapper portfolio +residual mirror + inline matcher cfg** (layout) | rides zero-copy portfolio | — | — | engine-owned residual rides auto | — | — | — | `58dc118`/`0f87dcb`/`7144d9b` | breaking | ADOPT both adds: residual +48B (engine-owned, auto on zero-copy drop) + inline matcher cfg +104B (wrapper-only, at PORTFOLIO_ENGINE_ACCOUNT_LEN). Reconcile final field order; client decoders regen. |

---

## (e) DEFINITIVE INSTRUCTION-TAG RENUMBER MAP

Toly frozen wrapper OWNS tags 0-69 (top used = 69 `RestartAssetOracle`; tags 70-88 all FREE at frozen).
**Adopt toly tags 0-69 byte-faithfully.** Verified at source (`~/toly-wrapper-frozen/src/v16_program.rs`).

### Frozen toly tag map (the target; load-bearing tags)
| Tag | Variant | Source line |
|---|---|---|
| 32 | `UpdateAuthority{new_pubkey:[u8;32]}` (NO kind, burn rejected) | 3179 |
| 56 | `TopUpInsuranceDomain{domain:u16, amount:u128}` | (decode @3158) |
| 57 | `WithdrawInsuranceAsset{asset_index:u16, amount:u128}` | 3293 |
| 65 | `UpdateAssetAuthority{asset_index:u16, kind:u8, new_pubkey:[u8;32]}` | 3182 |
| 66 | `BatchTradeNoCpi{Vec<BatchTradeLeg{asset_index:u16,size_q:i128,exec_price:u64,fee_bps:u64}>}` | 3125 |
| 67 | `BatchTradeCpi{Vec<BatchTradeCpiLeg{asset_index:u16,size_q:i128,fee_bps:u64,limit_price:u64}>}` | 3138 |
| 68 | `SetMatcherConfig{enabled:u8}` | 3151 |
| 69 | `RestartAssetOracle{asset_index:u16, now_slot:u64, initial_price:u64}` | (decode @3229) |

### Fork-added tags → v17 disposition
| Fork tag | Fork variant (loc) | Collision at frozen | v17 action |
|---|---|---|---|
| 23 | `WithdrawInsuranceLimited{amount:u128}` (@2649) | none (23 free) | **REMOVE** (`0cf5134` dropped the model). Frees 23. Re-express intent on per-asset auth if wanted. |
| 32 | `UpdateAuthority{kind:u8, new_pubkey}` (@2667) | same-tag/diff-wire vs toly 32 | **KEEP tag 32, DROP the kind byte** → adopt toly wire. Kinds move to tag 65. |
| 33 | `UpdateInsurancePolicy{max_bps,deposits_only,cooldown}` (@2671) | none (33 free) | **REMOVE** (`0cf5134`). Frees 33. |
| 57 | `WithdrawInsuranceDomain{domain:u8}` (@2777) | same-tag/diff-semantics | **ADOPT toly 57** = WithdrawInsuranceAsset{asset_index:u16}. |
| 65 | `CreateLpVault` (@2803) | **collides toly 65** (UpdateAssetAuthority) | **RENUMBER → 74** |
| 66 | `DepositToLpVault` (@2809) | **collides toly 66** (BatchTradeNoCpi) | **RENUMBER → 75** |
| 67 | `RequestRedeemLpShares` (@2812) | **collides toly 67** (BatchTradeCpi) | **RENUMBER → 76** |
| 68 | `ExecuteRedemption` (@2815) | **collides toly 68** (SetMatcherConfig) | **RENUMBER → 77** |
| 69 | `LpVaultCrankFees` (@2816) | **collides toly 69** (RestartAssetOracle) | **RENUMBER → 78** |
| 70 | `SetLpVaultPaused` (@2817) | free at frozen | **RENUMBER → 79** (keep LP block contiguous) |
| 71 | `CloseLpVault` (@2820) | free at frozen | **RENUMBER → 80** |
| 72 | `TransferPortfolioOwnership{new_owner:[u8;32],asset_index:u16}` (NFT-B3) | **FREE** at frozen | **KEEP at 72** |
| 73 | `SetNftProgramId{nft_program_id:[u8;32]}` (NFT-B3) | **FREE** at frozen | **KEEP at 73** |

### ADOPT (fork currently lacks these toly frozen tags; add verbatim)
65 UpdateAssetAuthority · 66 BatchTradeNoCpi · 67 BatchTradeCpi · 68 SetMatcherConfig · 69 RestartAssetOracle.

### Wire-format adoptions (match toly by content)
- Tag 32: 34B `{kind,new_pubkey}` → **33B `{new_pubkey}`** (drop kind byte).
- Domain field u8→u16 on tags 24/50/51/52/53/56 (`0f87dcb`).
- Tag 57: `WithdrawInsuranceDomain{domain:u8,amount}` → `WithdrawInsuranceAsset{asset_index:u16,amount}`.
- Batch wire: tag 66 = `[66][u8 leg_count][leg_count × (u16 asset_index, i128 size_q SIGNED LE, u64
  exec_price, u64 fee_bps)]`; tag 67 = `[67][u8 leg_count][leg_count × (u16 asset_index, i128 size_q
  SIGNED LE, u64 fee_bps, u64 limit_price)]`. NoCpi legs <= max_portfolio_assets; CPI legs <= 16.

### RESULTING v17 FORK TAG MAP
`0-69` = toly frozen exact · `72/73` = NFT-B3 (kept) · `74-80` = LP-vault (renumbered) · `23/33` freed ·
tag 32 loses kind byte. Tags 70/71 + 81-88 left FREE as a buffer for toly's next claims.

> **Decision recorded:** the recon concerns offered two LP-vault targets (74-80 vs 80-86). This synthesis
> ADOPTS **74-80** (the batch-trade + auth concerns' explicit recommendation; tightest contiguous block
> above toly's max-used 69, leaving 81-88 as a clean buffer). The OLD-PIN `80-86` proposal is superseded —
> 74-80 leaves a wider future-toly buffer above the LP block. Confirm no client hardcodes 70-73.

---

## (f) ABI / LAYOUT DELTA → Phase-6 CLIENT-REGEN SPEC

> All sizes measured by compiling the real crates (per recon), not estimated. Standing decision (4):
> fresh-start cutover — NO in-place migration; every off-chain decoder/init/rent path regenerates.

### Account layout deltas (on-chain ABI breaks)
| Account | Old (fork) | New (frozen) | Δ | Source | Client impact |
|---|---|---|---|---|---|
| `WrapperConfigV16` | 624 | **432** | −192 | `792256b` | Drop 6 authority keys (admin→marketauth net −6×32); ALL interior config offsets (fees/oracle/EWMA) shift — re-derive from 432-byte layout. |
| `AssetOracleProfileV16` | 368 | **400** | +32 | `f64b7ee` | +`asset_admin:[u8;32]` trailing (`:762`), bootstrapped to marketauth (`:1409`). `ASSET_ORACLE_WRAPPER_LEN` stays 512. |
| `MarketGroupV16HeaderAccount` | base | **+72** | +72 | `4bd3d79` | +5 aggregates after `pnl_matured_pos_tot` (`:4670-4674`). Shifts EVERY dynamic asset-slot offset +72 → full market-group account ABI break. A-6's 4 fields append AFTER these. |
| `PortfolioSourceDomainV16Account` | 192 | **196** | +4 | `945f2db` | +leading `domain:V16PodU32` tag (`:14470`). Full byte layout in recon. E6 2×V16PodU128 already present in both (`:14486-14487`). |
| `PortfolioAccountV16Account` | 2907 | **9227** | +6320 | `945f2db`+`58dc118` | +6272 inline `[PortfolioSourceDomainV16Account;32]` + 48 (3 residual u128 @14535-14537). source_domains now inline (no external slice). |
| Portfolio WIRE / on-chain | HEADER(16)+2907+(N×192 tail) | HEADER(16)+9227+matcher(104) | — | `945f2db`+`7144d9b` | **fund-critical.** Fixed **9347B** portfolio account (16+9227+104), independent of market slots. All keeper/SDK/indexer init paths switch from slot-derived size to the fixed constant. `PORTFOLIO_MATCHER_CONFIG_LEN=104` inline at `PORTFOLIO_ENGINE_ACCOUNT_LEN`. |
| `PortfolioV16View` | 2-field {header, source_domains slice} | 1-field {header} | — | `945f2db` | All `::new` call sites arity 2→1 (engine + wrapper). |
| `TradeRequestV16.size_q` | u128 (fork) | **i128** (frozen) | — | (frozen `:3166`) | SDK trade builders sign size_q. A-1 admit field must be RE-ADDED (frozen lacks it). |

### Engine Rust-API deltas (crate semver, not on-chain)
- Crate root exports 60 curated v16 symbols (frozen `lib.rs:52-73`) vs fork's blanket `pub use v16::*`.
  Any external consumer importing an off-allow-list `percolator::v16::X` breaks at compile. v17 re-adds
  fork-needed extras via a `fork-facade` feature re-export block (NOT blanket export).
- `wide_math` module flips public→crate-private (frozen `lib.rs:44-47`). Re-export under `fork-facade`.
- Trade API rename: `execute_*_in_place_not_atomic` → `execute_*_loss_stale_scoped_not_atomic` (`b75d352`).
- 6 new pub insurance/earnings APIs (`b01c8e0`) + `charge_account_backing_fee_not_atomic` = mandatory
  wrapper call surface. 6 helpers demoted pub→private (`b01c8e0`).

### Instruction-tag deltas (on-chain wire) — see §(e)
Tag 32 wire 34B→33B; new tags 65/66/67/68/69; tag 57 domain→asset_index(u16); domain u8→u16 on
24/50/51/52/53/56; tags 23/33 removed; LP-vault 65-71→74-80; NFT-B3 72/73 kept; const
`MIN_INSURANCE_WITHDRAW_FLOOR_UNITS` removed. Stake CPI wire 34B→36B (tag 65, kind 2→1, +asset_index u16).

### Account-list deltas
- CloseSlab: +secondary_vault_token@6, +secondary_dest_token@7 (when secondary_collateral_mint set; `03873e8`).
- Matcher TradeCpi/BatchTrade tail: dynamic start — 8 when LP (account_b) signs, 7 when unsigned
  (inline-config path); `MAX_MATCHER_TAIL_ACCOUNTS` unchanged.

### Phase-6 regen targets (cascade)
SDK (config/profile/portfolio/market-group decoders + offset tables; new tag builders + batch builders;
tag-32 wire shrink; LP-vault tag renumber; size_q i128), keeper (fixed portfolio size; CloseSlab account
list; matcher tail; fork-facade engine import), indexer (all account decoders + tag map), frontend +
mobile (LP-vault tag renumber; new auth/batch instructions). MATCHER_ABI_VERSION=3 unchanged.

---

## (g) STAKE-REDESIGN CONFIRMATION (D-STAKE-1)

**VERDICT: D-STAKE-1 is preservable at frozen — NO ESCALATION for the standing asset-0 design.** The
"no admin key drains insurance; PDA controls inflow + terminal-reclaim" invariant HOLDS at match-toly.

### Why it holds (the de-escalation)
The prior REVALIDATED doc escalated because marketauth could drain via the tag-57 shutdown-drain bypass
regardless of insurance_operator/asset_admin state. Toly's own `0cf5134` closed this **in-policy**:
`admin_shutdown_authorized = asset_index != 0 && shutdown_drain && live_authority_matches(&cfg.marketauth,
operator.key)` (frozen `~/toly-wrapper-frozen/src/v16_program.rs:8257-8259`, re-verified). **For asset 0
the marketauth shutdown-drain branch is UNREACHABLE.** toly documents this as its own invariant
(`README.md:460-461`, item 10). The `asset_admin`-burn carve-out survives (`:8840`: only
`ASSET_AUTH_ADMIN` may be burned to 0), and the PDA self-rotate escape survives
(`!admin_signed => expect_live_authority(current_value)`).

### The tag-65 CPI wire (re-confirmed at source)
The fork stake CPI is hard-pinned to the DELETED tag-32-with-kind 34B wire
(`~/percolator-stake/src/cpi.rs:131-132` `TAG_UPDATE_AUTHORITY=32` + `AUTHORITY_INSURANCE=2` + pubkey;
canary `:207-221`). At frozen tag 32 is marketauth-only (no kind) — this would **silently rotate the
governance key instead of insurance** if left unchanged. **fund-critical rewire required.**

**New wire (tag 65 UpdateAssetAuthority):** `push(65) + push_u16(asset_index=0) + push(kind=1
[ASSET_AUTH_INSURANCE]) + new_pubkey[32]` = **1+2+1+32 = 36 bytes** (was 34). Note kind value changes
**2 → 1** (toly per-asset INSURANCE=1; fork's old AUTHORITY_INSURANCE=2 == toly's INSURANCE_OPERATOR=2 —
a silent-misroute trap). 3-account list unchanged ([0]=current authority signer, [1]=new_authority signer
when new_pubkey!=0, [2]=market writable).

- **BIND** (stake tag 19): kind=1, asset_index=0, new_pubkey=vault_auth PDA. accounts[1]=vault_auth PDA
  co-signs via `invoke_signed`.
- **ROTATE** (stake tag 20): kind=1, asset_index=0, new_pubkey=new_target. accounts[0]=vault_auth PDA
  (CURRENT authority, `invoke_signed`); self-rotate path works because after asset_admin burn
  `admin_signed=false` → `expect_live_authority(current=insurance_authority, PDA)` matches.

### Operational sequence preserving no-admin-drain (Phase-4)
1. **Bind** via tag 65 kind=insurance to vault_auth PDA (was tag-32 wire).
2. **Burn asset_admin→0** via tag 65 kind=ADMIN(0), new_pubkey=[0;32] — STILL LEGAL (only admin burnable);
   locks out admin-rotation path A. No co-signer (new_pubkey==0 skips the co-sign block).
3. **DROP the old "burn insurance_operator" step — now ILLEGAL** (`:8840`). For asset 0 the marketauth
   operator-bypass is already unreachable (`:8257`); for nonzero assets rotate the operator to a controlled
   live key if used.
4. **Rotate** via tag 65 kind=insurance with the PDA self-signing — no-lockout escape survives.

### HARD design constraint (the only residual escalation trigger)
The de-escalation depends ENTIRELY on **binding asset_index==0**. If a future design binds a NONZERO
asset's insurance, the marketauth shutdown-drain re-opens for that asset (after timeout + empty) and a
fork-side guard returns. **Confirm the canonical insurance pool the stake vault flushes to (via
`cpi_top_up_insurance` tag 9) is asset 0.** Alternative (cleaner, diverges from toly by one guard): in
`handle_update_asset_authority`, when `kind==insurance && admin_signed`, ALSO require the current
insurance_authority holder to sign (PDA consent) — then step 2's burn becomes unnecessary. Decide in
Phase 3/4.

**Re: `0cf5134` inclusion in the frozen target — CONFIRMED.** `0cf5134` "Unify live insurance withdrawal
by asset" is present at frozen wrapper `0f87dcb` (it removed tags 23/33, changed tag 57, added the
asset_index!=0 gate). D-STAKE-1 is validated AGAINST it.

---

## (h) OPEN QUESTIONS + ESCALATIONS

### ESCALATION #1 — DISMISSED (false alarm; verified by execution lead 2026-06-07)
1. ~~Re-pin the frozen target because live toly tips are ahead on Findings C/D/E/F/G.~~ **FALSE — DISMISSED.**
   The synthesis mistook the stale detached HEADs of the local reference checkouts (`~/toly-percolator`@
   `b6e23b3`, `~/toly-percolator-prog`@`4ee339d`) for toly's live upstream tips. Verified: frozen engine
   `5c72af3` is **225 commits AHEAD** of `b6e23b3` and CONTAINS Findings C (`7188eec`), D (`b6e23b3`),
   E (`f9af174`), `0bee8ef`, `a57a408`; frozen wrapper `0f87dcb` is **327 commits AHEAD** of `4ee339d`
   and CONTAINS Findings F/G. The frozen target is complete and current as of the pin. **No re-pin; no
   cherry-pick needed — they are already in the target.** Honors the brief's "FROZEN for the program
   duration; do not chase." The fork's own W1–W5 Finding ports (engine `b8d4d78`/`b5924f2`/`90d2097`,
   etc.) converge cleanly into these same upstream Findings.

### fund-critical re-expression item (highest engineering risk, not an escalation)
2. **LP-vault redemption + resolved-payout-topup bare vault decrements** (rows 24, wrapper @6137/@11286)
   risk tripping `validate_header_aggregate_totals` (senior = c_tot+insurance+backing_provider_earnings_total
   must stay <= vault). PROVE redeemed atoms are pure junior/unliened backing principal, OR add a fork
   engine entry point analogous to `withdraw_domain_insurance_not_atomic`. Re-validate the 5ebd136
   dual-withdraw gate + OI-guard (Custom 37) against the unified `market_insurance_remaining_view` /
   `debit_market_insurance_budget_view` + `long_domain=asset_index*2` mapping.

### Material correction recorded (no longer open, but flagged for downstream agents)
3. **Matcher-auth = inline `PortfolioMatcherConfigV16` (7144d9b), NOT the `matcher-auth` PDA (8690121).**
   The OLD-PIN `collision_matrix.md` §G-1 and the fork-feature-collision-pass concern reference the
   abandoned separate-PDA form. Frozen reality: inline config + tag 68 SetMatcherConfig + dual
   signed/unsigned-LP tail. Adopt the inline form; keep matcher economics (vamm.rs) unchanged.

### Open questions to close in Phase-2/3
4. Per-harness audit of the fork's 113 `fn kani_` vs frozen 84: how many are pure runtime-exposure shims
   (deletable) vs genuine proofs needing ViewMut re-target. "~50 on runtime types" is the right ballpark.
5. Name-by-name content diff of the 99 fork-runtime method names absent from frozen ViewMut — separate
   renamed twins (`*_core_not_atomic` → `*_not_atomic`) from fork-only feature methods (LP-vault/NFT/batch)
   from naming-convention diffs.
6. Confirm the A-6 stress-envelope header fields are EXCLUDED from the rescan-equality set
   (frozen `:5183-5191`), else appending them trips the O(1) consistency check.
7. Confirm the zero-copy MarketView exposes the per-domain mutation points the 6 engine APIs assume
   (`backing_bucket_for_domain`, `domain_insurance_budget_spent`, `insurance_reservation_for_domain`)
   after runtime-vec drop.
8. Map which fork resolved-payout teardown paths change `resolved_payout_blocker_count` state and ensure
   they go through the engine hook that maintains the 5th aggregate (no fork direct-write site found yet).
9. Sparse-vs-dense equivalence: the fork's per-asset multi-domain claim accounting
   (`source_claim_market_id`/`source_lien_*`) must map cleanly onto the 32-slot sparse domain-tagged search
   without losing accounting; confirm no fork path expects >32 concurrently-occupied source domains
   (full → `Err(LockActive)`).
10. Confirm no off-chain decoder (keeper/SDK/indexer) reads the heap-vec runtime field names as `Vec`
    rather than the POD layout before the DROP lands; the standing decision regenerates clients but the
    drop must not silently break a decoder pre-regen.
11. Decide fork-facade re-export placement (crate-root curated `pub use v16::{…}` block vs
    `#[cfg(feature="fork-facade")] pub mod v16`); the cleaner mirror of toly is the curated block.
12. Confirm batch CPI calls the matcher per-leg with the unchanged ABI-v3 contract + inline matcher-config
    auth, so the matcher superset needs no per-leg logic change; verify the echo offset (bytes[56..64]) is
    read under BOTH tail layouts (signed [8..] / unsigned [7..]).
13. Stake bind asset-0 confirmation (the de-escalation depends on it; §(g)). Verify `cpi_top_up_insurance`
    targets domain/asset 0 and no frozen path hard-requires a non-zero asset_admin for asset 0 after burn.
