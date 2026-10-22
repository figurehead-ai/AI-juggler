#!/usr/bin/env python3
"""AJ-E2001: checks a Tauri updater signature against the public key the
app has baked in, before anything is published.

    updater_sig_check.py ARCHIVE SIGFILE PUBKEY [--openssl PATH]
    updater_sig_check.py --selftest [--openssl PATH]

PUBKEY is the value of AIJ_UPDATER_PUBKEY (public). Tauri's keys and
signatures are minisign's, base64-encoded once more:

- the public key: `untrusted comment: …` then base64(`Ed` + key id (8) +
  Ed25519 public key (32));
- the signature: `untrusted comment: …`, base64(algorithm (2) + key id (8)
  + Ed25519 signature (64)), `trusted comment: …`, base64(global
  signature (64)). Algorithm `ED` signs the BLAKE2b-512 of the file,
  `Ed` the file itself; the global signature covers the signature and the
  trusted comment.

It passes only when the key ids match and both signatures verify. Hashing
is the standard library's; Ed25519 is OpenSSL 3's (`pkeyutl -rawin`), which
GitHub's macOS runners carry. No third-party Python package, no build code.
"""

import argparse
import base64
import hashlib
import os
import subprocess
import sys
import tempfile

# DER prefix of an Ed25519 SubjectPublicKeyInfo (RFC 8410), before the key.
ED25519_SPKI = bytes.fromhex("302a300506032b6570032100")


class SigError(Exception):
    pass


def _lines(blob: str) -> list:
    try:
        text = base64.b64decode(blob.strip(), validate=True).decode()
    except Exception as error:  # noqa: BLE001 - any decoding failure is a bad file
        raise SigError(f"not base64-wrapped minisign data ({error})") from None
    return [line for line in text.splitlines() if line.strip()]


def parse_pubkey(blob: str):
    lines = _lines(blob)
    if len(lines) < 2 or not lines[0].startswith("untrusted comment:"):
        raise SigError("public key: unexpected layout")
    raw = base64.b64decode(lines[1])
    if len(raw) != 42 or raw[:2] != b"Ed":
        raise SigError("public key: not an Ed25519 minisign key")
    return raw[2:10], raw[10:]


def parse_signature(blob: str):
    lines = _lines(blob)
    if len(lines) < 4 or not lines[2].startswith("trusted comment: "):
        raise SigError("signature: unexpected layout")
    raw = base64.b64decode(lines[1])
    if len(raw) != 74 or raw[:2] not in (b"ED", b"Ed"):
        raise SigError("signature: not an Ed25519 minisign signature")
    trusted = lines[2][len("trusted comment: "):].encode()
    global_sig = base64.b64decode(lines[3])
    if len(global_sig) != 64:
        raise SigError("signature: bad global signature")
    return raw[:2], raw[2:10], raw[10:], trusted, global_sig


def ed25519_verify(openssl: str, public: bytes, message: bytes, signature: bytes) -> bool:
    with tempfile.TemporaryDirectory() as work:
        key = os.path.join(work, "key.der")
        msg = os.path.join(work, "msg")
        sig = os.path.join(work, "sig")
        for path, data in ((key, ED25519_SPKI + public), (msg, message), (sig, signature)):
            with open(path, "wb") as handle:
                handle.write(data)
        result = subprocess.run(
            [openssl, "pkeyutl", "-verify", "-pubin", "-inkey", key, "-keyform", "DER",
             "-rawin", "-in", msg, "-sigfile", sig],
            capture_output=True,
            check=False,
        )
        return result.returncode == 0


def check(archive: bytes, sig_blob: str, pubkey_blob: str, openssl: str) -> None:
    key_id, public = parse_pubkey(pubkey_blob)
    algorithm, sig_key_id, signature, trusted, global_sig = parse_signature(sig_blob)
    if sig_key_id != key_id:
        raise SigError(
            f"signed with key {sig_key_id[::-1].hex().upper()}, "
            f"the app trusts {key_id[::-1].hex().upper()}"
        )
    message = hashlib.blake2b(archive, digest_size=64).digest() if algorithm == b"ED" else archive
    if not ed25519_verify(openssl, public, message, signature):
        raise SigError("the signature doesn't match the archive")
    if not ed25519_verify(openssl, public, signature + trusted, global_sig):
        raise SigError("the trusted comment's signature doesn't verify")


