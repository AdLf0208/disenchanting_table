# Licence notice — Dis-Enchanting Table

This port carries the original mod's assets and data, and rewrites its behaviour
in Rust (or, for a data-tier port, declares its blocks from the same data). The
original licence and copyright notices are reproduced verbatim in
[LICENSE](LICENSE).

## Where the licence was read

- The pack jar `disenchanting_table-merged-1.21.1-5.0.2.jar` (kept at
  `licence-cache/`) declares `license = "Unlicense"` in both
  `META-INF/mods.toml` and `META-INF/neoforge.mods.toml` and carries no licence
  file.
- The text is the verbatim Unlicense (<https://unlicense.org>). The authors are
  the jar's `authors = "Lupin, Jason13"`.

## The port

- Original: Dis-Enchanting Table 5.0.2 by Lupin and Jason13 — Unlicense.
- Licence: **Unlicense**
- Port to Horizon by AdLf0208: the Rust runtime, the Horizon declarations and the
  build/verification tooling in this repository.
- No Mojang file is included. The carried files are the mod's own assets and
  data; `data/minecraft/tags/…` files only add this mod's ids to vanilla tags.
