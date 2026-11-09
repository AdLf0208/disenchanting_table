#!/usr/bin/env bash
# In-game check for the Dis-Enchanting Table slice 1: the table block.
#
# One offscreen creative launch, then in the Overworld on a platform:
#   1. the table places and its block entity reads three empty slots;
#   2. a sharpness-3 diamond sword and a book go into the menu's slots (open
#      the menu, click each slot), and the readout says a click is ready;
#   3. a survival player with 135 points right-clicks: the sword keeps its item
#      with no enchantments, the book slot empties, an enchanted book with
#      sharpness 3 appears in the output slot, and the player has 110 points —
#      exactly the 25-point cost, charged once;
#   4. the output book is taken through the menu;
#   5. controls: an unenchanted item, an enchanted item with no book, and an
#      enchanted item with no experience each dis-enchant nothing and lose
#      nothing; a vanilla enchanting table is untouched.
#
# Default run (EMBEDDED=0): the game also loads `content/` as a dev mod.
# EMBEDDED=1: the game is launched with **only the native bundle** (no
# `--dev-mod`) and the content comes from the bundle's embedded copy
# (`mixins/<id>/data/…`, the AE2 pattern). The run also checks the file the
# helper wrote, and CONTROL=1 (default with EMBEDDED) first launches the same
# binary with `.horizon-cache` unusable, where the ids must be unknown.
#
# SELFTEST=1 runs the same scenario but asserts the stripped sword kept its
# enchantment: that must fail (exit 1), proving the assertions are not vacuous.
set -euo pipefail
trap 'echo "FAILED at line $LINENO: $BASH_COMMAND" >&2' ERR

CLONE=${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/disenchanting_table
SHARED=$CLONE/ports/_shared/bin/horizon-1d63e0a5
CTL=${CTL:-$SHARED/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/disenchanting_table-e2e.sock}
SELFTEST=${SELFTEST:-0}
EMBEDDED=${EMBEDDED:-0}
CONTROL=${CONTROL:-$EMBEDDED}

TABLE=disenchanting_table:disenchanting_table
SWORD=minecraft:diamond_sword
BOOK=minecraft:book
EMBEDDED_FILE=$PORT/mixins/disenchanting_table/data/disenchanting_table-content.hmod
WORLD=DisenchantingTableE2E
CONTROL_WORLD=DisenchantingTableControl
# EMBEDDED=1 launches with only the bundle; otherwise `content/` is a dev mod too.
LAUNCH_ARGS=()
[ "$EMBEDDED" = 1 ] || LAUNCH_ARGS+=(--dev-mod "$PORT/content")

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }
[ -f "$PORT/content/horizon-mod.toml" ] || { echo "missing content mod: $PORT/content" >&2; exit 1; }
[ -f "$EMBEDDED_FILE" ] || { echo "missing embedded content: $EMBEDDED_FILE" >&2; exit 1; }

SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}"/disenchanting_table-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
# The game is launched with `--game-dir $HORIZON_HOME`, and the game then makes
# that its working directory (`game.rs`), so the embedded helper's
# `current_dir().join(".horizon-cache")` is *this* one, not the script's cwd.
GAME_DIR=$SCRATCH/game
CACHE=$GAME_DIR/.horizon-cache
mkdir -p "$(dirname "$SOCK")"
rm -f "$SOCK"
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }
say() { echo "== $* =="; }
count_item() { ctl inventory | jq -r --arg i "$1" '.counts[$i] // 0'; }
# The recent chat as one line: a refused command answers with the command and its
# <--[HERE] marker, then the reason.
reply() { ctl messages 6 | jq -r '.chat[]' | tr '\n' '|'; }

info() { # x y z -> the table's readout line
    ctl chat "/disenchanting_table info $1 $2 $3" >/dev/null
    ctl wait 2 >/dev/null
    ctl messages 6 | jq -r '.chat[]' \
        | grep -m1 -E "^disenchanting_table:[a-z0-9_]+: input=" || true
}
field() { # key [line]
    local line=${2:-$LINE}
    echo "$line" | sed -n "s/.* $1=\([^ ]*\).*/\1/p"
}

stand_near() { # x y z
    ctl tp "$1.5" "$(($2 + 1))" "$(($3 + 3)).5" >/dev/null
    ctl look 180 20 >/dev/null
    ctl wait 4 >/dev/null
}

