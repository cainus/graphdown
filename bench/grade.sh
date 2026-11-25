#!/usr/bin/env bash
# Scripted checks for the change tasks. Prints one JSON object.
#   bench/grade.sh TASK DEST
# Builds DEST with its own target dir, then probes the result. Questions (t1,
# t2) and judgement calls are graded blind by a separate grader instead.
set -uo pipefail
task="$1"; dest="$2"
export CARGO_TARGET_DIR="$dest/.bench/target"
cd "$dest"
build=false; tests=false
cargo build -q 2>/dev/null && build=true
cargo test -q >/dev/null 2>&1 && tests=true
bin="$CARGO_TARGET_DIR/debug/graphdown"
real="${GRAPHDOWN_REAL:-/tmp/graphdown-real}"
# References are checked with the run's own binary when it built, so a
# renamed command or changed slug rule is judged by the code that has it.
gd() { if $build; then "$bin" "$@"; else "$real" "$@"; fi; }
check_ok=false; gd --root "$dest" check >/dev/null 2>&1 && check_ok=true
broken=$(gd --root "$dest" check 2>/dev/null | grep -cE ':[0-9]+:[0-9]+: ' || true)
diffstat=$(git -C "$dest" diff --shortstat HEAD 2>/dev/null; git -C "$dest" ls-files --others --exclude-standard | grep -v '^.bench' | wc -l | sed 's/$/ new files/')
extra='{}'
case "$task" in
  t3)
    plain=$(gd --root "$dest" refs docs/cli --count 2>/dev/null | tr -d '[:space:]')
    js=$(gd --root "$dest" refs docs/cli --count --json 2>/dev/null)
    js_ok=$(printf '%s' "$js" | python3 -c 'import json,sys
try:
  v=json.load(sys.stdin); print("true" if (isinstance(v,int) or (isinstance(v,dict) and any(isinstance(x,int) for x in v.values()))) else "false")
except Exception: print("false")')
    count_plain=$([[ "$plain" =~ ^[0-9]+$ ]] && echo true || echo false)
    documented=$(grep -q -- '--count' docs/cli.md && echo true || echo false)
    extra="{\"count_plain\":$count_plain,\"count_json_is_json\":$js_ok,\"documented\":$documented}"
    ;;
  t4)
    tmp=$(mktemp -d); touch "$tmp/graphdown.toml"
    printf '# T\n\n## Retry & Backoff\n\nSee @a#retry-and-backoff\n' > "$tmp/a.md"
    works=false; $build && "$bin" --root "$tmp" check >/dev/null 2>&1 && works=true
    syntax=$(sed -n '/^## Symbols are headings/,/^## /p' docs/syntax.md | grep -qiE '&.*and|and.*&' && echo true || echo false)
    skill=$(grep -qiE '&.*`?and`?|`and`' skill/SKILL.md && echo true || echo false)
    extra="{\"slug_works\":$works,\"syntax_doc_updated\":$syntax,\"skill_updated\":$skill}"
    ;;
  t5)
    map=false; old_gone=false; alias=false
    if $build; then
      "$bin" --root "$dest" map >/dev/null 2>&1 && map=true
      "$bin" --root "$dest" outline >/dev/null 2>&1 || old_gone=true
      "$bin" --root "$dest" ls >/dev/null 2>&1 && alias=true
    fi
    leftovers=$(grep -rnE 'graphdown outline|#outline\b|cli#outline|cli\.md#outline' --include='*.md' --include='*.rs' . 2>/dev/null | grep -v '^./.bench' | wc -l)
    extra="{\"map_works\":$map,\"outline_removed\":$old_gone,\"ls_alias\":$alias,\"leftover_mentions\":$leftovers}"
    ;;
  t6)
    have=0; for c in check outline show refs slice fmt skill; do [ -f "docs/cli/$c.md" ] && have=$((have+1)); done
    old_words=$(git -C "$dest" show HEAD:docs/cli.md | wc -w)
    new_words=$(cat docs/cli.md docs/cli/*.md 2>/dev/null | wc -w)
    stale=$(grep -rnE 'cli#(check|outline|show|refs|slice|fmt|skill|exit-codes)\b|cli\.md#(check|outline|show|refs|slice|fmt|skill)\b' --include='*.md' --include='*.rs' . 2>/dev/null | grep -v '^./.bench' | wc -l)
    extra="{\"command_files\":$have,\"words_before\":$old_words,\"words_after\":$new_words,\"stale_section_refs\":$stale}"
    ;;
esac
python3 - "$task" "$build" "$tests" "$check_ok" "$broken" "$extra" "$diffstat" <<'PY'
import json,sys
t,b,te,c,br,ex,ds=sys.argv[1:]
print(json.dumps({"task":t,"build":b=="true","tests":te=="true","refs_valid":c=="true","broken_refs":int(br or 0),**json.loads(ex),"diff":" ".join(ds.split())}))
PY
