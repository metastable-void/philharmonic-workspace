# Gate-2 review outcome — `philharmonic-policy` Wave 2 + auth-boundary

**Date:** 2026-04-22

## Condition of Gate-2 satisfied

Yuka's Gate-2 approval at
`docs/design/crypto-approvals/2026-04-21-phase-2-sck-and-pht-01.md`
is conditional on Claude Code performing one more review pass
over the whole affected codebase. This note documents that pass.

**Scope covered** (every file touched by Wave 2 + the
auth-boundary hardening + the panic hardening):

- `philharmonic-policy/src/sck.rs` — SCK AES-GCM primitives.
- `philharmonic-policy/src/token.rs` — `pht_` token generation
  and parsing.
- `philharmonic-policy/src/evaluation.rs` — authorization
  evaluator with the Finding #1 role-tenant check.
- `philharmonic-policy/src/permission.rs` — permission document
  parser with the Finding #2 atom validation.
- `philharmonic-policy/src/entity.rs` — TenantEndpointConfig,
  subdomain-name validation, entity-kind definitions.
- `philharmonic-policy/src/error.rs` — error variant list.
- `philharmonic-policy/src/lib.rs` — re-export surface.
- `philharmonic-policy/Cargo.toml` — dependency list.
- `philharmonic-policy/tests/crypto_vectors.rs` — Rust hex
  constants.
- `philharmonic-policy/tests/crypto_vectors/gen_sck.py` and
  `gen_pht.py` — Python reference scripts (regenerated during
  this pass; byte-for-byte match against the committed Rust
  hex constants using pyca `cryptography 41.0.7`).

## Verdict

**No blocking findings.** Construction is correct, vectors
match the external Python reference, side-channel hygiene
(opaque `SckDecryptFailed`) is in place, no `unsafe`, no
`anyhow`, no `println!` / `eprintln!` / `tracing` in library
code. Miri is clean across the affected tests with both
yesterday's and today's nightly.

Findings below are hardening recommendations and documentation
observations — none prevent publish at `0.1.0`.

## Hardening recommendations (non-blocking, Yuka's call)

### H1 — `token.rs` stack-copy zeroization gap (medium)