place_table() { # x y z block
    stand_near "$1" "$2" "$3"
    ctl setblock "$1" "$(( $2 - 1 ))" "$3" minecraft:stone >/dev/null
    ctl command "setblock $1 $2 $3 $4" >/dev/null
    ctl wait 4 >/dev/null
    [ "$(ctl block "$1" "$2" "$3" | jq -r '.block // "unknown"')" = "$4" ] \
        || fail "the placed block at $1 $2 $3 is not $4"
}

open_menu() { # x y z
    stand_near "$1" "$2" "$3"
    ctl interact "$1" "$2" "$3" >/dev/null || fail "the click did not open the table menu"
    ctl wait-for screen container --timeout 10 >/dev/null \
        || fail "the table did not open a menu (screen=$(ctl state | jq -r '.screen // ""'))"
    local slots
    slots=$(ctl screen | jq '[.slots[] | select(.slot >= 300)] | length' 2>/dev/null || echo 0)
    [ "$slots" = 3 ] || fail "the table menu shows $slots slots, expected 3"
}

put_in_slot() { # menu-slot item
    local src
    src=$(ctl screen | jq -r --arg i "$2" \
        '[.slots[] | select(.slot < 36 and .item == $i) | .slot] | map(.) | .[0] // ""' 2>/dev/null || true)
    if [ -z "$src" ]; then
        echo "screen with no $2:" >&2
        ctl screen >&2
        fail "$2 is not in the inventory"
    fi
    ctl click-slot "$src" >/dev/null || fail "cannot pick up $2 from slot $src"
    ctl wait 2 >/dev/null
    ctl click-slot "$1" >/dev/null || fail "cannot place $2 into table slot $1"
    ctl wait 2 >/dev/null
}

clear_hand() { ctl command "clear @s" >/dev/null 2>&1 || true; ctl wait 2 >/dev/null; }

give_and_hold() { # item [enchant]
    ctl command "item replace entity @s hotbar.0 with $1 1" >/dev/null \
        || fail "cannot give $1"
    ctl select 1 >/dev/null || fail "cannot select the $1 hotbar slot"
    ctl wait 2 >/dev/null
    if [ -n "${2:-}" ]; then
        # The vanilla command, on the server: `ctl enchant` is a client-side
        # station control that only enchants the window's own copy.
        ctl command "enchant @s $2 ${3:-3}" >/dev/null || fail "cannot enchant $2"
        ctl wait 3 >/dev/null
    fi
}

put_inputs() { # x y z input-item [enchant]
    clear_hand
    give_and_hold "$4" "${5:-}" "${6:-3}"
    open_menu "$1" "$2" "$3"
    put_in_slot 300 "$4"
    ctl close >/dev/null 2>&1 || true
    ctl wait 3 >/dev/null
    clear_hand
    ctl command "item replace entity @s hotbar.0 with $BOOK 1" >/dev/null || fail "cannot give a book"
    ctl wait 2 >/dev/null
    open_menu "$1" "$2" "$3"
    put_in_slot 301 $BOOK
    ctl close >/dev/null 2>&1 || true
    ctl wait 4 >/dev/null
}

# Every run ends with its status line, so the artefact proves the verdict.
finish() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    rm -rf "$SCRATCH" || true
    echo "E2E EXIT:$status"
}
trap finish EXIT

launch() { # world
    local world=$1
    rm -f "$SOCK"
    "$CTL" --socket "$SOCK" launch \
        --game "$GAME" --world "$world" --seed 20261109 --creative --offscreen --wait \
        --game-dir "$GAME_DIR" --resources "$RES" "${LAUNCH_ARGS[@]}" >/dev/null
    ctl wait-ready 180 >/dev/null
    if grep -q "content mods not loaded" "${SOCK}.log" 2>/dev/null; then
        grep "content mods not loaded" "${SOCK}.log" >&2
        fail "the content mod did not load"
    fi
    ctl wait 10
}

