#!/bin/bash
#
# Rotate LiveKit API key/secret for a self-hosted LiveKit deployment.
#
# Usage:
#   cd ~/livekit.sphinx.chat
#   /path/to/rotate_livekit_keys.sh [--dry-run]
#
# Options:
#   --dry-run    Show what would change (matched files, line counts, diffs)
#                without writing, backing up, or restarting anything.
#
# Run this script from inside the LiveKit deployment directory (the one
# containing livekit.yaml, docker-compose.yaml, etc.).
#
# What it does (without --dry-run):
#   1. Reads the current API key/secret from livekit.yaml
#   2. Generates a new key/secret pair
#   3. Backs up all relevant config files into ./.backup/<timestamp>/
#   4. Replaces the old key/secret in:
#        livekit.yaml, egress.yaml, caddy.yaml, docker-compose.yaml
#   5. Restarts the docker compose stack
#
# Caveats:
#   - All active LiveKit sessions will disconnect on restart.
#   - Any external clients (sphinx-relay, meet frontend, mobile apps, etc.)
#     using the old key MUST be updated separately or they will break.
#   - For zero-downtime rotation, manually add the new key alongside the
#     old one, migrate clients, then re-run this script (or remove the old
#     key by hand).

set -euo pipefail

DRY_RUN=0

for arg in "$@"; do
  case "$arg" in
    --dry-run|-n)
      DRY_RUN=1
      ;;
    -h|--help)
      sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg" >&2
      exit 1
      ;;
  esac
done

if [ ! -f livekit.yaml ]; then
  echo "Error: livekit.yaml not found in $(pwd)" >&2
  echo "       cd into the LiveKit deployment directory first." >&2
  exit 1
fi

if [ "$DRY_RUN" -eq 1 ]; then
  echo "*** DRY RUN — no files will be modified ***"
  echo
fi
echo "Working directory: $(pwd)"
echo

