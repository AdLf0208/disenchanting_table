// Runs at the head of `Game::native_block_entity` (API 15): the table gets a
// block entity whose slots are its whole state.
fn observe_block_entity(block: &str, entity: &mut Option<(String, horizon_world::nbt::Compound)>) {
    crate::game::disenchanting_table_state::on_block_entity(block, entity);
}
