#!/usr/bin/env python3
"""Reference vector generator for Phase 5 Wave A COSE_Sign1 tokens.

Produces the CBOR-encoded claim payload and the final COSE_Sign1
bytes for a fixed known-answer input:

- Ed25519 keypair: RFC 8032 §7.1 TEST 1 (public, external reference).
- Claim set: deterministic constants matching the proposal's test
  plan (see docs/design/crypto-proposals/2026-04-22-phase-5-wave-a-
  cose-sign1-tokens.md §Test-vector plan).
- Payload: SHA-256("phase-5-wave-a-test-payload"), with the 27-byte
  plaintext also committed for reproducibility.

The outputs are the reference hex values that the Rust
implementations in philharmonic-connector-client (sign path) and
philharmonic-connector-service (verify path) must reproduce
exactly. Matching bytes = correct construction.

Run inside a venv with `cbor2`, `cose`, and `cryptography`:

    python3 -m venv /tmp/wave-a-vendor
    /tmp/wave-a-vendor/bin/pip install cbor2 cose cryptography
    /tmp/wave-a-vendor/bin/python docs/crypto-vectors/wave-a/gen_wave_a_vectors.py

The script writes vector files next to itself. The Rust tests
either include the hex strings as `hex!(...)` literals or load
the files via `include_bytes!` / `include_str!`.
"""

from __future__ import annotations

import hashlib
import os
import sys
from pathlib import Path

import cbor2
from cryptography.hazmat.primitives.asymmetric.ed25519 import (
    Ed25519PrivateKey,
    Ed25519PublicKey,
)
from cryptography.hazmat.primitives import serialization


# ---------------------------------------------------------------------------
# Inputs (keep in lockstep with the proposal's test-vector plan).

# RFC 8032 §7.1 TEST 1 — public reference vector.
SEED_HEX = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
PUBLIC_KEY_HEX = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"

# Claim-set values. Field order below must match the declaration
# order of `philharmonic_connector_common::ConnectorTokenClaims`
# (Rust struct field declaration order). ciborium+serde emit CBOR
# map entries in declaration order; cbor2 emits Python-dict key
# order. We pick the same order here so both byte streams match.
CLAIM_ORDER = [
    "iss",
    "exp",
    "kid",
    "realm",
    "tenant",
    "inst",
    "step",
    "config_uuid",
    "payload_hash",
]

ISS = "lowerer.main"
EXP_MILLIS = 1924992000000  # 2031-01-01T00:00:00Z
KID = "lowerer.main-2026-04-22-3c8a91d0"
REALM = "llm"
TENANT_UUID = "11111111-2222-4333-8444-555555555555"
INST_UUID = "66666666-7777-4888-8999-aaaaaaaaaaaa"
STEP = 7
CONFIG_UUID = "bbbbbbbb-cccc-4ddd-8eee-ffffffffffff"

# Payload plaintext (committed for reproducibility). The
# service-side verify hashes caller-supplied payload bytes; Wave A
# tests use arbitrary bytes and verify SHA-256 binding.
PAYLOAD_PLAINTEXT = b"phase-5-wave-a-test-payload"


# ---------------------------------------------------------------------------
# Helpers.


def uuid_bytes(hex_dashed: str) -> bytes:
    """Serialize a UUID as 16 raw bytes (matches ciborium's
    uuid::Uuid encoding when `is_human_readable() == false`)."""
    return bytes.fromhex(hex_dashed.replace("-", ""))


def encode_cbor_map_in_order(pairs: list[tuple[str, object]]) -> bytes:
    """Encode a CBOR map with deterministic key order matching the
    insertion order of `pairs`. cbor2's dict serialization preserves
    Python dict insertion order (Python 3.7+); we rely on that."""
    return cbor2.dumps(dict(pairs))


def build_claim_pairs() -> list[tuple[str, object]]:
    return [
        ("iss", ISS),
        ("exp", EXP_MILLIS),
        ("kid", KID),
        ("realm", REALM),
        ("tenant", uuid_bytes(TENANT_UUID)),
        ("inst", uuid_bytes(INST_UUID)),
        ("step", STEP),
        ("config_uuid", uuid_bytes(CONFIG_UUID)),
        (
            "payload_hash",
            hashlib.sha256(PAYLOAD_PLAINTEXT).digest(),
        ),
    ]


