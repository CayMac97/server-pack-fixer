// Hook helper: font::Region::place (cancellable HEAD)
fn server_pack_fixer_region_place(
    reg: &mut Region,
    w: u32,
    h: u32,
    atlas: (u32, u32),
    region: (u32, u32),
) -> core::ops::ControlFlow<Option<(u32, u32)>> {
    crate::mod_server_pack_fixer_render::on_region_place(
        &mut reg.cursor,
        &mut reg.row,
        w,
        h,
        atlas,
        region,
    )
}
