// Hook helper: hud::HudBatch::text_styled_world (cancellable HEAD)
fn server_pack_fixer_text_styled_world(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    text: &crate::text::Text,
    x: f32,
    y: f32,
    color: u32,
    shadow: bool,
) -> core::ops::ControlFlow<f32> {
    let start_len = batch.vertices.len();
    let ret = batch.styled_text(atlas, text, x, y, color, shadow, true);
    crate::mod_server_pack_fixer_render::on_text_styled_world_after(batch, start_len);
    core::ops::ControlFlow::Break(ret)
}
