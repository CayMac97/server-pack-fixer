// Hook helper: packs::build_overlay (RETURN)
fn server_pack_fixer_build_overlay(
    vanilla: &Path,
    packs: &[Pack],
    out: &Path,
    ret: io::Result<()>,
) -> io::Result<()> {
    crate::mod_server_pack_fixer_render::on_build_overlay(vanilla, packs, out, ret)
}
