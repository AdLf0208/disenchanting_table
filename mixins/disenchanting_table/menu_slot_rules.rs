// Runs at the head of `native_menu_slot_rules` (API 130, in `menu.rs`, which
// both the game crate and `horizon-client-lib` compile): the table's slot
// rules, from the pure engine injected beside this function as
// `disenchanting_table_rules`.
fn slot_rules(
    menu: &str,
    slot: usize,
    stack: &crate::items::ItemStack,
    may_place: &mut bool,
    max_stack: &mut u32,
) {
    let levels: Vec<(&str, u32)> = stack
        .crafting_enchantments()
        .iter()
        .map(|(id, level)| (&**id, *level))
        .collect();
    crate::menu::disenchanting_table_rules::slot_rule(
        menu,
        slot,
        &stack.item,
        &levels,
        may_place,
        max_stack,
    );
}
