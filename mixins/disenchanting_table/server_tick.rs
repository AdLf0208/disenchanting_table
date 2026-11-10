// Runs at the head of `Game::native_server_tick` (API 4): run the experience
// charges the clicks queued, where the console can take points.
fn run_charges(game: &mut crate::game::Game<'_>, phase: ServerTickPhase) {
    crate::game::disenchanting_table_state::on_server_tick(game, phase);
}
