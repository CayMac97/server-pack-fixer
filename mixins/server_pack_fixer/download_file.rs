// Hook helper: download::download_file (cancellable HEAD)
fn server_pack_fixer_download_file(
    dir: &Path,
    request: &Request,
    headers: &[(&'static str, String)],
    max_size: u64,
    progress: &Sender<Progress>,
) -> core::ops::ControlFlow<Result<PathBuf, String>> {
    crate::mod_server_pack_fixer_download::on_download_file(dir, request, headers, max_size, progress)
}
