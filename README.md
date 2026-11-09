# Dis-Enchanting Table — Horizon port

A port of [Dis-Enchanting Table](https://github.com/Mods-For-Lupin/Dis-Enchanting-Redux)
by Lupin and Jason13 for Horizon. The table strips enchantments off an item and
writes them onto a book.

- Place the table and right-click it: put an enchanted item in the input slot,
  a `minecraft:book` in the book slot, and an empty slot for the result.
- A right click moves the enchantments onto a new enchanted book and costs 25
  experience points; an enchanted book needs at least two enchantments to be
  worth dis-enchanting. A missing book, missing experience or an occupied output
  leaves everything as it was.
- The table keeps the jar's light 7, its 12-pixel shape, its stone-pickaxe
  harvest level, and hoppers may fill the input and book slots.
- `/disenchanting_table info <x> <y> <z>` prints the three slots for testing.

Unlicense — original copyright and licence notices are kept in the port
repository; see LICENSE and LICENSE-NOTICE.md there.

**Original:** [Dis-Enchanting Table](https://github.com/Mods-For-Lupin/Dis-Enchanting-Redux) 5.0.2 by
Lupin, Jason13, **Unlicense**. The original
copyright and licence notices are kept in [LICENSE](LICENSE), and
[LICENSE-NOTICE.md](LICENSE-NOTICE.md) records where each was read from.

**Horizon:** a port for [Horizon](https://github.com/horizon-rust/horizon), Minecraft:
Java Edition 26.3 rewritten in Rust. The exact Horizon and horizon-mixins
revisions this port builds against are pinned in `compatibility.json`.

## Install

1. Take `disenchanting_table.hnmod` from this repository (or build it below).
2. Put it in the game folder's `horizon-native-mods` folder — the game folder is the one
   holding `options.txt`.
3. Native mods are always on once installed; the first install shows a
   trust notice, and the launcher builds the mod for your game.

To try it without installing, start the game with
`--dev-mod disenchanting_table.hnmod`.

## Build

The port is a native mod (`.hnmod`): carried content plus a Rust runtime and
source mixins.

```sh
cargo test --locked --offline                        # the pure runtime rules
python3 tools/horizon.py stage   --horizon <horizon-checkout>
python3 tools/horizon.py check   --horizon <horizon-checkout>
python3 tools/horizon.py test    --horizon <horizon-checkout>
python3 tools/horizon.py build   --horizon <horizon-checkout>   # rebuild the .hnmod
```

`tools/e2e.sh` is the in-game harness. It expects a campaign checkout
(`$CAMPAIGN_ROOT`), the shared pins under `ports/_shared/pins/`, and the
prepared 26.3 resources (`$HORIZON_RES`, default `.local/horizon-client/26.3`).

## Layout

`content/` is the shipped content; `mixins/disenchanting_table/` holds the declarations and the runtime; `tools/` builds and verifies the port.

This is an unofficial port: Horizon is not affiliated with the original author.
The licence above is the original's and travels with every copy; see
[LICENSE](LICENSE) and [LICENSE-NOTICE.md](LICENSE-NOTICE.md).