[philharmonic-policy/src/token.rs:19-23](../../philharmonic-policy/src/token.rs#L19-L23):

```rust
pub fn generate_api_token() -> (Zeroizing<String>, TokenHash) {
    let mut raw = [0_u8; TOKEN_BYTES];
    fill_random(&mut raw);
    let generated = generate_api_token_from_bytes(raw);
    raw.zeroize();
    generated
}
```

Because `[u8; 32]` is `Copy`, `generate_api_token_from_bytes(raw)`
passes a **copy** of the 32 random bytes into the callee's
stack frame. The callee's parameter is a plain `[u8; 32]` —
not wrapped in `Zeroizing`. When the callee returns, its stack
slot retains the bytes until overwritten. The
`raw.zeroize()` on line 22 only clears the caller's slot, not
the callee's.

These 32 bytes are effectively the `pht_` token's credential
material — the base64url encoding of the same bytes becomes
the public part of the token. An attacker who can read
post-return stack memory of `generate_api_token_from_bytes`
would reconstruct the token.

**Narrow exploit window**: stack slots get overwritten
quickly by subsequent function calls, and reading
post-return stack memory requires either a kernel bug or a
co-located process with memory-read ability — both of which
imply worse capabilities anyway. Defense-in-depth concern,
not an immediate threat.

**Fix recommendation**: change `generate_api_token_from_bytes`
to take `&[u8; 32]` by reference. Zero copies, one-character
change at each call site (add `&`). Cleanest API too.

### H2 — `sck.rs` `from_file` stack-copy zeroization gap (low)

[philharmonic-policy/src/sck.rs:31-43](../../philharmonic-policy/src/sck.rs#L31-L43):

```rust
pub fn from_file(path: &Path) -> Result<Self, PolicyError> {
    let bytes = Zeroizing::new(std::fs::read(path)?);
    if bytes.len() != SCK_KEY_LEN { return Err(...); }
    let mut key = [0_u8; SCK_KEY_LEN];
    key.copy_from_slice(bytes.as_slice());
    Ok(Self::from_bytes(key))
}
```

Same class as H1. `key` at line 40 is a plain `[u8; 32]`,
not `Zeroizing`. `Self::from_bytes(key)` copies the bytes
into `from_bytes`'s parameter before wrapping in `Zeroizing`.
The original `key` local and the `from_bytes` parameter both
live on the stack unprotected during the narrow transfer.

SCK is the long-lived endpoint-config encryption key — if
leaked, every `TenantEndpointConfig.encrypted_config` in the
substrate becomes decryptable. Higher-stakes material than
`pht_` tokens, but the exploit window is the same narrow
kernel-read-or-colocated-process scenario.

**Fix recommendation**: restructure `from_file` to construct
`Sck` directly without going through `from_bytes`:

```rust
let mut key = Zeroizing::new([0_u8; SCK_KEY_LEN]);
key.copy_from_slice(bytes.as_slice());
Ok(Self { key })
```

Drops the `Self::from_bytes(...)` call, but `key` is now
`Zeroizing<[u8; 32]>` from the start — zeroized on drop /
early-return / panic.

### H3 — `sck.rs` encrypt-error variant naming (cosmetic)

[philharmonic-policy/src/sck.rs:119](../../philharmonic-policy/src/sck.rs#L119)
maps `Aes256Gcm::encrypt` failures to
`PolicyError::SckDecryptFailed` (the "decrypt" variant reused
on the encrypt path). Unreachable in practice for the inputs
we pass (`aes-gcm`'s `encrypt` with `Vec<u8>` output is
effectively infallible).

Options:
- **(a) Leave it** — the path is unreachable; no runtime
  consequence.
- **(b)** Split to `PolicyError::SckEncryptFailed` — cleaner
  naming, costs one error variant.
- **(c)** Treat encrypt as infallible at the type level —
  `.expect("aes-gcm encrypt cannot fail for Vec<u8> output")`
  and drop the `Result` wrapper from the encrypt path.

My preference: **(c)** — the cleanest expression of the fact
that encrypt genuinely can't fail here, and matches the
narrow-exception pattern used for OS RNG in the same file.
**(a)** is also fine if you'd rather not touch the API.

## Documentation observations (existing, already flagged)

### D1 — Finding #2 error-roundtrip fragility

`permission.rs`'s `parse_permission_document` recovers the
typed `PolicyError::UnknownPermissionAtom { atom }` by
string-matching serde's custom error message. Documented in
note 0003; not revisiting here beyond confirming the
mitigation is private-scope (`UNKNOWN_PERMISSION_ATOM_PREFIX`
is a module-private const).

### D2 — Tenant status not checked during permission evaluation

[philharmonic-policy/src/evaluation.rs](../../philharmonic-policy/src/evaluation.rs)
checks the principal's tenant attribute and its `is_retired`
scalar, but does **not** load the `Tenant` entity itself to
check its `status` scalar (`Active` / `Suspended` / `Retired`).
A principal in a Suspended tenant would today have permissions
evaluated normally.

Two ways to read this:
- **Intentional** — tenant suspension is enforced at the API
  layer (Phase 8, `philharmonic-api`), and the evaluator
  stays focused on per-principal authorization. This would be
  consistent with the "layered ignorance" design principle.
- **Gap** — defense-in-depth says every permission eval
  should gate on tenant status too, belt-and-suspenders.

This is **Wave 1 code, not introduced by Wave 2 or the
auth-boundary fixes.** Flagging because I saw it on this
pass; your call whether to address now, address in Phase 8
alongside the API-layer enforcement, or document as an
intentional boundary.

### D3 — `parse_permission_document` test gap

`permission.rs` has unit tests for the Deserialize-level
rejection of unknown atoms, but no direct unit test that
`parse_permission_document` → `Err(PolicyError::UnknownPermissionAtom
{ atom: "totally:made_up" })` with the correctly-extracted
atom field. The contract is exercised indirectly through the
evaluator's call site, but a direct test would lock the
helper's behavior. Low priority.

## Test-vector cross-check result

Ran `python3 tests/crypto_vectors/gen_sck.py` and `gen_pht.py`
fresh against pyca `cryptography 41.0.7`. Output matches the
Rust hex constants in `tests/crypto_vectors.rs` byte-for-byte
across:

- SCK vector 1 (38-byte JSON plaintext, 67-byte wire)
- SCK vector 2 (28-byte Unicode JSON, 57-byte wire)
- SCK vector 3 (2-byte `{}`, 31-byte wire)
- `pht_` vector 1 (sequential 0x00..0x1f)
- `pht_` vector 2 (reverse-sequential 0xff..0xe0)
- `pht_` vector 3 (alternating 0xa5/0x5a)

Independent-implementation check: OpenSSL-backed AES-256-GCM
(pyca) agrees with RustCrypto `aes-gcm 0.10`. If either
implementation drifts from the committed hex, the Rust code
has a regression (per the vector discipline — don't re-generate
the hex to match a drift).

## Recommendation

Given the Gate-2 approval condition is satisfied and no
blocking issues were found, I think you have three reasonable
paths:

1. **Publish `philharmonic-policy 0.1.0` as-is.** The
   hardening items above are defense-in-depth, not
   correctness bugs. A patch release (`0.1.1`) later can
   address H1/H2.
2. **Apply H1 + H2 (zeroization hardening) now, then
   publish 0.1.0.** Small diffs (H1 is one-line API change;
   H2 is one-function restructure). Another miri pass to
   re-confirm, then publish.
3. **Apply H1 + H2 + H3, then publish.** Same as #2 plus
   the cosmetic encrypt-error cleanup — a fuller sweep
   before the first real publish.

My preference is **#2** — the two zeroization fixes are cheap
and strictly improve the initial release's security posture.
H3 is cosmetic and can wait. D1–D3 are documentation-level
and don't need code action now.

Awaiting your call before proceeding to publish.
