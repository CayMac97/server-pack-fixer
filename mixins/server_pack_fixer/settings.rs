// Hook helper: native_settings
fn server_pack_fixer_settings(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    buttons: &mut Vec<String>,
) {
    crate::game::mod_server_pack_fixer_glue::on_settings(game, mod_id, buttons);
}