# An Overworld platform at y=100, away from the terrain, and the five tables'
# positions. The player is teleported first so the fills touch loaded chunks.
arena() {
    PX=$(ctl state | jq -r '.player.position[0]' | cut -d. -f1)
    PZ=$(ctl state | jq -r '.player.position[2]' | cut -d. -f1)
    ctl command "fill $((PX-10)) 100 $((PZ-10)) $((PX+10)) 100 $((PZ+10)) minecraft:stone" >/dev/null
    ctl command "fill $((PX-10)) 101 $((PZ-10)) $((PX+10)) 110 $((PZ+10)) minecraft:air" >/dev/null
    ctl tp "$PX.5" 101 "$((PZ+4)).5" >/dev/null
    ctl wait 10
    ctl command "gamerule spawn_mobs false" >/dev/null
    ctl command "gamerule spawn_monsters false" >/dev/null
    ctl command "kill @e[type=!minecraft:player]" >/dev/null
    ctl wait 10
    Y=102
    T1X=$PX;            T1Z=$((PZ + 1))
    T2X=$((PX - 3));    T2Z=$((PZ + 1))
    T3X=$((PX - 6));    T3Z=$((PZ + 1))
    T4X=$((PX - 9));    T4Z=$((PZ + 1))
    T5X=$((PX - 3));    T5Z=$((PZ - 2))
    ctl look 180 20
    ctl wait 4
}

# EMBEDDED=1: the bundle must have written its content out at startup, byte for
# byte the delivered content mod.
embedded_content_check() {
    local cache=$CACHE/disenchanting_table-embedded-content.hmod
    [ -f "$cache" ] || fail "the bundle did not write its embedded content to $cache"
    cmp -s "$cache" "$PORT/content/disenchanting_table.hmod" \
        || fail "the embedded content differs from content/disenchanting_table.hmod"
    echo "  embedded content: $(wc -c < "$cache" | tr -d ' ') B at $cache, identical to content/disenchanting_table.hmod"
}

# The control for EMBEDDED=1: the same binary (every hook is in it) with the
# embedded content made unavailable — the game folder's `.horizon-cache` is a
# plain file, so the helper can neither compare nor write its content file and
# falls back to `Continue`. Both ids must then be unknown; if they resolve, the
# run proves nothing about the embedded file.
control_leg() {
    say "control: the same game with no embedded content file"
    rm -rf "$CACHE"
    mkdir -p "$GAME_DIR"
    : > "$CACHE"
    launch "$CONTROL_WORLD"
    arena
    ctl command "give @s $TABLE 1" >/dev/null || true
    ctl wait 3 >/dev/null
    local gave
    gave=$(reply)
    echo "  give: $gave"
    case "$gave" in
        *"Unknown item '$TABLE'"*) ;;
        *) fail "the control game knows the table item: $gave" ;;
    esac
    ctl command "setblock $T1X $Y $T1Z $TABLE" >/dev/null || true
    ctl wait 3 >/dev/null
    local placed name
    placed=$(reply)
    echo "  setblock: $placed"
    case "$placed" in
        *"Unknown block type '$TABLE'"*) ;;
        *) fail "the control game knows the table block: $placed" ;;
    esac
    name=$(ctl block "$T1X" "$Y" "$T1Z" | jq -r '.block // "unknown"')
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the block id was known"
    ctl chat "/disenchanting_table info $T1X $Y $T1Z" >/dev/null
    ctl wait 3 >/dev/null
    local readout
    readout=$(reply)
    echo "  readout: $readout"
    case "$readout" in
        *"not a dis-enchanting table"*) ;;
        *) fail "the control's readout did not report a missing table: $readout" ;;
    esac
    [ -f "$CACHE" ] || fail "the control's .horizon-cache was replaced: the helper wrote its content"
    ctl quit >/dev/null 2>&1 || true
    sleep 2
    rm -f "$CACHE"
}

if [ "$CONTROL" = 1 ]; then control_leg; fi

say "launch (offscreen renderer, $([ "$EMBEDDED" = 1 ] && echo 'bundle only, no --dev-mod' || echo 'with --dev-mod content/'))"
launch "$WORLD"
if [ "$EMBEDDED" = 1 ]; then embedded_content_check; fi

say "an Overworld platform at y=100, away from the terrain"
arena

say "survival, at 135 experience points (level 9 exactly)"
ctl command "gamemode survival" >/dev/null
ctl command "xp add @s 135 points" >/dev/null
ctl wait 4
ctl command "clear @s" >/dev/null 2>&1 || true

