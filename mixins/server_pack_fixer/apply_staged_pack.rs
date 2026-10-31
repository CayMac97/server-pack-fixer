// Hook helper: Game::apply_staged_pack (HEAD)
fn server_pack_fixer_apply_staged_pack(game: &mut crate::game::Game<'_>) {
    crate::game::mod_server_pack_fixer_glue::on_apply_staged_pack_head(game);
}
