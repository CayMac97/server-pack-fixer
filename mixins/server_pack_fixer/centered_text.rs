// Hook helper: ui::centered_text (cancellable HEAD)
fn server_pack_fixer_centered_text(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    text: &Text,
    x: f32,
    y: f32,
    w: f32,
    color: u32,
) -> core::ops::ControlFlow<()> {
    crate::mod_server_pack_fixer_render::on_centered_text(batch, atlas, text, x, y, w, color)
}
