# Stake Insurance Authority — v17 Threat Model REVALIDATED
## Date: 2026-06-06
## Frozen target: wrapper e20a381 / engine 7fe89cc
## Scope: read-only source analysis; no code modified
## Supersedes prior analysis at: stake_threat_model.md (2026-06-04, origin/main tip 5081495)

---

## DECISION: ESCALATION

The asset_admin burn (Step 2 from the prior model) **still works** as a mechanism.
However, a **new, independent drain path exists that the burn does NOT close**: the
`verify_domain_withdrawal_preflight` `marketauth` fallback makes `marketauth` unconditionally
authorized to call tag 57 `WithdrawInsuranceDomain` at any time — even with asset_admin burned and
insurance_authority bound to the stake vault_auth PDA. The no-admin-drain invariant cannot be
preserved at match-toly without a fork-side guard or accepted trust assumption.

---

## Step 1 — Frozen target confirmation

| Artifact | Commit | Confirmed |
|---|---|---|
| Wrapper (toly-percolator-prog) | e20a381dffe25e642d275a84b65374b5425798b7 | YES — is current origin/main tip |
| Engine (toly-percolator) | 7fe89cccf5d5be06a2e3f4ae76bbf4fa2a0c8ba2 | YES — ancestor of origin/master tip 7fbc873 |
| c37d3e4 "Reject zero domain authority burns" | ancestor of e20a381 | YES |
| 087a404 "Reject market authority burns" | ancestor of e20a381 | YES |
| 03873e8 "Require secondary vault recovery on close slab" | ancestor of e20a381 | YES |

Engine origin/master tip after fetch: 7fbc873 (4 new commits ahead of 7fe89cc, all Kani-proof
strengthening only — no program logic changes).

---

## Step 2 — Authority-burn rejection commits

### c37d3e4 — "Reject zero domain authority burns"

File: `src/v16_program.rs`, inside `handle_update_asset_authority` (tag 65), after the
`current_value` match block.

**Exact new guard (e20a381:src/v16_program.rs:9002-9005):**

```rust
// Required domain authorities must stay live after activation. A zero insurance/backing/oracle
// authority can strand funds or oracle liveness during wind-down; only the cold-storage
// asset_admin may be intentionally burned.
if new_pubkey == [0u8; 32] && kind != ASSET_AUTH_ADMIN {
    return Err(PercolatorError::InvalidInstruction.into());
}
```

Where:
- `ASSET_AUTH_ADMIN = 0`
- `ASSET_AUTH_INSURANCE = 1`
- `ASSET_AUTH_INSURANCE_OPERATOR = 2`
- `ASSET_AUTH_BACKING_BUCKET = 3`
- `ASSET_AUTH_ORACLE = 4`

**Effect:** Burning to `[0u8; 32]` is REJECTED for kind=1 (insurance), kind=2 (insurance_operator),
kind=3 (backing_bucket), kind=4 (oracle). Burning to zero is PERMITTED ONLY for kind=0 (asset_admin).

**Precisely:**

| kind | Burn to zero | Rotate to non-zero PDA |
|---|---|---|
| ASSET_AUTH_ADMIN (0) | PERMITTED | PERMITTED |
| ASSET_AUTH_INSURANCE (1) | **REJECTED** | PERMITTED |
| ASSET_AUTH_INSURANCE_OPERATOR (2) | **REJECTED** | PERMITTED |
| ASSET_AUTH_BACKING_BUCKET (3) | **REJECTED** | PERMITTED |
| ASSET_AUTH_ORACLE (4) | **REJECTED** | PERMITTED |

### 087a404 — "Reject market authority burns"

File: `src/v16_program.rs`, inside `handle_update_authority` (tag 32), at the top of the handler.

**Exact new guard (e20a381:src/v16_program.rs:8937-8939):**

```rust
if new_pubkey == [0u8; 32] {
    return Err(PercolatorError::InvalidInstruction.into());
}
```

**Effect:** Burning `marketauth` to zero via tag 32 is unconditionally REJECTED. The old conditional
burn-guard (only-if-Live and permissionless-resolve+force-close not configured) is entirely replaced
by an unconditional rejection. `marketauth` rotation to a non-zero key (with new key co-signing)
remains PERMITTED.

