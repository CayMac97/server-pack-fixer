// Hook helper: native_hud
fn server_pack_fixer_hud(
    game: &mut crate::game::Game<'_>,
    batch: &mut horizon_client::hud::HudBatch,
    corners: &mut horizon_client::extras_hud::Corners,
) {
    crate::game::mod_server_pack_fixer_glue::on_hud(game, batch, corners);
}