say "the table places and starts empty"
place_table "$T1X" "$Y" "$T1Z" "$TABLE"
LINE=$(info "$T1X" "$Y" "$T1Z")
echo "readout: $LINE"
[ -n "$LINE" ] || fail "the table has no block entity: the readout is not a table line"
[ "$(field input)" = none ] || fail "the empty table has an input: $LINE"
[ "$(field output)" = none ] || fail "the empty table has an output: $LINE"
[ "$(field ready)" = false ] || fail "an empty table is ready: $LINE"

say "a sharpness-3 sword and a book go into the menu's slots"
place_table "$T2X" "$Y" "$T2Z" "$TABLE"
place_table "$T3X" "$Y" "$T3Z" "$TABLE"
place_table "$T4X" "$Y" "$T4Z" "$TABLE"
put_inputs "$T1X" "$Y" "$T1Z" "$SWORD" minecraft:sharpness 3
LINE=$(info "$T1X" "$Y" "$T1Z")
echo "readout: $LINE"
[ "$(field input)" = "$SWORD" ] || fail "the input slot did not take the sword: $LINE"
[ "$(field input_ench)" = "minecraft:sharpness:3" ] || fail "the enchant did not land on the sword: $LINE"
[ "$(field book)" = "$BOOK" ] || fail "the book slot did not take a book: $LINE"
[ "$(field output)" = none ] || fail "a result appeared before the click: $LINE"
[ "$(field ready)" = true ] || fail "the table is not ready with a sword and a book: $LINE"
[ "$(field cost)" = 25 ] || fail "the readout cost is $(field cost), expected 25"

say "the right-click dis-enchants and charges 25 points"
BEFORE=$(field xp)
echo "experience before: $BEFORE"
ctl interact "$T1X" "$Y" "$T1Z" >/dev/null || fail "the dis-enchant click failed"
ctl wait 10
LINE=$(info "$T1X" "$Y" "$T1Z")
echo "readout: $LINE"
[ "$(field input)" = "$SWORD" ] || fail "the stripped item is gone: $LINE"
[ "$(field input_ench)" = none ] || fail "the sword kept its enchantment: $LINE"
[ "$(field book)" = none ] || fail "the book was not spent: $LINE"
[ "$(field output)" = "minecraft:enchanted_book" ] || fail "no enchanted book was produced: $LINE"
[ "$(field output_ench)" = "minecraft:sharpness:3" ] || fail "the book does not carry sharpness 3: $LINE"
[ "$(field ready)" = false ] || fail "a done table is still ready: $LINE"
AFTER=$(field xp)
echo "experience after: $AFTER (cost 25)"
[ "$AFTER" = "$((BEFORE - 25))" ] || fail "the charge was $((BEFORE - AFTER)) points, expected 25"

say "the output book is taken through the menu"
open_menu "$T1X" "$Y" "$T1Z"
ctl click-slot 302 >/dev/null || fail "cannot take the output book"
ctl wait 2 >/dev/null
ctl close >/dev/null 2>&1 || true
ctl wait 4 >/dev/null
books=$(count_item minecraft:enchanted_book)
echo "enchanted books held: $books"
[ "$books" -ge 1 ] || fail "the produced book never reached the inventory"
LINE=$(info "$T1X" "$Y" "$T1Z")
[ "$(field output)" = none ] || fail "the output slot still holds the book: $LINE"

say "control: an unenchanted item never enters the input slot"
clear_hand
give_and_hold "$SWORD"
open_menu "$T2X" "$Y" "$T2Z"
put_in_slot 300 "$SWORD"
ctl close >/dev/null 2>&1 || true
ctl wait 4
LINE=$(info "$T2X" "$Y" "$T2Z")
echo "readout: $LINE"
[ "$(field input)" = none ] || fail "a plain sword entered the input slot: $LINE"
[ "$(count_item "$SWORD")" -ge 1 ] || fail "the refused plain sword was lost"
ctl interact "$T2X" "$Y" "$T2Z" >/dev/null || fail "the plain-item click failed"
ctl wait 8
ctl close >/dev/null 2>&1 || true
LINE=$(info "$T2X" "$Y" "$T2Z")
echo "after the click: $LINE"
[ "$(field output)" = none ] || fail "an empty table produced a book: $LINE"

