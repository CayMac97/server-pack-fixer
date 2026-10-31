// Hook helper: ui::nine_slice (cancellable HEAD)
fn server_pack_fixer_nine_slice(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    sprite: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: u32,
) -> core::ops::ControlFlow<()> {
    let has_meta = nine_slice_metadata(sprite).is_some();
    let meta_loaded = NINE_SLICE.read().unwrap_or_else(|e| e.into_inner()).is_some();
    crate::mod_server_pack_fixer_render::on_nine_slice(
        batch,
        atlas,
        sprite,
        x,
        y,
        w,
        h,
        color,
        has_meta,
        meta_loaded,
    )
}
