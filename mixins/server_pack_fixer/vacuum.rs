// Hook helper: download::vacuum (cancellable HEAD)
fn server_pack_fixer_vacuum(dir: &Path, max_files: usize) -> core::ops::ControlFlow<()> {
    crate::mod_server_pack_fixer_download::on_vacuum(dir, max_files)
}
