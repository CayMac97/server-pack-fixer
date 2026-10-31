// Hook helper: native_settings_click
fn server_pack_fixer_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    crate::game::mod_server_pack_fixer_glue::on_click(game, mod_id, button);
}
