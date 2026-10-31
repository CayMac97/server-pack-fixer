// Hook helper: packs::server_pack (cancellable HEAD)
fn server_pack_fixer_server_pack(
    id: u128,
    path: &Path,
) -> core::ops::ControlFlow<Option<Pack>> {
    crate::mod_server_pack_fixer_render::on_server_pack_head(id, path)
}
