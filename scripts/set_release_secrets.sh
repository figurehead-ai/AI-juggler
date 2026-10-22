#!/usr/bin/env bash
# D-20 / AJ-E1010: puts the release signing secrets into GitHub, for the
# maintainer to run (docs/internal/MAINTAINER-STEPS.md step 5). Run it again when a
# certificate is renewed.
#
#   scripts/set_release_secrets.sh [path/to/DeveloperID.p12]
#
# Secrets come only from files you exported or from gh's hidden prompt:
# nothing is echoed, logged or written anywhere else. If the updater key
# doesn't exist yet, it's generated here and you choose its password.
# APPLE_TEAM_ID and APPLE_SIGNING_IDENTITY are not secrets and are set
# separately.
set -euo pipefail
repo="figurehead-ai/AI-juggler"
# The exported .p12: the argument, else the usual places it's saved.
p12="${1:-}"
if [ -z "$p12" ]; then
  for f in "$HOME/Desktop/DeveloperID.p12" "$HOME/Downloads/Certificates.p12" "$HOME/Downloads/DeveloperID.p12"; do
    [ -f "$f" ] && { p12="$f"; break; }
  done
fi
key="$HOME/.tauri/ai-juggler-updater.key"
ui="$(cd "$(dirname "$0")/../ui" && pwd)"

command -v gh >/dev/null || { echo "Install the GitHub CLI first: brew install gh"; exit 1; }
[ -n "$p12" ] && [ -f "$p12" ] || { echo "No .p12 found — export the Developer ID certificate as .p12 (step 3b), then: $0 path/to/file.p12"; exit 1; }

echo "1/7  Developer ID certificate → APPLE_CERTIFICATE (from $p12)"
base64 -i "$p12" | gh secret set APPLE_CERTIFICATE --repo "$repo"

echo "2/7  APPLE_CERTIFICATE_PASSWORD — paste the .p12 password (hidden):"
gh secret set APPLE_CERTIFICATE_PASSWORD --repo "$repo"

echo "3/7  APPLE_ID — paste your Apple ID email:"
gh secret set APPLE_ID --repo "$repo"

echo "4/7  APPLE_PASSWORD — paste the app-specific password (hidden):"
gh secret set APPLE_PASSWORD --repo "$repo"

if [ ! -f "$key" ]; then
  echo "     No updater key yet: generating one. Choose a password and save it in your password manager."
  mkdir -p "$(dirname "$key")"
  (cd "$ui" && pnpm tauri signer generate -w "$key")
fi

echo "5/7  Updater private key → TAURI_SIGNING_PRIVATE_KEY (from $key)"
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo "$repo" < "$key"

echo "6/7  TAURI_SIGNING_PRIVATE_KEY_PASSWORD — paste the updater key's password (hidden):"
gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo "$repo"

echo "7/7  Updater public key → AIJ_UPDATER_PUBKEY (a variable: it's public)"
gh variable set AIJ_UPDATER_PUBKEY --repo "$repo" < "$key.pub"

echo
echo "Done. Names now set (values are never shown):"
gh secret list --repo "$repo"
gh variable list --repo "$repo"
echo
echo "Back up $key and its password, and the .p12 and its password, in your password manager."
echo "Then delete $p12: the certificate stays in your keychain."
