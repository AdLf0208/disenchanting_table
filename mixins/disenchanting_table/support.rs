// Game-side glue for the Dis-Enchanting Table slice 1.
//
// Injected into `crate::game` as `disenchanting_table_state`; it needs the
// game, so it is not part of the standalone library (only `runtime.rs` is).
//
// The table's block entity keeps three slots (input, book, output) and no
// saved fields of its own: every value the port needs is derived from the
// slots, so a write inside a tick can never be reverted (the pin drops those,
// `moderndynamics` item 77). A right-click with an eligible input and a book
// performs upstream's `onTake` in one step — strip or split the input, spend
// one book, put the produced enchanted book in the output slot, charge the
// experience — and any refusal leaves all three slots exactly as they were.
//
// Read from Dis-Enchanting Table 5.0.2 (`DisEnchantingMenu.createResult` /
// `onTake`, `DisEnchantingStrategy`, `DisEnchantingTableBlockEntity`); see
// `../../FINDINGS.md`.

use std::sync::{Arc, Mutex, PoisonError};

use horizon_client::containers::{BlockEntity, ModBlockEntity, ModContainer};
use horizon_client::hud::HudBatch;
use horizon_client::item_data::Levels;
use horizon_client::items::ItemStack;
use horizon_client::text::Text;
use horizon_world::nbt::Compound;

use crate::game::disenchanting_table::{
    BOOK_SLOT, Charge, DEFAULT_SETTINGS, ENCHANTED_BOOK_ITEM, INPUT_SLOT, OUTPUT_SLOT, Plan,
    SLOT_COUNT, TABLE_MENU, affordable, can_disenchant, charge, is_book, is_table, plan,
};
use crate::game::native_hooks::NativeScreen;
use crate::game::native_lifecycle::ServerTickPhase;
use crate::game::native_server::{NativeCommand, NativeServerPlayer};
use crate::game::Game;

/// An experience charge waiting for a server tick: the console is the only
/// surface that takes points (`native_give_experience` only gives them, and
/// `native_run_command` refuses inside a player's hook).
struct PendingCharge {
    name: String,
    points: u32,
}

static PENDING: Mutex<Vec<PendingCharge>> = Mutex::new(Vec::new());

// ---------------------------------------------------------------------------
// Declaration hooks
// ---------------------------------------------------------------------------

/// Give the table a block entity (`native_block_entity`, API 15). It saves
/// nothing of its own: the slots are the whole state.
#[allow(private_interfaces)]
pub fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if is_table(block) {
        *entity = Some((block.to_owned(), Compound::new()));
    }
}

/// Three slots and the table's menu (`native_block_entity_container`,
/// API 16/17). Upstream lets a hopper put an input and a book in and take a
/// dis-enchanted item out (`DisEnchantingStrategy.canOutputToHopper`), so every
/// face reaches all three slots, the input and book slots fill, and the input
/// and output slots empty.
#[allow(private_interfaces)]
pub fn on_container(block: &str, container: &mut Option<ModContainer>) {
    if !is_table(block) {
        return;
    }
    let all = [0, 1, 2];
    *container = Some(ModContainer {
        slots: SLOT_COUNT,
        faces: std::array::from_fn(|_| all.to_vec()),
        inputs: vec![INPUT_SLOT, BOOK_SLOT],
        outputs: vec![INPUT_SLOT, OUTPUT_SLOT],
        menu: Some(TABLE_MENU.to_owned()),
    });
}

// ---------------------------------------------------------------------------
// Client side: the screen
// ---------------------------------------------------------------------------

