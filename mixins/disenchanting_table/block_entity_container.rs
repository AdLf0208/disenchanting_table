// Runs at the head of `Game::native_block_entity_container` (API 16/17): three
// slots (input, book, output) and the table's menu.
fn observe_container(
    block: &str,
    container: &mut Option<horizon_client::containers::ModContainer>,
) {
    crate::game::disenchanting_table_state::on_container(block, container);
}
