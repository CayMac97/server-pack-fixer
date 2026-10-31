// Hook helper: Game::remote_chunk_in_range (cancellable HEAD)
fn server_pack_fixer_remote_chunk_in_range(
    game: &crate::game::Game<'_>,
    x: i32,
    z: i32,
) -> core::ops::ControlFlow<bool> {
    let center = game
        .remote
        .as_ref()
        .map(|r| (r.chunk_center, r.view_distance));
    match crate::game::mod_server_pack_fixer_glue::on_remote_chunk_in_range_head(game, x, z, center)
    {
        Some(v) => core::ops::ControlFlow::Break(v),
        None => core::ops::ControlFlow::Continue(()),
    }
}