/// The table's three-slot layout (`native_screen`, API 19): `DisEnchantingMenu`'s
/// 27 / 76 / 134 at y 47, in a 176×166 screen.
#[allow(private_interfaces)]
pub fn on_screen(block: &str, screen: &mut Option<NativeScreen>) {
    if !is_table(block) {
        return;
    }
    *screen = Some(NativeScreen {
        width: 176.0,
        height: 166.0,
        slots: vec![
            Some([27.0, 47.0]),
            Some([76.0, 47.0]),
            Some([134.0, 47.0]),
        ],
        inventory_y: 84.0,
        inventory_x: 8.0,
        own_labels: false,
        // The jar's GUI texture is carried but not drawn: Horizon's plain
        // panel and slot frames are used (a gap, see `notes.md`).
        own_background: false,
    });
}

/// Draws the cost under the book slot (`native_screen_draw`, API 19).
#[allow(private_interfaces)]
pub fn on_screen_draw(
    game: &Game<'_>,
    block: &str,
    _data: &[i32],
    batch: &mut HudBatch,
    origin: [f32; 2],
) {
    if !is_table(block) {
        return;
    }
    let cost = match charge(DEFAULT_SETTINGS, false) {
        Charge::Points(points) => format!("{points} XP"),
        Charge::Levels(levels) => format!("{levels} levels"),
        Charge::Nothing => "free".to_owned(),
    };
    game.native_text(
        batch,
        &cost,
        [origin[0] + 8.0, origin[1] + 68.0],
        0x404040,
        false,
    );
}

// ---------------------------------------------------------------------------
// The dis-enchant
// ---------------------------------------------------------------------------

/// Right click (`native_right_click_block`, API 21): with an eligible input, a
/// book and an empty output slot, do upstream's take in one step and consume
/// the click. Anything else falls through to the barrel template's use path,
/// which opens the table's menu.
#[allow(private_interfaces)]
pub fn on_right_click(
    game: &mut Game<'_>,
    pos: [i32; 3],
    _face: [i32; 3],
    canceled: &mut bool,
) {
    if !is_table(&game.native_block(pos)) {
        return;
    }
    let Some(player) = game.native_acting_player() else {
        return;
    };
    let Some(slots) = read_slots(game, pos) else {
        return;
    };
    // The produced book waits in the output slot; a click then opens the menu.
    if slots.output.is_some() {
        return;
    }
    let Some(input) = slots.input.as_ref() else {
        return;
    };
    if slots.book.is_none() {
        return;
    }
    let levels = levels_of(input);
    let Some(disenchant) = plan(&input.item, &levels) else {
        return;
    };
    let cost = charge(DEFAULT_SETTINGS, player.game_mode == 1);
    if !affordable(cost, player.xp_level, player.xp_progress) {
        game.native_action_bar(
            player.uuid,
            Text::literal("Not enough experience to dis-enchant"),
        );
        return;
    }
    let moved = match &disenchant {
        Plan::Whole { moved } => moved.clone(),
        Plan::Split { moved, .. } => moved.clone(),
    };
    let applied = game.native_with_mod_block_entity(pos, |entity| {
        finish(entity, &disenchant, &moved)
    });
    if applied.is_none() {
        return;
    }
    match cost {
        Charge::Points(points) => queue_charge(&player.name, points),
        Charge::Levels(levels) => {
            let levels = i32::try_from(levels).unwrap_or(i32::MAX);
            let _ = game.native_give_levels(player.uuid, -levels);
        }
        Charge::Nothing => {}
    }
    *canceled = true;
}

