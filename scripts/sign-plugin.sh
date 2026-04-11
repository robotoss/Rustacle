#!/usr/bin/env bash
set -euo pipefail

# Sign a WASM plugin with Ed25519.
# Usage: bash scripts/sign-plugin.sh <plugin.wasm> [private_key_file]
#
# If no key file is provided, uses keys/dev_signing_key.pem (generated if missing).

WASM_FILE="${1:?Usage: sign-plugin.sh <plugin.wasm> [private_key_file]}"
KEY_FILE="${2:-keys/dev_signing_key.pem}"
SIG_FILE="${WASM_FILE}.sig"

if [ ! -f "$WASM_FILE" ]; then
    echo "ERROR: $WASM_FILE not found"
    exit 1
fi

# Generate dev key if missing
if [ ! -f "$KEY_FILE" ]; then
    echo "Generating dev signing key at $KEY_FILE..."
    mkdir -p "$(dirname "$KEY_FILE")"
    openssl genpkey -algorithm ed25519 -out "$KEY_FILE" 2>/dev/null
    openssl pkey -in "$KEY_FILE" -pubout -out "${KEY_FILE%.pem}.pub.pem" 2>/dev/null
    echo "Dev key generated. Public key: ${KEY_FILE%.pem}.pub.pem"
fi

# Sign
openssl pkeyutl -sign -inkey "$KEY_FILE" -rawin -in "$WASM_FILE" -out "$SIG_FILE" 2>/dev/null

echo "Signed: $SIG_FILE ($(wc -c < "$SIG_FILE") bytes)"
