// Runs at the head of `Game::native_screen` (API 19): the table's three-slot
// layout.
fn observe_screen(block: &str, screen: &mut Option<NativeScreen>) {
    crate::game::disenchanting_table_state::on_screen(block, screen);
}