# ---------------------------------------------------------------------------
# 1. Detect current key/secret from livekit.yaml
# ---------------------------------------------------------------------------
# Grab the first non-comment "key: secret" line under the keys: section.
KEYS_LINE=$(awk '
  /^[[:space:]]*keys[[:space:]]*:/ { in_keys=1; next }
  in_keys && /^[^[:space:]#]/ { in_keys=0 }
  in_keys && /^[[:space:]]+[A-Za-z0-9_-]+[[:space:]]*:/ && !/^[[:space:]]*#/ {
    print; exit
  }
' livekit.yaml)

if [ -z "$KEYS_LINE" ]; then
  echo "Error: could not find a key/secret pair under 'keys:' in livekit.yaml" >&2
  exit 1
fi

OLD_KEY=$(echo "$KEYS_LINE"    | awk -F: '{print $1}' | tr -d ' ')
OLD_SECRET=$(echo "$KEYS_LINE" | awk -F: '{print $2}' | tr -d ' ')

if [ -z "$OLD_KEY" ] || [ -z "$OLD_SECRET" ]; then
  echo "Error: failed to parse key/secret from line: $KEYS_LINE" >&2
  exit 1
fi

echo "Current key:    $OLD_KEY"
echo "Current secret: $OLD_SECRET"
echo

# ---------------------------------------------------------------------------
# 2. Generate new key/secret
# ---------------------------------------------------------------------------
NEW_KEY="API$(openssl rand -hex 6)"
NEW_SECRET=$(openssl rand -base64 48 | tr -d '=+/\n' | cut -c1-40)

echo "New key:        $NEW_KEY"
echo "New secret:     $NEW_SECRET"
echo

# ---------------------------------------------------------------------------
# Scan files for matches (used by both dry-run and real run)
# ---------------------------------------------------------------------------
FILES=(livekit.yaml egress.yaml caddy.yaml docker-compose.yaml redis.conf init_script.sh)

MATCHED_FILES=()
TOTAL_KEY_HITS=0
TOTAL_SECRET_HITS=0

echo "Scanning for matches..."
printf "  %-22s %-10s %-10s\n" "FILE" "KEY_HITS" "SECRET_HITS"
printf "  %-22s %-10s %-10s\n" "----" "--------" "-----------"
for f in "${FILES[@]}"; do
  if [ ! -f "$f" ]; then
    printf "  %-22s %s\n" "$f" "(not found)"
    continue
  fi
  k_hits=$(grep -cF "$OLD_KEY"    "$f" || true)
  s_hits=$(grep -cF "$OLD_SECRET" "$f" || true)
  printf "  %-22s %-10s %-10s\n" "$f" "$k_hits" "$s_hits"
  if [ "$k_hits" -gt 0 ] || [ "$s_hits" -gt 0 ]; then
    MATCHED_FILES+=("$f")
    TOTAL_KEY_HITS=$((TOTAL_KEY_HITS + k_hits))
    TOTAL_SECRET_HITS=$((TOTAL_SECRET_HITS + s_hits))
  fi
done
echo
echo "Total: ${TOTAL_KEY_HITS} key match(es), ${TOTAL_SECRET_HITS} secret match(es) across ${#MATCHED_FILES[@]} file(s)."
echo

if [ "${#MATCHED_FILES[@]}" -eq 0 ]; then
  echo "Error: no files contain the current key or secret. Aborting." >&2
  exit 1
fi

# Sanity check: livekit.yaml must contain both
if ! grep -qF "$OLD_KEY" livekit.yaml || ! grep -qF "$OLD_SECRET" livekit.yaml; then
  echo "Error: livekit.yaml is missing the detected key or secret. Aborting." >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# Dry-run: show diffs and exit
# ---------------------------------------------------------------------------
if [ "$DRY_RUN" -eq 1 ]; then
  echo "Preview of changes (lines that would be modified):"
  echo
  for f in "${MATCHED_FILES[@]}"; do
    echo "--- $f ---"
    grep -nF -e "$OLD_KEY" -e "$OLD_SECRET" "$f" | while IFS= read -r line; do
      ln="${line%%:*}"
      content="${line#*:}"
      new_content=$(echo "$content" | sed "s|${OLD_KEY}|${NEW_KEY}|g; s|${OLD_SECRET}|${NEW_SECRET}|g")
      echo "  $ln: - $content"
      echo "  $ln: + $new_content"
    done
    echo
  done
  echo "*** DRY RUN complete — no files were changed. ***"
  exit 0
fi

# ---------------------------------------------------------------------------
# Confirm before destructive changes
# ---------------------------------------------------------------------------
read -r -p "Proceed with rotation and restart? [y/N] " confirm
case "$confirm" in
  [yY]|[yY][eE][sS]) ;;
  *) echo "Aborted."; exit 0 ;;
esac

# ---------------------------------------------------------------------------
# 3. Backup
# ---------------------------------------------------------------------------
TS=$(date +%Y%m%d_%H%M%S)
BACKUP_DIR=".backup/$TS"
mkdir -p "$BACKUP_DIR"

for f in "${FILES[@]}"; do
  [ -f "$f" ] && cp "$f" "$BACKUP_DIR/"
done
echo "Backed up configs to $(pwd)/$BACKUP_DIR"
echo

# ---------------------------------------------------------------------------
# 4. Replace key/secret across config files
# ---------------------------------------------------------------------------
for f in "${MATCHED_FILES[@]}"; do
  sed -i.bak "s|${OLD_KEY}|${NEW_KEY}|g; s|${OLD_SECRET}|${NEW_SECRET}|g" "$f"
  rm -f "$f.bak"
  echo "Updated: $f"
done
echo

# ---------------------------------------------------------------------------
# 5. Restart docker compose stack
# ---------------------------------------------------------------------------
if command -v docker >/dev/null 2>&1; then
  echo "Restarting docker compose stack..."
  if docker compose version >/dev/null 2>&1; then
    docker compose down
    docker compose up -d
  else
    docker-compose down
    docker-compose up -d
  fi
else
  echo "WARNING: docker not found in PATH; skipping restart." >&2
fi

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
cat <<EOF

==========================================================
LiveKit key rotation complete.

  LIVEKIT_API_KEY=$NEW_KEY
  LIVEKIT_API_SECRET=$NEW_SECRET

Backup of previous configs: $(pwd)/$BACKUP_DIR

Update any external clients (sphinx-relay, meet frontend,
mobile apps, etc.) with the new key/secret.
==========================================================
EOF
