// Hook helper: native_key
fn server_pack_fixer_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    crate::game::mod_server_pack_fixer_glue::on_key(game, key, pressed);
}