say "control: an enchanted item with no book dis-enchants nothing"
clear_hand
give_and_hold "$SWORD" minecraft:sharpness 3
open_menu "$T3X" "$Y" "$T3Z"
put_in_slot 300 "$SWORD"
ctl close >/dev/null 2>&1 || true
ctl wait 4
LINE=$(info "$T3X" "$Y" "$T3Z")
echo "readout: $LINE"
[ "$(field book)" = none ] || fail "the no-book control has a book: $LINE"
[ "$(field ready)" = false ] || fail "a table with no book is ready: $LINE"
ctl interact "$T3X" "$Y" "$T3Z" >/dev/null || fail "the no-book click failed"
ctl wait 8
ctl close >/dev/null 2>&1 || true
LINE=$(info "$T3X" "$Y" "$T3Z")
echo "after the click: $LINE"
[ "$(field input_ench)" = "minecraft:sharpness:3" ] || fail "the no-book control lost the enchantment: $LINE"
[ "$(field output)" = none ] || fail "the no-book control produced a book: $LINE"

say "control: no experience, no dis-enchant, and nothing is lost"
put_inputs "$T4X" "$Y" "$T4Z" "$SWORD" minecraft:sharpness 3
# `xp set 0 points` only clears the bar's progress; taking points away
# (`giveExperiencePoints(-1000)`) clamps the level and the points at 0.
ctl command "xp add @s -1000 points" >/dev/null
ctl wait 4
LINE=$(info "$T4X" "$Y" "$T4Z")
echo "readout at 0 points: $LINE"
[ "$(field xp)" = 0 ] || fail "the player still has experience: $LINE"
ctl interact "$T4X" "$Y" "$T4Z" >/dev/null || fail "the poor-player click failed"
ctl wait 8
ctl close >/dev/null 2>&1 || true
LINE=$(info "$T4X" "$Y" "$T4Z")
echo "after the click: $LINE"
[ "$(field input_ench)" = "minecraft:sharpness:3" ] || fail "a poor player lost the enchantment: $LINE"
[ "$(field book)" = "$BOOK" ] || fail "a poor player lost the book: $LINE"
[ "$(field output)" = none ] || fail "a poor player got a book: $LINE"
[ "$(field xp)" = 0 ] || fail "the refused click still charged: $LINE"
ctl command "xp add @s 135 points" >/dev/null
ctl wait 4

say "control: a vanilla enchanting table is untouched"
place_table "$T5X" "$Y" "$T5Z" minecraft:enchanting_table
clear_hand
give_and_hold "$SWORD" minecraft:sharpness 3
ctl interact "$T5X" "$Y" "$T5Z" >/dev/null || fail "the vanilla table click failed"
ctl wait 6
SCREEN=$(ctl state | jq -r '.screen // ""')
echo "vanilla enchanting table screen: $SCREEN"
# The control protocol names a vanilla station menu `station`; the table's own
# mod menu is `container`, so this also proves the click did not reach it.
case "$SCREEN" in
    station|*enchant*) ;;
    *) fail "the vanilla enchanting table opened '$SCREEN', not a station menu" ;;
esac
ctl close >/dev/null 2>&1 || true
ctl wait 4
LINE=$(info "$T4X" "$Y" "$T4Z")
echo "the dis-enchanting table is unchanged: $LINE"
[ "$(field input_ench)" = "minecraft:sharpness:3" ] || fail "the vanilla table's click reached the dis-enchanting table: $LINE"

if [ "$SELFTEST" = 1 ]; then
    say "selftest: expect the stripped sword to have kept its enchantment (must fail)"
    LINE=$(info "$T1X" "$Y" "$T1Z")
    [ "$(field input_ench)" = "minecraft:sharpness:3" ] || fail "SELFTEST: the sword was stripped, so the check is alive"
    echo "SELFTEST unexpectedly saw the enchantment kept"
    exit 0
fi

echo "PASS: the table dis-enchants a sharpness-3 sword onto a book for exactly 25 points, spends the book and keeps the item; an unenchanted item, a missing book, no experience and a vanilla enchanting table all dis-enchant nothing and lose nothing"
