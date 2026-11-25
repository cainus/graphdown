#!/usr/bin/env bash
# Builds one arm's copy of the corpus.
#   bench/setup.sh ARM DEST     ARM is A, B or C (see docs/benchmark.md#arms)
# The copy is a fresh git repo so a run's changes can be graded as a diff.
set -euo pipefail
arm="$1"; dest="$2"
repo="$(cd "$(dirname "$0")/.." && pwd)"
real="${GRAPHDOWN_REAL:-$repo/target/release/graphdown}"

rm -rf "$dest"; mkdir -p "$dest"
git -C "$repo" archive HEAD | tar -x -C "$dest"
# Nothing that describes the benchmark, and no agent instructions that would
# tell arm A or C about the CLI. Arm B gets the skill in its prompt instead.
rm -rf "$dest/bench" "$dest/docs/benchmark.md" "$dest/docs/benchmark-pilot.md" \
       "$dest/AGENTS.md" "$dest/CLAUDE.md" "$dest/.claude"
# The benchmark plan is linked from the README; drop that line too.
sed -i '/docs\/benchmark/d; /^- @AGENTS:/d' "$dest/README.md"
sed -i 's/@AGENTS\b/`AGENTS.md`/g' "$dest/docs/design.md"

case "$arm" in
  A|B) "$real" --root "$dest" fmt --to links >/dev/null ;;   # docs as found: plain links
  C)   ;;                                                    # docs in @ form
  *)   echo "unknown arm $arm" >&2; exit 2 ;;
esac
"$real" --root "$dest" check >/dev/null   # every arm starts valid

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