/// Strips or splits the input, spends one book and puts the produced book in
/// the output slot — upstream's `onTake` in one step. False (and nothing
/// written) when the output slot is not free.
fn finish(entity: &mut ModBlockEntity, disenchant: &Plan, moved: &[(String, u32)]) -> bool {
    if entity.items.len() < SLOT_COUNT {
        entity.items.resize(SLOT_COUNT, None);
    }
    if entity.items[OUTPUT_SLOT].is_some() {
        return false;
    }
    if let Some(mut input) = entity.items[INPUT_SLOT].take() {
        match disenchant {
            Plan::Whole { .. } => {
                input.set_crafting_enchantments(Vec::new());
                // `resets_repair_cost`: no prior-work penalty on a stripped item.
                if DEFAULT_SETTINGS.resets_repair_cost {
                    input.data_mut().repair_cost = 0;
                    input.tidy();
                }
            }
            // A book keeps its first enchantment (`onTake`'s book branch).
            Plan::Split { kept, .. } => {
                input.set_crafting_enchantments(levels_of_pairs(std::slice::from_ref(kept)));
            }
        }
        entity.items[INPUT_SLOT] = Some(input);
    }
    if let Some(mut book) = entity.items[BOOK_SLOT].take() {
        book.count = book.count.saturating_sub(1);
        entity.items[BOOK_SLOT] = (book.count > 0).then_some(book);
    }
    let mut output = ItemStack {
        item: ENCHANTED_BOOK_ITEM.into(),
        count: 1,
        damage: 0,
        extra: None,
    };
    output.set_crafting_enchantments(levels_of_pairs(moved));
    entity.items[OUTPUT_SLOT] = Some(output);
    true
}

fn queue_charge(name: &str, points: u32) {
    let mut guard = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
    guard.push(PendingCharge {
        name: name.to_owned(),
        points,
    });
}

// ---------------------------------------------------------------------------
// The deferred experience charge
// ---------------------------------------------------------------------------

/// End of a running server tick (`native_server_tick`, API 4): run the charges
/// the clicks queued. `native_run_command` (API 21) is the only surface that
/// takes experience points, and it refuses inside a player's hook, so the click
/// cannot charge itself.
#[allow(private_interfaces)]
pub fn on_server_tick(game: &mut Game<'_>, phase: ServerTickPhase) {
    let ServerTickPhase::ServerTickEnd { world_running } = phase else {
        return;
    };
    if !world_running {
        return;
    }
    let pending = {
        let mut guard = PENDING.lock().unwrap_or_else(PoisonError::into_inner);
        std::mem::take(&mut *guard)
    };
    for charge in pending {
        let line = format!("xp add {} -{} points", charge.name, charge.points);
        let _ = game.native_run_command(&line);
    }
}

// ---------------------------------------------------------------------------
// The `/disenchanting_table info` readout
// ---------------------------------------------------------------------------

/// The readout command (`native_register_commands`, API 8). The rest of the
/// line is one greedy text argument, so any coordinates (negative included)
/// parse and the handler reads the words itself.
#[allow(private_interfaces)]
pub fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("disenchanting_table <args:text>"));
}

/// `/disenchanting_table info <x> <y> <z>` (`native_perform_command`, API 8):
/// the three slots, the cost and the acting player's experience, for the
/// harness and for players.
#[allow(private_interfaces)]
pub fn on_perform_command(
    game: &mut Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    if words.first() != Some(&"disenchanting_table") {
        return;
    }
    *ran = true;
    let parsed = (2..5)
        .map(|index| words.get(index).and_then(|word| word.parse::<i32>().ok()))
        .collect::<Option<Vec<i32>>>();
    let Some(parsed) = parsed.filter(|_| words.get(1) == Some(&"info")) else {
        game.native_feedback(Text::literal(
            "usage: /disenchanting_table info <x> <y> <z>",
        ));
        return;
    };
    let pos = [parsed[0], parsed[1], parsed[2]];
    let line = readout(game, pos);
    game.native_feedback(Text::literal(line));
}

