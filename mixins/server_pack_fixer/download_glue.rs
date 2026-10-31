// Support module injected into `crates/horizon-client/src/lib.rs` as
// `pub mod mod_server_pack_fixer_download;`.
// Implements resilient multi-pack downloads (3 retries + timeouts + fallback headers) and expands
// the download cache limit from 20 to 256 files.

use crate::server_packs::{
    Request,
    download::Progress,
};
use core::ops::ControlFlow;
use horizon_client_render::mod_server_pack_fixer as m;
use sha1::{Digest, Sha1};
use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    time::{Duration, SystemTime},
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn sha1_of(path: &Path) -> io::Result<String> {
    let mut hasher = Sha1::new();
    io::copy(&mut fs::File::open(path)?, &mut hasher)?;
    Ok(hex(&hasher.finalize()))
}

fn matches_sha1(path: &Path, sha1: &str) -> bool {
    path.exists() && sha1_of(path).is_ok_and(|actual| actual.eq_ignore_ascii_case(sha1))
}

fn touch(path: &Path) {
    let _ = fs::File::options()
        .append(true)
        .open(path)
        .and_then(|f| f.set_modified(SystemTime::now()));
}

fn rand_suffix() -> u64 {
    use std::hash::{BuildHasher, RandomState};
    RandomState::new().hash_one(std::time::Instant::now())
}

fn save_hashed(
    input: &mut dyn Read,
    path: &Path,
    max_size: u64,
    progress: &mut dyn FnMut(u64),
) -> io::Result<String> {
    let mut out = fs::File::create(path)?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 32_768];
    let mut read_so_far = 0u64;
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        read_so_far += n as u64;
        progress(read_so_far);
        if read_so_far > max_size {
            return Err(io::Error::other(format!(
                "Filesize was bigger than maximum allowed (got >= {read_so_far}, limit was {max_size})"
            )));
        }
        out.write_all(&buffer[..n])?;
        hasher.update(&buffer[..n]);
    }
    Ok(hex(&hasher.finalize()))
}