def sig_structure1(protected_bytes: bytes, payload_bytes: bytes) -> bytes:
    """Encode `Sig_structure1` per RFC 9052 §4.4.

    Sig_structure1 = [ context: "Signature1",
                       body_protected: bstr,
                       external_aad: bstr,
                       payload: bstr ]
    """
    return cbor2.dumps(
        ["Signature1", protected_bytes, b"", payload_bytes]
    )


def build_protected_header() -> bytes:
    """Encode the COSE_Sign1 protected header as a bstr-wrapped CBOR
    map {1: -8, 4: <utf8-bytes-of-kid>}."""
    header = {
        1: -8,                 # alg = EdDSA
        4: KID.encode("utf-8"),  # kid (byte string)
    }
    return cbor2.dumps(header)


def build_cose_sign1(protected_bytes: bytes, payload_bytes: bytes,
                     signature: bytes) -> bytes:
    """Encode the final COSE_Sign1 structure.

    COSE_Sign1 = [ protected: bstr,
                   unprotected: {},
                   payload: bstr,
                   signature: bstr ]

    We do NOT wrap the outer array in a CBOR tag (18 for
    COSE_Sign1) — the ConnectorSignedToken is a tag-less
    COSE_Sign1 per coset's default `from_slice` behavior.
    """
    return cbor2.dumps([
        protected_bytes,
        {},
        payload_bytes,
        signature,
    ])


# ---------------------------------------------------------------------------
# Main.


def main(outdir: Path) -> None:
    seed = bytes.fromhex(SEED_HEX)
    private_key = Ed25519PrivateKey.from_private_bytes(seed)

    # Derive and verify public key.
    public_bytes = private_key.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    expected_public = bytes.fromhex(PUBLIC_KEY_HEX)
    if public_bytes != expected_public:
        raise SystemExit(
            f"derived public key {public_bytes.hex()} does not match"
            f" RFC 8032 TEST 1 expected {PUBLIC_KEY_HEX}"
        )

    # Build and serialize the claim payload.
    claim_pairs = build_claim_pairs()
    payload_bytes = encode_cbor_map_in_order(claim_pairs)

    # Build the protected header.
    protected_bytes = build_protected_header()

    # Build Sig_structure1 and sign.
    to_be_signed = sig_structure1(protected_bytes, payload_bytes)
    signature = private_key.sign(to_be_signed)

    # Build the final COSE_Sign1.
    cose_sign1_bytes = build_cose_sign1(
        protected_bytes, payload_bytes, signature
    )

    # Write artifacts.
    outdir.mkdir(parents=True, exist_ok=True)
    artifacts = {
        "wave_a_seed.hex": SEED_HEX,
        "wave_a_public.hex": PUBLIC_KEY_HEX,
        "wave_a_payload_plaintext.hex": PAYLOAD_PLAINTEXT.hex(),
        "wave_a_payload_hash.hex":
            hashlib.sha256(PAYLOAD_PLAINTEXT).digest().hex(),
        "wave_a_claims.cbor.hex": payload_bytes.hex(),
        "wave_a_protected.hex": protected_bytes.hex(),
        "wave_a_sig_structure1.hex": to_be_signed.hex(),
        "wave_a_signature.hex": signature.hex(),
        "wave_a_cose_sign1.hex": cose_sign1_bytes.hex(),
    }
    for name, hx in artifacts.items():
        (outdir / name).write_text(hx + "\n")

    # Print a human summary to stdout so rerunning the script in a
    # terminal shows the values without opening the files.
    print("# Wave A reference vectors")
    print(f"seed                = {SEED_HEX}")
    print(f"public              = {PUBLIC_KEY_HEX}")
    print(f"payload plaintext   = {PAYLOAD_PLAINTEXT.hex()}")
    print(
        f"payload hash        = "
        f"{hashlib.sha256(PAYLOAD_PLAINTEXT).digest().hex()}"
    )
    print(f"claims CBOR ({len(payload_bytes):>3d} B) = {payload_bytes.hex()}")
    print(
        f"protected  ({len(protected_bytes):>3d} B) = "
        f"{protected_bytes.hex()}"
    )
    print(
        f"Sig_structure1 ({len(to_be_signed):>3d} B) = "
        f"{to_be_signed.hex()}"
    )
    print(f"signature ({len(signature):>3d} B) = {signature.hex()}")
    print(
        f"COSE_Sign1 ({len(cose_sign1_bytes):>3d} B) = "
        f"{cose_sign1_bytes.hex()}"
    )


if __name__ == "__main__":
    here = Path(__file__).resolve().parent
    main(here)