/// One line: `input`, `book` and `output` with their enchantments, the cost,
/// the player's points and level, and whether the slots hold what a
/// dis-enchant needs (the experience is reported next to it, not folded in).
fn readout(game: &Game<'_>, pos: [i32; 3]) -> String {
    let block = game.native_block(pos);
    let Some(slots) = read_slots(game, pos) else {
        return format!("{block}: not a dis-enchanting table");
    };
    let cost = match charge(DEFAULT_SETTINGS, false) {
        Charge::Points(points) => u64::from(points),
        Charge::Levels(levels) => u64::from(levels),
        Charge::Nothing => 0,
    };
    let (points, level) = game.native_acting_player().map_or((0, 0), |player| {
        (
            crate::game::disenchanting_table::total_points(player.xp_level, player.xp_progress),
            player.xp_level,
        )
    });
    let ready = slots.output.is_none()
        && slots
            .input
            .as_ref()
            .is_some_and(|input| can_disenchant(&input.item, &levels_of(input)))
        && slots.book.as_ref().is_some_and(|book| is_book(&book.item));
    format!(
        "{block}: input={} input_ench={} book={} output={} output_ench={} cost={cost} xp={points} level={level} ready={ready}",
        item_name(slots.input.as_ref()),
        enchant_list(slots.input.as_ref()),
        item_name(slots.book.as_ref()),
        item_name(slots.output.as_ref()),
        enchant_list(slots.output.as_ref()),
    )
}

// ---------------------------------------------------------------------------
// Shared reads
// ---------------------------------------------------------------------------

/// The table's three slots, cloned out of its block entity.
struct TableSlots {
    input: Option<ItemStack>,
    book: Option<ItemStack>,
    output: Option<ItemStack>,
}

fn read_slots(game: &Game<'_>, pos: [i32; 3]) -> Option<TableSlots> {
    let entity = match game.world.block_entities.get(&(pos[0], pos[1], pos[2])) {
        Some(BlockEntity::Mod(entity)) if is_table(&entity.id) => entity,
        _ => return None,
    };
    let slot = |index: usize| {
        entity
            .items
            .get(index)
            .and_then(|stack| stack.as_ref())
            .filter(|stack| stack.count > 0)
            .cloned()
    };
    Some(TableSlots {
        input: slot(INPUT_SLOT),
        book: slot(BOOK_SLOT),
        output: slot(OUTPUT_SLOT),
    })
}

fn levels_of(stack: &ItemStack) -> Vec<(&str, u32)> {
    stack
        .crafting_enchantments()
        .iter()
        .map(|(id, level)| (&**id, *level))
        .collect()
}

fn levels_of_pairs(levels: &[(String, u32)]) -> Levels {
    levels
        .iter()
        .map(|(id, level)| (Arc::from(id.as_str()), *level))
        .collect()
}

fn item_name(stack: Option<&ItemStack>) -> &str {
    stack.map_or("none", |stack| &stack.item)
}

fn enchant_list(stack: Option<&ItemStack>) -> String {
    let Some(stack) = stack else {
        return "none".to_owned();
    };
    let levels = levels_of(stack);
    if levels.is_empty() {
        return "none".to_owned();
    }
    levels
        .iter()
        .map(|(id, level)| format!("{id}:{level}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// The book item the extra slot takes, for the readout's own checks.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::disenchanting_table::BOOK_ITEM;

    #[test]
    fn the_readout_lists_enchantments_compactly() {
        let mut stack = ItemStack {
            item: "minecraft:diamond_sword".into(),
            count: 1,
            damage: 0,
            extra: None,
        };
        stack.set_crafting_enchantments(vec![
            (Arc::from("minecraft:sharpness"), 3),
            (Arc::from("minecraft:unbreaking"), 2),
        ]);
        assert_eq!(
            enchant_list(Some(&stack)),
            "minecraft:sharpness:3,minecraft:unbreaking:2"
        );
        assert_eq!(enchant_list(None), "none");
    }

    #[test]
    fn a_book_s_levels_are_its_stored_ones() {
        let mut book = ItemStack {
            item: ENCHANTED_BOOK_ITEM.into(),
            count: 1,
            damage: 0,
            extra: None,
        };
        book.set_crafting_enchantments(vec![(Arc::from("minecraft:mending"), 1)]);
        assert_eq!(levels_of(&book), vec![("minecraft:mending", 1)]);
        // A plain book has none.
        let plain = ItemStack {
            item: BOOK_ITEM.into(),
            count: 1,
            damage: 0,
            extra: None,
        };
        assert!(levels_of(&plain).is_empty());
    }
}