def find_openssl(given):
    for candidate in (given, "/opt/homebrew/opt/openssl@3/bin/openssl",
                      "/usr/local/opt/openssl@3/bin/openssl", "openssl"):
        if not candidate:
            continue
        try:
            out = subprocess.run([candidate, "version"], capture_output=True, text=True, check=False)
        except OSError:
            continue
        if out.returncode == 0 and out.stdout.startswith("OpenSSL 3"):
            return candidate
    raise SigError("OpenSSL 3 not found (Ed25519 needs it; LibreSSL won't do)")


def _selftest(openssl: str) -> None:
    """Builds a minisign key and signature with OpenSSL, the way Tauri
    lays them out, then checks the good case and each way it must fail."""
    with tempfile.TemporaryDirectory() as work:
        private = os.path.join(work, "k.pem")
        subprocess.run([openssl, "genpkey", "-algorithm", "ed25519", "-out", private], check=True,
                       capture_output=True)
        der = subprocess.run([openssl, "pkey", "-in", private, "-pubout", "-outform", "DER"],
                             check=True, capture_output=True).stdout
        public = der[-32:]

        def sign(data: bytes) -> bytes:
            path = os.path.join(work, "m")
            with open(path, "wb") as handle:
                handle.write(data)
            return subprocess.run([openssl, "pkeyutl", "-sign", "-inkey", private, "-rawin", "-in", path],
                                  check=True, capture_output=True).stdout

        key_id = bytes(range(1, 9))
        pubkey = base64.b64encode(
            (f"untrusted comment: minisign public key: {key_id[::-1].hex().upper()}\n"
             + base64.b64encode(b"Ed" + key_id + public).decode() + "\n").encode()
        ).decode()
        archive = b"an updater archive"

        def signature_for(data: bytes, kid: bytes = key_id, trusted: bytes = b"timestamp:1\tfile:x") -> str:
            sig = sign(hashlib.blake2b(data, digest_size=64).digest())
            global_sig = sign(sig + trusted)
            text = ("untrusted comment: signature from tauri secret key\n"
                    + base64.b64encode(b"ED" + kid + sig).decode() + "\n"
                    + "trusted comment: " + trusted.decode() + "\n"
                    + base64.b64encode(global_sig).decode() + "\n")
            return base64.b64encode(text.encode()).decode()

        good = signature_for(archive)
        check(archive, good, pubkey, openssl)
        print("ok   a good signature passes")
        cases = (
            ("another file", lambda: check(b"tampered", good, pubkey, openssl)),
            ("another key id", lambda: check(archive, signature_for(archive, kid=b"\x09" * 8), pubkey, openssl)),
            ("a tampered trusted comment", lambda: check(
                archive,
                base64.b64encode(base64.b64decode(good).replace(b"timestamp:1", b"timestamp:2")).decode(),
                pubkey,
                openssl,
            )),
            ("not a signature", lambda: check(archive, "bm90IGEgc2lnbmF0dXJl", pubkey, openssl)),
        )
        for name, run in cases:
            try:
                run()
            except SigError as error:
                print(f"ok   {name} fails: {error}")
            else:
                raise SystemExit(f"FAIL {name} passed")
    print("updater-sig-check selftest: OK")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("archive", nargs="?")
    parser.add_argument("sigfile", nargs="?")
    parser.add_argument("pubkey", nargs="?")
    parser.add_argument("--openssl")
    parser.add_argument("--selftest", action="store_true")
    args = parser.parse_args()
    try:
        openssl = find_openssl(args.openssl)
        if args.selftest:
            _selftest(openssl)
            return 0
        if not (args.archive and args.sigfile and args.pubkey):
            parser.error("ARCHIVE, SIGFILE and PUBKEY are required")
        with open(args.archive, "rb") as handle:
            archive = handle.read()
        with open(args.sigfile) as handle:
            sig_blob = handle.read()
        check(archive, sig_blob, args.pubkey, openssl)
    except SigError as error:
        print(f"::error::Updater signature: {error}", file=sys.stderr)
        return 1
    print(f"updater signature OK: {os.path.basename(args.archive)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
