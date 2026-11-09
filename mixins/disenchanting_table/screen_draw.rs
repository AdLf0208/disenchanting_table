// Runs at the head of `Game::native_screen_draw` (API 19): the experience cost
// under the book slot.
fn draw_cost(
    game: &crate::game::Game<'_>,
    block: &str,
    data: &[i32],
    batch: &mut horizon_client::hud::HudBatch,
    origin: [f32; 2],
) {
    crate::game::disenchanting_table_state::on_screen_draw(game, block, data, batch, origin);
}
