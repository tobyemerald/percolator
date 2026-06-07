# v17 Convergence — REFRESHED Fork-Feature × Toly-Refactor COLLISION MATRIX

> Standalone clean table. Companion to `v17_delta_recon.md`. Built from 9 source-grounded concern recons,
> re-verified at `file:line` 2026-06-07 against FROZEN engine `5c72af3` (`~/toly-engine-frozen/src/v16.rs`)
> + FROZEN wrapper `0f87dcb` (`~/toly-wrapper-frozen/src/v16_program.rs`); fork engine `~/percolator`
> @`07fe138`, fork wrapper `~/percolator-prog` @`07fe138`. Supersedes the OLD-PIN `collision_matrix.md`.
>
> **Severity:** `fund-critical` = silent drain / lockout / aggregate-drift risk if mishandled ·
> `breaking` = compile/decode/ABI break needing explicit re-expression · `mechanical` = rename / renumber /
> one-clause / verbatim port · `none` = survives clean.
>
> **Axis columns** = the v17 refactor each fork feature is checked against:
> `sparse` (945f2db) · `O(1)` (4bd3d79/b01c8e0) · `auth+burn` (792256b/f64b7ee/c37d3e4/087a404) ·
> `drop-rv` (drop-runtime-vec f26a6f0) · `batch` (0fa25bd/05e6e98) · `api-hide` (993e0cd…/24d334d/b75d352) ·
> `tag-renum` (tag-space) · `new-fn` (new functional: 58dc118/a57a408/0cf5134/cc91b07/03873e8/7144d9b).

## ENGINE FORK FEATURES

| # | Fork feature (loc) | sparse | O(1) | auth+burn | drop-rv | batch | api-hide | tag-renum | new-fn | Sev | Resolution |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Runtime heap-Vec `PortfolioAccountV16`@2661 + `MarketGroupV16`@2805 + impls | — | — | — | DELETED (frozen has none) | — | — | — | — | mechanical | DELETE defs + impls (255+85 methods); View/ViewMut twin already exists. |
| 2 | Heap-vec converters ×11 (gated) | — | — | — | no frozen analog | — | — | — | — | mechanical | DELETE all 11; keep the 12 ungated VALUE-codec pairs. |
| 3 | `fork_facade` mod (26 fns @22172) | reads source_domains | reads aggs | — | imports heap types | — | private in frozen | — | — | breaking | RE-EXPRESS to POD/View refs; re-lift under `fork-facade`. Wrapper-facing break. |
| 4 | `account_equity` family + A-4 lifts (@21679/21689 + ~44 fns) | — | agg consumers | — | heap arg + gate | — | PRIVATE in frozen | — | — | breaking | RE-EXPRESS arg→POD/parts (`account_equity_from_parts`@14663); re-lift verbatim. |
| 5 | ~85 kani_ shims in impl MarketGroupV16 + ~28 free (113 vs frozen 84) | — | — | — | runtime-gated | — | — | — | — | mechanical | REBUILD onto ViewMut; drop ~29 runtime-exposure shims. |
| 6 | Runtime shape/capacity validators (~33 hits) | obviated by sparse | — | — | runtime-only | — | — | — | — | mechanical | DELETE; use `source_domain_slot[_or_insert]`/`compact` (CAP=32). |
| 7 | Cargo.toml runtime-vec feature | — | — | — | removed by f26a6f0 | — | — | — | — | mechanical | Mirror toly: delete feature/mappings, `fuzz=[]`. |
| 8 | A-1 admit-threshold (`TradeRequestV16` field; h_lock_lane gate) | header path | re-validate fires pre-mutation | — | drop runtime twin | **per-leg in batch loop** | — | — | size_q i128 vs u128 | breaking | RE-EXPRESS: re-add `Option<u128>`; gate on ViewMut + per-leg; reconcile size_q. |
| 9 | A-4 pub-lifts ×8 in-place | — | agg readers | — | rebuild on ViewMut | — | re-lift behind fork-facade | — | — | breaking | RE-LIFT verbatim under `fork-facade` on frozen private bodies (already take View). |
| 10 | A-6 stress-envelope (4 fields; POD @4398-4407) | header POD | **APPEND after 5 aggs; EXCLUDE from rescan set @5183** | — | drop runtime writer | — | — | — | — | breaking | REBUILD + layout-reconcile. |
| 11 | A-9 fee mutator (`FeePolicyUpdateV16` 4 fields + 3 Kani) | header scoped | route fee→insurance thru agg API | — | drop runtime mutator | — | — | **new tag (renum range)** | toly mutators 37/49/51/55 narrower → not obviated | breaking | RE-EXPRESS as new tag → config writer; keep ViewMut mutator; rewrite proofs. |
| 12 | A-10 max_price_move upper cap (`> MAX_MARGIN_BPS`) | — | — | — | zero-copy ok | — | — | — | field CONVERGED (@1462+`==0`@1950) | mechanical | Re-insert ONE clause in frozen validate. |
| 13 | engine `lp_vault` mod (@22482; primitive math) | no touch | pure math | — | survives clean | — | reaches `crate::wide_math` (crate-visible) | — | absent in frozen | none | SURVIVES-CLEAN: re-add under fork-facade, byte-identical; re-run Kani. |
| 14 | E6 capital-at-risk (2 V16PodU128; Vec mirror + POD) | rides inline sparse POD (+32B carrier) | agg no double-count | — | drop ONLY Vec mirror | — | — | — | **CONVERGED** (a57a408; frozen @14486-14487) | mechanical | Adopt toly POD; drop Vec mirror + Kani twin; verify byte-equality. |
| 15 | Residual reward counters (3 u128, portfolio-level) | rides header | non-margin | — | engine-owned, auto on drop | transfer-on-trade | — | — | **MISSING in fork** (58dc118) | breaking | ADOPT: +3 fields (+48B) + record machinery + spent<=crystallized guard. |

