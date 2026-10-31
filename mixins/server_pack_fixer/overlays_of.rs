// Hook helper: packs::overlays_of (cancellable HEAD)
fn server_pack_fixer_overlays_of(
    meta: &serde_json::Value,
) -> core::ops::ControlFlow<Vec<String>> {
    crate::mod_server_pack_fixer_render::on_overlays_of_head(meta)
}
