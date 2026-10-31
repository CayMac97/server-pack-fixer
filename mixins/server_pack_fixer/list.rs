// Hook helper: native_mods
fn server_pack_fixer_list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    crate::game::mod_server_pack_fixer_glue::on_list(mods);
}