**Consequence for the redesign:** The secondary mitigation suggested in the prior model ("burn
insurance_operator to zero") is now BLOCKED. You cannot burn insurance_operator to zero via tag 65
(c37d3e4 blocks it). You cannot use tag 32 to eliminate marketauth either (087a404 blocks it). These
are now hard in-toly constraints.

---

## Step 3 — Re-validation of the stake redesign

### 3A — Does the asset_admin burn still work?

YES. At e20a381:src/v16_program.rs:9003:
```rust
if new_pubkey == [0u8; 32] && kind != ASSET_AUTH_ADMIN {
    return Err(PercolatorError::InvalidInstruction.into());
}
```
`ASSET_AUTH_ADMIN (kind=0)` is explicitly exempted. Burning asset_admin to zero via tag 65 is still
permitted. The prior model's Step 2 sequence remains operational at the frozen target.

After Step 1 (bind insurance_authority → vault_auth_pda) + Step 2 (burn asset_admin → zero):
- `admin_signed` evaluates to false (`profile.asset_admin == [0u8;32]`).
- Only path B (self-rotation by current insurance_authority) can call tag 65.
- The vault_auth PDA is the only entity that can rotate insurance_authority via tag 65.
- Dead-end: no EOA revival is possible once asset_admin is zero.

**Tag 65 drain path: CLOSED by the two-step.**

### 3B — The tag 57 shutdown-drain path: NOT CLOSED

`handle_withdraw_insurance_domain` (tag 57) at e20a381:src/v16_program.rs:8233 calls
`verify_domain_withdrawal_preflight` with `authority_kind = DOMAIN_WITHDRAW_AUTH_INSURANCE`.

Inside `verify_domain_withdrawal_preflight` (e20a381:src/v16_program.rs:7669-7679):

```rust
let local_authorized = match authority_kind {
    DOMAIN_WITHDRAW_AUTH_INSURANCE => {
        live_authority_matches(&authorities.insurance_operator, authority.key)
    }
    ...
};
if !local_authorized && !live_authority_matches(&cfg.marketauth, authority.key) {
    return Err(PercolatorError::Unauthorized.into());
}
```

The preflight returns `Ok` if either `insurance_operator == operator` OR `marketauth == operator`.
This is NOT conditioned on shutdown. Then inside `handle_withdraw_insurance_domain` proper
(e20a381:src/v16_program.rs:8251-8261), there is the additional shutdown gate:

```rust
let shutdown_drain =
    live_domain_withdraw_health_or_shutdown_view(&cfg, &group, domain)?;
let admin_shutdown_authorized =
    shutdown_drain && live_authority_matches(&cfg.marketauth, operator.key);
if !local_authorized && !admin_shutdown_authorized {
    return Err(PercolatorError::Unauthorized.into());
}
```

**Reading both gates together:**

The preflight gate passes if `marketauth == operator` (unconditionally, no shutdown check).
The body gate passes if `insurance_operator == operator` OR (`shutdown_drain && marketauth == operator`).

For a marketauth-signed call where insurance_operator != marketauth:
- Preflight: `local_authorized = false`, `live_authority_matches(&cfg.marketauth, operator.key) = true` → preflight PASSES.
- Body: `local_authorized = false`, `admin_shutdown_authorized = shutdown_drain && true`.

So the body gate requires `shutdown_drain = true` for marketauth to succeed when not the insurance_operator.

**`shutdown_drain` source:** `live_domain_withdraw_health_or_shutdown_view` — this evaluates whether
the domain is retired/empty/matured. This is the "extraordinary condition" noted in the prior model.

**Critical update from e20a381:** The prior model's mitigation was "burn insurance_operator to a
controlled key or zero." Both paths are now BLOCKED:
- Burning insurance_operator to zero: REJECTED by c37d3e4 (kind != ASSET_AUTH_ADMIN).
- Marketauth can still call tag 57 during shutdown (shutdown_drain gate) regardless of what
  insurance_operator is set to, because the preflight itself has `|| live_authority_matches(&cfg.marketauth, operator.key)`.

**Can marketauth itself be burned?** No — 087a404 rejects `new_pubkey == [0u8; 32]` for tag 32
unconditionally. Marketauth can only be rotated to another live key.

**Can insurance_operator be rotated to a PDA (non-zero)?** Yes, tag 65 kind=ASSET_AUTH_INSURANCE_OPERATOR
to a non-zero target is still permitted. But this only closes the `local_authorized` path. The
`marketauth` fallback in the preflight + the shutdown-drain body gate are entirely independent of
insurance_operator.

**Conclusion:** The marketauth + shutdown-drain path to drain insurance via tag 57 CANNOT be closed
at match-toly. After asset_admin is burned and insurance_authority is PDA-bound, the marketauth
holder still retains the ability to drain insurance via tag 57 once the domain reaches the retired/
empty/matured state.

### 3C — Does 03873e8 change the tag-57 shutdown-drain surface?

03873e8 modifies `handle_close_slab` (the market-teardown handler), adding mandatory secondary vault
recovery: it now requires both primary and secondary collateral token accounts to be passed and
swept before zeroing the market account. This is about ensuring vault funds are recovered at slab
close and preventing stranded secondary collateral.

It does NOT touch `handle_withdraw_insurance_domain`, `verify_domain_withdrawal_preflight`, or any
authority gate. The tag-57 shutdown-drain surface is unchanged by 03873e8.

### 3D — Summary of drain paths at frozen target e20a381

| Path | Gate | Status after bind+burn-admin |
|---|---|---|
| Tag 65 path A (asset_admin rotate insurance_authority to EOA) | admin_signed check | CLOSED — admin burned |
| Tag 65 path B (self-rotate by insurance_authority holder) | PDA must invoke_signed | CLOSED — PDA controls |
| Tag 41 terminal-reclaim | insurance_authority strict match | CLOSED — PDA controls |
| Tag 9 inflow (top_up_insurance) | insurance_authority gate | CLOSED — PDA controls |
| Tag 57 insurance_operator path | insurance_operator match | Closeable via rotate-to-PDA |
| Tag 57 marketauth shutdown-drain | shutdown_drain && marketauth | **OPEN — cannot close at match-toly** |

---

## DECISION (final)

**ESCALATION.**

The asset_admin burn survives at e20a381 and the tag 65 drain path is closeable. However, the
tag 57 `WithdrawInsuranceDomain` shutdown-drain path (`marketauth` + `shutdown_drain`) cannot be
closed at match-toly because:
1. marketauth cannot be burned (087a404 rejects zero burns on tag 32).
2. insurance_operator cannot be burned (c37d3e4 rejects zero burns for kind != ASSET_AUTH_ADMIN).
3. The `verify_domain_withdrawal_preflight` unconditional `marketauth` fallback (line 7677) is
   baked into toly's policy — marketauth is always a valid preflight signer.

The invariant "no admin key can drain insurance" is NOT fully preserved at match-toly without one
of the following:

### Option A — Fork-side wrapper guard (diverges from toly by one check)

In a fork of `handle_withdraw_insurance_domain`, add an additional gate before the
`admin_shutdown_authorized` branch: require that the PDA-controlled `insurance_authority` co-signs
the shutdown-drain, or require that the `marketauth` shutdown-drain path is explicitly disabled per
market via a config flag. This is a one-check divergence from toly that preserves the operational
guarantee.

**Recommendation: implement Option A** as a wrapper-side guard in the fork's v16_program.rs. The
guard is minimal: before the `admin_shutdown_authorized` check in `handle_withdraw_insurance_domain`,
add `if admin_shutdown_authorized { return Err(PercolatorError::Unauthorized.into()); }` when
`group.header.insurance_lock` (or equivalent stake-bound flag) is set. Alternatively, simply require
the vault_auth PDA to co-sign on the shutdown-drain path when insurance_authority is a PDA.

### Option B — Accept a documented trust assumption

Accept that after market wind-down (domain retired/empty/matured), marketauth can drain the residual
insurance balance. Document this in the stake program's security model as: "once the domain is fully
closed, the insurance residual is recoverable by marketauth; the stake PDA's exclusive control
applies only during the live operational phase." This weakens the formal guarantee to "no drain while
live" rather than "no drain ever."

### Option C — Rotate marketauth to a multi-sig before the bind

Since marketauth can only rotate to a non-zero live key (087a404), rotate it to a Squads multi-sig
before performing the stake bind. The shutdown-drain then requires multi-sig consent. This is
operational rather than on-chain enforceable, but raises the bar from single-key EOA to multi-sig.

**Recommendation: Option A is the cleanest for on-chain proof. Option C is the lowest-effort
operational mitigation if a code divergence is unacceptable. Option B should be explicitly rejected
as it leaves a single-key drain path open during a period (wind-down) when user funds may still be
at risk.**

---

## Wire/handler changes required at the frozen target (regardless of ESCALATION resolution)

Even if Option A/B/C is chosen, the following changes are required to align the stake CPI surface
with e20a381:

1. **Stake `BindInsuranceAuthority` (tag 19) CPI must target tag 65, not tag 32.**
   Tag 32 `handle_update_authority` now rotates only `cfg.marketauth`. Per-asset insurance_authority
   is exclusively via tag 65 `handle_update_asset_authority` with `kind=ASSET_AUTH_INSURANCE`.
   The CPI must pass `asset_index` and `kind=1` in addition to the new pubkey.

2. **Stake `RotateInsuranceAuthority` (tag 20) must use tag 65** for the same reason.

3. **Burn of asset_admin must be its own explicit CPI step** (tag 65, kind=ASSET_AUTH_ADMIN,
   new_pubkey=[0u8;32]). No co-signer is required for a burn. This step must be sequenced AFTER the
   insurance_authority bind (otherwise admin_signed=true means no self-rotation-only guarantee exists
   yet).

4. **insurance_operator rotation** (to a controlled non-zero key) must use tag 65 kind=ASSET_AUTH_INSURANCE_OPERATOR.
   Burning to zero is REJECTED (c37d3e4). Rotate to a vault_auth-controlled key or a separate
   governance key.

---

## Source references (e20a381)

| Claim | Source |
|---|---|
| ASSET_AUTH_* constants | `src/v16_program.rs:4667-4671` |
| c37d3e4 burn guard | `src/v16_program.rs:9002-9005` |
| handle_update_authority zero-burn rejection (087a404) | `src/v16_program.rs:8937-8939` |
| handle_update_asset_authority full handler | `src/v16_program.rs:8954-9021` |
| verify_domain_withdrawal_preflight marketauth fallback | `src/v16_program.rs:7677-7679` |
| handle_withdraw_insurance_domain shutdown_drain gate | `src/v16_program.rs:8251-8261` |
| 03873e8 secondary vault recovery (handle_close_slab only) | `src/v16_program.rs:8405-8500` |
| admin_signed path A gate | `src/v16_program.rs:8985-8988` |
| live_authority_matches rejects zero | `src/v16_program.rs:~10421-10423` |
