#!/usr/bin/env bash
# Builds one arm's copy of the corpus.
#   bench/setup.sh ARM DEST [SOURCE]   ARM is A, B or C (see docs/benchmark.md#arms)
# SOURCE is a git checkout to copy; it defaults to this repo.
# The copy is a fresh git repo so a run's changes can be graded as a diff.
set -euo pipefail
arm="$1"; dest="$2"
self="$(cd "$(dirname "$0")/.." && pwd)"
repo="${3:-$self}"
real="${GRAPHDOWN_REAL:-$self/target/release/graphdown}"

rm -rf "$dest"; mkdir -p "$dest"
git -C "$repo" archive HEAD | tar -x -C "$dest"
if [ "$repo" = "$self" ]; then
  # Nothing that describes the benchmark, and no agent instructions that would
  # tell arm A or C about the CLI. Arm B gets the skill in its prompt instead.
  rm -rf "$dest/bench" "$dest"/docs/benchmark*.md \
         "$dest/AGENTS.md" "$dest/CLAUDE.md" "$dest/.claude"
  sed -i '/docs\/benchmark/d; /^- @AGENTS:/d' "$dest/README.md"
  sed -i 's/@AGENTS\b/`AGENTS.md`/g' "$dest/docs/design.md"
fi

case "$arm" in
  A|B) "$real" --root "$dest" fmt --to links >/dev/null ;;   # docs as found: plain links
  C)   ;;                                                    # docs in @ form
  *)   echo "unknown arm $arm" >&2; exit 2 ;;
esac
# Graphdown's own repo starts with every reference valid; a foreign repo
# starts as found, broken links included, the same for every arm.
if [ "$repo" = "$self" ]; then "$real" --root "$dest" check >/dev/null; fi

if [ "$arm" = B ]; then
  mkdir -p "$dest/.bench/bin"
  cat > "$dest/.bench/bin/graphdown" <<WRAP
#!/usr/bin/env bash
# Logs each call, then runs the real binary.
out=\$("$real" "\$@"); code=\$?
printf '%s\t%s\t%s\n' "\$(date +%s)" "\${#out}" "\$*" >> "$dest/.bench/calls.log"
printf '%s\n' "\$out"
exit \$code
WRAP
  chmod +x "$dest/.bench/bin/graphdown"
fi

mkdir -p "$dest/.bench"
( cd "$dest" && git init -q && echo .bench >> .git/info/exclude && git add -A && git -c user.name=bench -c user.email=bench@x commit -qm base )
echo "$dest"
