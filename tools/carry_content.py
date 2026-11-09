#!/usr/bin/env python3
"""Carry the Dis-Enchanting Table 5.0.2 content into content/.

The jar is the ATM10 one, resolved by SHA-1 from Modrinth's pack record (see
../../_shared/carry-work/disenchanting_table/SOURCE.md). Asset bytes are copied
verbatim; the few data files 26.3 spells differently, and the block/item/menu/
creative-tab declarations the port adds, are written from the jar's own values.

Run: python3 tools/carry_content.py [--jar PATH] [--check]
  --check  compare the files on disk with what the carry would write, exit 1
           on a difference (no writes).
"""
from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_JAR = (
    ROOT.parent / "_shared/carry-work/disenchanting_table"
    / "disenchanting_table-merged-1.21.1-5.0.2.jar"
)
JAR_SHA1 = "c054a8bef63addff4d4a9520ec4c7a915c3fc1d1"
MOD = "disenchanting_table"
TABLE = "disenchanting_table"

# Bytes copied unchanged from the jar.
VERBATIM_FILES = [
    f"assets/{MOD}/blockstates/{TABLE}.json",
    f"assets/{MOD}/models/block/{TABLE}.json",
    f"assets/{MOD}/models/item/{TABLE}.json",
    f"assets/{MOD}/textures/block/{TABLE}_side.png",
    f"assets/{MOD}/textures/block/{TABLE}_top.png",
    f"assets/{MOD}/textures/gui/container/{TABLE}.png",
    f"data/minecraft/tags/block/mineable/pickaxe.json",
    f"data/minecraft/tags/block/needs_stone_tool.json",
]

MANIFEST = f"""\
# Port-owned content mod for the Dis-Enchanting Table 5.0.2 slice: the table
# block, its menu and carried assets. The models, textures and blockstate are
# the upstream mod's own Unlicense assets, read from the exact ATM10 jar (see
# ../../_shared/carry-work/disenchanting_table/SOURCE.md).
id = "{MOD}"
name = "Dis-Enchanting Table"
version = "5.0.2"
api = "0.2"
side = "both"
capabilities = ["registries"]
"""

# `Properties.ofFullCopy(Blocks.ENCHANTING_TABLE)`: hardness 5.0, blast
# resistance 1200, light 7, stone sounds; the jar's own data adds the pickaxe
# and stone-tool tags, so the block drops only to a stone pickaxe.
BLOCK_DECL = {
    "template": "minecraft:barrel",
    "properties": {
        "hardness": 5.0,
        "explosion_resistance": 1200.0,
        "light": 7,
        "sounds": "minecraft:stone",
        "requires_tool": True,
        "occludes": False,
    },
    # `Block.box(0, 0, 0, 16, 12, 16)`: twelve pixels tall.
    "shapes": {"default": [[0.0, 0.0, 0.0, 1.0, 0.75, 1.0]]},
    "collision_shapes": {"default": [[0.0, 0.0, 0.0, 1.0, 0.75, 1.0]]},
}

# Input, book, output; the output is a `results` slot, so players only take.
MENU_DECL = {"slots": 3, "results": [2]}

CREATIVE_TAB = {
    "title": "itemGroup.disenchantingTable",
    "icon": f"{MOD}:{TABLE}",
    "items": [f"{MOD}:{TABLE}"],
}

ITEM_DEF = {
    "model": {"type": "minecraft:model", "model": f"{MOD}:item/{TABLE}"},
}

LANG = {
    "itemGroup.disenchantingTable": "Dis-Enchanting Table",
    f"block.{MOD}.{TABLE}": "Dis-Enchanting Table",
    f"container.{MOD}.{TABLE}": "Dis-Enchanting Table",
}

# The jar's pool is the 1.20 shape (`pool.conditions` array); 26.3 reads a
# pool-level `condition` object (factory_blocks, 2026-11-08: the old shape's
# pool is skipped and the block drops nothing).
LOOT_TABLE = {
    "type": "minecraft:block",
    "pools": [
        {
            "rolls": 1.0,
            "entries": [{"name": f"{MOD}:{TABLE}", "type": "minecraft:item"}],
            "condition": {"type": "minecraft:survives_explosion"},
        }
    ],
}

# The jar's shaped recipe (obsidian, lapis lazuli, diamond), in 26.3's spelling.
RECIPE = {
    "type": "minecraft:crafting_shaped",
    "category": "misc",
    "group": f"{MOD}:{TABLE}",
    "pattern": [" L ", "D#D", "###"],
    "key": {
        "#": "minecraft:obsidian",
        "L": "minecraft:lapis_lazuli",
        "D": "minecraft:diamond",
    },
    "result": {"id": f"{MOD}:{TABLE}", "count": 1},
}


def pretty(value) -> bytes:
    return (json.dumps(value, indent=2, sort_keys=False) + "\n").encode()


def carried_files(jar: zipfile.ZipFile) -> dict[str, bytes]:
    """Every file the carry writes: path -> bytes."""
    out: dict[str, bytes] = {}
    for path in VERBATIM_FILES:
        out[path] = jar.read(path)
    out[f"data/{MOD}/horizon/block/{TABLE}.json"] = pretty(BLOCK_DECL)
    out[f"data/{MOD}/horizon/menu/{TABLE}.json"] = pretty(MENU_DECL)
    out[f"data/{MOD}/horizon/creative_tab/{TABLE}.json"] = pretty(CREATIVE_TAB)
    out[f"assets/{MOD}/items/{TABLE}.json"] = pretty(ITEM_DEF)
    out[f"assets/{MOD}/lang/en_us.json"] = pretty(LANG)
    out[f"data/{MOD}/loot_table/blocks/{TABLE}.json"] = pretty(LOOT_TABLE)
    out[f"data/{MOD}/recipe/{TABLE}.json"] = pretty(RECIPE)
    out["horizon-mod.toml"] = MANIFEST.encode()
    return out


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jar", type=Path, default=DEFAULT_JAR)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    jar_path = args.jar.resolve()
    if not jar_path.exists():
        raise SystemExit(f"missing jar: {jar_path}")
    digest = hashlib.sha1(jar_path.read_bytes()).hexdigest()
    if digest != JAR_SHA1:
        raise SystemExit(f"jar SHA-1 is {digest}, expected {JAR_SHA1}")
    content = ROOT / "content"
    with zipfile.ZipFile(jar_path) as jar:
        files = carried_files(jar)
    pending = 0
    for name, data in sorted(files.items()):
        target = content / name
        current = target.read_bytes() if target.exists() else None
        if current == data:
            continue
        if args.check:
            print(f"stale: {name}", file=sys.stderr)
            pending += 1
        else:
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(data)
    if args.check:
        if pending:
            return 1
        print(f"carry check: {len(files)} files, 0 pending, jar SHA-1 {JAR_SHA1}")
        return 0
    print(f"carry: wrote {len(files)} files from jar SHA-1 {JAR_SHA1}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
