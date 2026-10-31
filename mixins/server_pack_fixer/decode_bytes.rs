// Hook helper: hud::decode_bytes (cancellable HEAD)
fn server_pack_fixer_decode_bytes(
    data: &[u8],
) -> core::ops::ControlFlow<Result<(u32, u32, Vec<u8>)>> {
    crate::mod_server_pack_fixer_render::on_decode_bytes(data)
}