fn attempt_download(
    dir: &Path,
    request: &Request,
    headers: &[(&'static str, String)],
    max_size: u64,
    progress: &Sender<Progress>,
    attempt: usize,
) -> Result<PathBuf, String> {
    let target = request.sha1.as_ref().map(|sha1| dir.join(sha1));
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(12))
        .timeout_read(Duration::from_secs(45))
        .build();
    let mut call = agent.get(&request.url);
    for (name, value) in headers {
        call = call.set(name, value);
    }
    if attempt > 0 {
        call = call.set("Accept", "*/*");
    }
    let response = call.call().map_err(|e| e.to_string())?;
    let size = response
        .header("Content-Length")
        .and_then(|v| v.trim().parse::<u64>().ok());
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    if attempt == 0 {
        let _ = progress.send(Progress::DownloadStart(size));
    }
    if let Some(size) = size
        && size > max_size
    {
        return Err(format!(
            "Filesize is bigger than maximum allowed (file is {size}, limit is {max_size})"
        ));
    }
    let mut input = response.into_reader();
    let mut report = |written: u64| {
        let _ = progress.send(Progress::Bytes(written));
    };
    match (&target, &request.sha1) {
        (Some(target), Some(sha1)) => {
            let tmp = dir.join(format!("download{:016x}.tmp", rand_suffix()));
            let actual = match save_hashed(&mut input, &tmp, max_size, &mut report) {
                Ok(h) => h,
                Err(e) => {
                    let _ = fs::remove_file(&tmp);
                    return Err(e.to_string());
                }
            };
            if !actual.eq_ignore_ascii_case(sha1) {
                let _ = fs::remove_file(&tmp);
                return Err(format!(
                    "Hash of downloaded file ({actual}) did not match requested ({sha1})"
                ));
            }
            let _ = fs::remove_file(target);
            fs::rename(&tmp, target).map_err(|e| {
                let _ = fs::remove_file(&tmp);
                e.to_string()
            })?;
            Ok(target.clone())
        }
        _ => {
            let tmp = dir.join(format!("download{:016x}.tmp", rand_suffix()));
            let result = save_hashed(&mut input, &tmp, max_size, &mut report).and_then(|actual| {
                let path = dir.join(&actual);
                if matches_sha1(&path, &actual) {
                    touch(&path);
                } else {
                    let _ = fs::remove_file(&path);
                    fs::rename(&tmp, &path)?;
                }
                Ok(path)
            });
            let _ = fs::remove_file(&tmp);
            result.map_err(|e| e.to_string())
        }
    }
}

/// Cancellable HEAD hook on `horizon_client::server_packs::download::download_file`:
/// Retries failed or stalled CDN downloads up to 3 times and writes atomically via a `.tmp` file.
pub fn on_download_file(
    dir: &Path,
    request: &Request,
    headers: &[(&'static str, String)],
    max_size: u64,
    progress: &Sender<Progress>,
) -> ControlFlow<Result<PathBuf, String>> {
    let s = m::current_settings();
    if !s.enabled || !s.download_retries {
        return ControlFlow::Continue(());
    }
    if let Some(sha1) = &request.sha1 {
        let target = dir.join(sha1);
        if matches_sha1(&target, sha1) {
            log::info!("Horizon: returning cached file since actual hash matches requested");
            touch(&target);
            return ControlFlow::Break(Ok(target));
        }
    }
    let mut last_err = String::new();
    for attempt in 0..3 {
        match attempt_download(dir, request, headers, max_size, progress, attempt) {
            Ok(path) => return ControlFlow::Break(Ok(path)),
            Err(e) => {
                log::warn!(
                    "[Server Pack Fixer] Download attempt {}/3 for {} failed: {e}",
                    attempt + 1,
                    request.url
                );
                last_err = e;
                if attempt < 2 {
                    std::thread::sleep(Duration::from_millis(350 * (attempt as u64 + 1)));
                }
            }
        }
    }
    ControlFlow::Break(Err(last_err))
}

/// Cancellable HEAD hook on `horizon_client::server_packs::download::vacuum`:
/// Keeps at least `MIN_CACHED_SERVER_PACKS` (256) server pack files in `downloads/` instead of 20
/// so multi-pack servers like HugoSMP never evict their own active sub-packs on startup.
pub fn on_vacuum(dir: &Path, max_files: usize) -> ControlFlow<()> {
    let s = m::current_settings();
    if !s.enabled || !s.download_retries || max_files >= m::MIN_CACHED_SERVER_PACKS {
        return ControlFlow::Continue(());
    }
    let keep = m::MIN_CACHED_SERVER_PACKS;
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let path = e.path();
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() && path.parent() != Some(dir) {
                let time = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                files.push((path, time));
            }
        }
    }
    let Some(remove) = files.len().checked_sub(keep).filter(|n| *n > 0) else {
        return ControlFlow::Break(());
    };
    files.sort_by_key(|f| std::cmp::Reverse(f.1));
    let mut counts = std::collections::HashMap::<PathBuf, usize>::new();
    let mut ranked: Vec<(PathBuf, usize)> = files
        .into_iter()
        .map(|(path, _)| {
            let count = counts
                .entry(path.parent().unwrap_or(dir).to_path_buf())
                .or_default();
            let rank = *count;
            *count += 1;
            (path, rank)
        })
        .collect();
    ranked.reverse();
    ranked.sort_by_key(|f| std::cmp::Reverse(f.1));
    let mut emptied = std::collections::HashSet::new();
    for (path, rank) in ranked.into_iter().take(remove) {
        if fs::remove_file(&path).is_ok() && rank == 0 {
            emptied.insert(path.parent().unwrap_or(dir).to_path_buf());
        }
    }
    emptied.remove(dir);
    for d in emptied {
        let _ = fs::remove_dir(d);
    }
    ControlFlow::Break(())
}

