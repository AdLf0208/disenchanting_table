// Runs at the head of `Game::native_register_commands` (API 8): the table's
// readout.
fn register_readout(commands: &mut Vec<NativeCommand>) {
    crate::game::disenchanting_table_state::on_register_commands(commands);
}