## WRAPPER + SATELLITE FORK FEATURES

| # | Fork feature (loc) | sparse | O(1) | auth+burn | drop-rv | batch | api-hide | tag-renum | new-fn | Sev | Resolution |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 16 | Portfolio deserializers (`portfolio_source_domain_*` + dynamic 2N tail) | frozen = single inline blob | — | — | runtime hydration drops | — | — | — | — | breaking | DELETE tail helpers + split builder; adopt frozen single-blob `portfolio_wire` + one-arg `ViewMut::new`. |
| 17 | Portfolio account rent/size (`PORTFOLIO_ACCOUNT_LEN` + 2N tail) | fixed 9347B inline vs dynamic | — | — | — | — | — | — | +104B inline matcher cfg | **fund-critical** | Hard byte-size break; fresh-start cutover; fixed const HEADER(16)+9227+104; verify init zeroing. |
| 18 | Local budget helpers `set_domain_budget_view`/`add_to_domain_budget_view`@4616-4652 | per-domain | bypass `insurance_domain_budget_remaining_total` | — | mutate Vec | — | — | — | — | **fund-critical** | DELETE; route thru `credit_domain_insurance_budget_not_atomic`@7285. |
| 19 | Fee-credit helpers (`credit_market_insurance_budget_view`@4664/`credit_fee_to_domain_budget_view`@4804 +2; hot path @7139) | per-domain | DoS hot path | — | Vec mirror | per-leg in batch | — | — | — | **fund-critical** | Thin wrappers over `credit_domain_insurance_budget_not_atomic`; preserve `fee_redirect_to_market_0_bps`. |
| 20 | `handle_withdraw_insurance_domain` inline triple @8978-9001 | — | bypass budget total | tag 57→Asset(u16) | — | — | — | tag 57 wire | 0cf5134 | **fund-critical** | Replace with `withdraw_domain_insurance_not_atomic`@7378; keep ledger sync post-step. |
| 21 | `handle_withdraw_insurance/_limited` (terminal/market-wide) | — | bypass | tag 23 DELETED | — | — | — | — | 0cf5134 | **fund-critical** | Loop per-domain debit thru `withdraw_domain_insurance_not_atomic`. |
| 22 | Crank backing-fee split (`header.insurance+=`; bucket+=; budget; @12096 + @11526) | — | bucket/total maint | — | zero-copy crank | — | — | — | one legal inline `header.insurance+=` | **fund-critical** | provider→`credit_backing_provider_earnings_not_atomic`@6547; budget→`credit_domain_insurance_budget_not_atomic`; KEEP single inline. |
| 23 | LP-vault DepositToLpVault vault increment @5665-5685 | — | relaxes senior<=vault; 4 totals untouched | — | re-verify zero-copy | — | — | renum | — | mechanical | Keep direct vault write (SAFE); verify bypasses `utilization_fee_earnings`. |
| 24 | LP-vault ExecuteRedemption vault decrement @6137 + resolved-payout topup @11286 | — | **bare vault− risks senior<=vault** | — | — | — | — | renum | — | breaking | **FLAG:** prove redeemed atoms junior/unliened OR add fork engine entry point. Highest-risk re-express. |
| 25 | Insurance top-ups @7765/@7906 | — | budget total | tag 56 u16 | — | — | — | — | 0f87dcb u16 | breaking | Route thru `deposit_domain_insurance_not_atomic`@7298; backing-bucket top-up @8378 stays vault-only. |
| 26 | Crank reward to insurance @9460/9522 + @11526 | — | unbudgeted credit safe; payout must respect budget | — | zero-copy crank | — | — | — | — | breaking | Per-site: credit=safe; payout→`credit_account_from_insurance_not_atomic`@7458. Cranker reward = fork KEEP. |
| 27 | WrapperConfigV16 7 authority keys | — | — | **COLLAPSED→marketauth (624→432)** | — | — | — | — | 792256b | breaking | ADOPT collapse; re-express checks → marketauth / per-asset profile (`domain_authorities_from_profile`@7657-7670). |
| 28 | AssetOracleProfileV16 +asset_admin (368→400) | — | — | per-asset cold-storage admin | — | — | — | — | f64b7ee | breaking | ADOPT +32B; bootstrapped to marketauth (@1409). |
| 29 | Fork tag 32 `UpdateAuthority{kind,new_pubkey}` (7-kind enum) | — | — | tag 32→marketauth-only (no kind, burn rejected @8774) | — | — | — | same-tag/diff-wire | 792256b/087a404 | breaking | DROP kind byte; adopt toly 32; map rotations onto tag 65 (insurance=1 not fork's 2). |
| 30 | NEW tag 65 UpdateAssetAuthority (fork lacks) | — | — | per-asset, 5-kind (@4661-4665), burn carve-out @8840 | — | — | — | — | f64b7ee/c37d3e4 | adopt | ADOPT verbatim; host for D-STAKE-1. |
| 31 | LP-vault tags 65-71 (@2803-2820) | reads source-credit/backing ledger | RE-EXPRESS thru agg APIs | gate→marketauth/profile | rebuild zero-copy | LP = batch counterparty | — | **collide 65-69** | — | **fund-critical** | RENUMBER 65-71→**74-80**; route all writes thru agg APIs (rows 18-26); gate Create on marketauth. |
| 32 | LP-vault dual withdraw-gate (5ebd136; @6102-6133) | source_after on sparse | re-validate vs O(1) | — | rebuild | — | — | — | cc91b07 lag gate | breaking | Re-attach `expected_source_credit_rate_num`@7952; ADD `reject_exposed_target_effective_lag_view`. |
| 33 | NFT-B3 tags 72/73 | owner+provenance on sparse header | — | SetNftProgramId admin→marketauth | — | — | — | **FREE at frozen (top=69)** | — | mechanical | KEEP at 72/73; re-base SetNftProgramId(73) gate→marketauth; NFT CPI tag 72 lockstep. |
| 34 | B-11 oracle staleness cap (`MAX_ORACLE_STALENESS_SECS=86400`; @1057/1179) | — | — | re-apply atop restructured config/profile | — | — | — | — | cc91b07 DISTINCT/complementary | mechanical | RE-APPLY `> MAX` clause (frozen has `==0`@1163/1284, no cap); cc91b07 adopted separately. |
| 35 | Fork tags 23 WithdrawInsuranceLimited + 33 UpdateInsurancePolicy | — | — | old global-insurance model | — | — | — | freed | DELETED by 0cf5134 | breaking | REMOVE both; re-express intent on per-asset auth; frees 23/33. |
| 36 | Fork tag 57 `WithdrawInsuranceDomain{domain:u8}` + u8-domain on 24/50/51/52/53/56 | — | per-asset budget view | — | — | — | — | wire change | 0cf5134/0f87dcb | breaking | ADOPT WithdrawInsuranceAsset{asset_index:u16}; domain u8→u16 everywhere. |
| 37 | Wrapper trade calls + manual loss-stale toggle (@7111-7125; calls @7116/7120/7483/7487) | — | — | — | — | scoped API absorbs toggle | removed trade API (b75d352) | — | — | breaking | ADOPT `execute_*_loss_stale_scoped_not_atomic`; DELETE toggle (engine @12459-12468). |
| 38 | Wrapper wide_math cross-crate calls (7 sites @4253/4322/7963) | — | — | — | — | — | `wide_math` privatized (24d334d) | — | — | breaking | Re-export under `fork-facade` (mirror toly kani pattern) OR move helpers into engine as proven pub APIs. |
| 39 | Wrapper TokenValueFlowProofV16 ctor @11291 | — | — | — | — | — | kani-only (2fa4561) | — | — | breaking | Re-express onto public ResolvedClose/ResolvedPayout API; re-lift only if no public seam. |
| 40 | Wrapper has_pending_residual call @11737 | — | — | — | reads `try_to_runtime` | — | demoted private (44cfd06) | — | — | mechanical | Inline over public CloseProgressLedgerV16 fields; read POD directly. |
| 41 | Crate public API (blanket `pub use v16::*`) | — | — | — | — | — | frozen = 60 curated symbols | — | — | breaking | ADOPT frozen lib.rs (private mod + allow-list) + `fork-facade` re-export block; no blanket. |
| 42 | percolator-match superset (vAMM, asset_index echo @56, ABI v3) | — | — | adopt inline matcher-cfg dual tail | — | matcher per-leg | — | — | inline `PortfolioMatcherConfigV16` (7144d9b) | mechanical | KEEP logic (decision 4); ADOPT inline cfg + tag 68 + dual signed[8..]/unsigned[7..] tail. NOT the abandoned separate-PDA form. |
| 43 | percolator-stake Bind/Rotate (tags 19/20; CPI tag32+kind=2 34B) | — | — | tag32 marketauth-only; per-asset→tag65; `0cf5134` asset_index!=0 gate | — | — | — | CPI re-target | 792256b/087a404/0cf5134 | **fund-critical** | REWIRE→tag 65 {asset0,kind=1,pubkey} 36B; canary @207-221; D-STAKE-1 (see §g). |
| 44 | percolator-nft program (Token-2022 hook, NftRegistry) | — | — | NftRegistry unaffected | — | — | — | depends on wrapper id stability | — | mechanical | KEEP; re-register via tag 73 if wrapper id changes; CPI tag 72 lockstep. |
| 45 | CloseSlab handler (single primary vault drain) | — | — | — | — | — | — | account-list +2 | 03873e8 | mechanical | ADOPT secondary-vault drain+close (@8369-8447); removes `verify_withdrawable_vault_token_account`; +secondary_vault@6/dest@7. |
| 46 | Wrapper portfolio +residual mirror + inline matcher cfg (layout) | rides zero-copy portfolio | — | — | residual rides auto | — | — | — | 58dc118/0f87dcb/7144d9b | breaking | ADOPT residual +48B (auto) + inline matcher +104B (at PORTFOLIO_ENGINE_ACCOUNT_LEN); reconcile order; regen decoders. |

## COLLISION COUNTS BY SEVERITY
- **fund-critical: 9** — rows 17, 18, 19, 20, 21, 22, 31, 43 (8 distinct) + the §(h) escalation re-pin
  (Findings C/D/E/F/G ahead of frozen). [Strict matrix-row count = 8 fund-critical.]
- **breaking: 21** — rows 3, 4, 8, 10, 11, 15, 16, 24, 25, 26, 27, 28, 29, 32, 35, 36, 37, 38, 39, 41, 46.
- **mechanical: 13** — rows 1, 2, 5, 6, 7, 12, 14, 23, 33, 34, 40, 42, 44, 45 (14 listed; row 30 is "adopt").
- **none: 1** — row 13 (engine lp_vault).
- **adopt-verbatim (toly-owned, fork lacks): 1** — row 30 (tag 65); plus the broader ADOPT set
  (sparse, O(1), batch, residual, E6-converged, auth collapse, 0cf5134, cc91b07, 03873e8, inline matcher cfg).

## TAG RENUMBER MAP (summary)
toly frozen `0-69` adopted verbatim (top used 69). Fork: `23`/`33` REMOVED · tag `32` loses kind byte ·
`57`→WithdrawInsuranceAsset(u16) · LP-vault `65-71` → **`74-80`** · NFT-B3 `72/73` KEPT · ADD toly
`65/66/67/68/69`. Tags `70/71`+`81-88` left FREE as toly-buffer. Stake CPI wire `34B`→`36B` (tag 65,
kind `2`→`1`, +asset_index u16).
