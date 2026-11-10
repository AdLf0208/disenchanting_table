// Runs at the head of `Game::native_right_click_block` (API 21): dis-enchant
// when the table is ready, otherwise let the click open its menu.
fn disenchant_on_click(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face: [i32; 3],
    canceled: &mut bool,
) {
    crate::game::disenchanting_table_state::on_right_click(game, pos, face, canceled);
}
