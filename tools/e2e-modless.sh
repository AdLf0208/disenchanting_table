#!/usr/bin/env bash
# The Dis-Enchanting Table's negative and packaged-content checks.
#
# Two launches in one scratch home, in fresh worlds:
#   1. the default world WITHOUT `--dev-mod`: the block and its item are unknown,
#      so the e2e above is not vacuous (the native half is in the binary either
#      way, so its command still answers — it finds no table);
#   2. the same checks with the packaged `content/disenchanting_table.hmod`, which
#      must register the block and the item and give the command a table to read.
set -euo pipefail
trap 'echo "FAILED at line $LINENO: $BASH_COMMAND" >&2' ERR

CLONE=${CAMPAIGN_ROOT:?set CAMPAIGN_ROOT to a checkout of the ATM10-Horizon campaign tree}
PORT=$CLONE/ports/disenchanting_table
SHARED=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$SHARED/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/disenchanting_table-modless.sock}
HMOD=$PORT/content/disenchanting_table.hmod
TABLE=disenchanting_table:disenchanting_table

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -f "$HMOD" ] || { echo "missing packaged content: $HMOD" >&2; exit 1; }

SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/disenchanting_table-modless.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")"
rm -f "$SOCK"
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
say() { echo "== $* =="; }
# The recent chat as one line: a refused command answers with the command and its
# <--[HERE] marker, then the reason.
reply() { ctl messages 6 | jq -r '.chat[]' | tr '\n' '|'; }

# Every run ends with its status line, so the artefact proves the verdict.
finish() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    rm -rf "$SCRATCH" || true
    echo "E2E EXIT:$status"
}
trap finish EXIT

launch() { # world [extra args...]
    local world=$1
    shift
    "$CTL" --socket "$SOCK" launch \
        --game "$GAME" --world "$world" --seed 20261109 --creative --offscreen --wait \
        --game-dir "$SCRATCH/game" --resources "$RES" "$@"
    ctl wait-ready 180 >/dev/null
    local px pz
    px=$(ctl state | jq -r '.player.position[0]' | cut -d. -f1)
    pz=$(ctl state | jq -r '.player.position[2]' | cut -d. -f1)
    ctl command "fill $((px-6)) 100 $((pz-6)) $((px+6)) 100 $((pz+6)) minecraft:stone" >/dev/null
    ctl command "fill $((px-6)) 101 $((pz-6)) $((px+6)) 108 $((pz+6)) minecraft:air" >/dev/null
    ctl tp "$px.5" 101 "$((pz+2)).5" >/dev/null
    ctl wait 12 >/dev/null
    X=$((px - 2)); Y=102; Z=$((pz + 1))
}

say "launch without --dev-mod"
launch NoTableE2E

say "the block and the item are unknown, and the readout finds no table"
ctl command "give @s $TABLE 1" >/dev/null
ctl wait 3 >/dev/null
gave=$(reply)
echo "give: $gave"
echo "$gave" | grep -q "Unknown item '$TABLE'" \
    || fail "the unmodded world knows the table item: $gave"

ctl command "setblock $X $Y $Z $TABLE" >/dev/null
ctl wait 3 >/dev/null
placed=$(reply)
echo "setblock: $placed"
echo "$placed" | grep -q "Unknown block type '$TABLE'" \
    || fail "the unmodded world knows the table block: $placed"
[ "$(ctl block "$X" "$Y" "$Z" | jq -r '.block // "unknown"')" = "minecraft:air" ] \
    || fail "the unmodded world placed something at $X $Y $Z"

ctl chat "/disenchanting_table info $X $Y $Z" >/dev/null
ctl wait 3 >/dev/null
readout=$(reply)
echo "readout: $readout"
echo "$readout" | grep -q "not a dis-enchanting table" \
    || fail "the readout did not report a missing table: $readout"
echo "$readout" | grep -q "disenchanting_table:disenchanting_table: input=" \
    && fail "the unmodded world read a table's slots: $readout"

ctl quit >/dev/null
sleep 2
rm -f "$SOCK"

say "launch with the packaged content/disenchanting_table.hmod"
launch TableHmodE2E --dev-mod "$HMOD"

say "the packaged content registers the block and the item"
ctl command "setblock $X $Y $Z $TABLE" >/dev/null
ctl wait 3 >/dev/null
placed=$(reply)
echo "setblock: $placed"
echo "$placed" | grep -q "Unknown block type" \
    && fail "the packaged content did not register the block: $placed"
[ "$(ctl block "$X" "$Y" "$Z" | jq -r '.block // "unknown"')" = "$TABLE" ] \
    || fail "the packaged table did not place at $X $Y $Z"

ctl command "give @s $TABLE 1" >/dev/null
ctl wait 3 >/dev/null
gave=$(reply)
echo "give: $gave"
echo "$gave" | grep -q "Unknown item" \
    && fail "the packaged content did not register the item: $gave"

ctl chat "/disenchanting_table info $X $Y $Z" >/dev/null
ctl wait 3 >/dev/null
readout=$(reply)
echo "readout: $readout"
echo "$readout" | grep -q "disenchanting_table:disenchanting_table: input=none" \
    || fail "the packaged table's command did not answer: $readout"

echo "PASS: without --dev-mod the table's block and item are unknown and the readout finds no table, and the packaged content/disenchanting_table.hmod registers the block and item and answers the readout"
