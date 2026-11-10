// Runs at the head of `Game::native_perform_command` (API 8): the readout.
fn run_readout(
    game: &mut crate::game::Game<'_>,
    player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    crate::game::disenchanting_table_state::on_perform_command(game, player, command, ran);
}
