// Hook helper: font::Font::split_formatted (cancellable HEAD)
fn server_pack_fixer_split_formatted(
    font: &Font,
    t: &Text,
    max_width: f32,
) -> core::ops::ControlFlow<Vec<Text>> {
    crate::mod_server_pack_fixer_render::on_split_formatted(font, t, max_width)
}
