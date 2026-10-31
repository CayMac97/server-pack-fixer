// Support module injected into `crates/horizon-client-render/src/lib.rs` as
// `pub mod mod_server_pack_fixer_render;`.
// Implements all rendering, resource pack overlay, custom GUI button, font glyph region,
// hologram line splitting, and world text depth fixes.

use crate::mod_server_pack_fixer as m;
use crate::{
    font::Font,
    hud::{HudAtlas, HudBatch, rgba},
    pack_screen::Compat,
    packs::{Filter, Pack, RESOURCE_FORMAT, Source},
    text::{Span, Style, Text, legacy_format},
    zip::Zip,
};
use core::ops::ControlFlow;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::Path,
};

fn format_of_val(v: &serde_json::Value) -> Option<(i64, i64)> {
    match v {
        serde_json::Value::Number(n) => Some((n.as_i64()?, 0)),
        serde_json::Value::Array(a) => Some((
            a.first()?.as_i64()?,
            a.get(1).and_then(|m| m.as_i64()).unwrap_or(0),
        )),
        _ => None,
    }
}

fn entry_bounds(entry: &serde_json::Value) -> Option<(i64, i64)> {
    let min_fmt = format_of_val(&entry["min_format"]).map(|f| f.0);
    let max_fmt = format_of_val(&entry["max_format"]).map(|f| f.0);
    let formats = &entry["formats"];
    let formats_int = formats.as_i64();
    let formats_arr = formats.as_array().and_then(|a| {
        Some((
            a.first()?.as_i64()?,
            a.get(1)?.as_i64()?,
        ))
    });
    let formats_obj = match (
        formats["min_inclusive"].as_i64(),
        formats["max_inclusive"].as_i64(),
    ) {
        (Some(lo), Some(hi)) => Some((lo, hi)),
        _ => None,
    };
    m::overlay_format_bounds(min_fmt, max_fmt, formats_int, formats_arr, formats_obj)
}

fn overlay_applies_vanilla(entry: &serde_json::Value) -> bool {
    let current = RESOURCE_FORMAT;
    if let (Some(min), Some(max)) = (
        format_of_val(&entry["min_format"]),
        format_of_val(&entry["max_format"]),
    ) {
        return min <= current && current <= (max.0, if max.1 == 0 { i64::MAX } else { max.1 });
    }
    let formats = &entry["formats"];
    if let Some(n) = formats.as_i64() {
        return n == current.0;
    }
    if let Some(a) = formats.as_array() {
        return match (
            a.first().and_then(|v| v.as_i64()),
            a.get(1).and_then(|v| v.as_i64()),
        ) {
            (Some(lo), Some(hi)) => lo <= current.0 && current.0 <= hi,
            _ => false,
        };
    }
    match (
        formats["min_inclusive"].as_i64(),
        formats["max_inclusive"].as_i64(),
    ) {
        (Some(lo), Some(hi)) => lo <= current.0 && current.0 <= hi,
        _ => false,
    }
}

/// Cancellable HEAD hook on `horizon_client_render::packs::overlays_of`:
/// When a 1.21+ server pack declares overlay directories with `max_format < 75` (e.g. `34..=64`),
/// Horizon's strict `RESOURCE_FORMAT = (75, 0)` check skips them. We include modern 1.21+ overlays
/// (`max >= 34`) whose `min <= 75`.
pub fn on_overlays_of_head(meta: &serde_json::Value) -> ControlFlow<Vec<String>> {
    let s = m::current_settings();
    if !s.enabled || !s.pack_compat {
        return ControlFlow::Continue(());
    }
    let Some(entries) = meta["overlays"]["entries"].as_array() else {
        return ControlFlow::Break(Vec::new());
    };
    let mut ret: Vec<String> = Vec::new();
    let mut candidates: Vec<(i64, String)> = Vec::new();
    for entry in entries {
        let Some(dir) = entry["directory"].as_str() else {
            continue;
        };
        if overlay_applies_vanilla(entry) {
            ret.push(dir.to_owned());
        } else if let Some((lo, hi)) = entry_bounds(entry)
            && lo <= RESOURCE_FORMAT.0
            && hi >= 34
        {
            candidates.push((hi, dir.to_owned()));
        }
    }
    if !candidates.is_empty() {
        let best_hi = candidates.iter().map(|(hi, _)| *hi).max().unwrap_or(0);
        for (hi, dir) in candidates {
            if (ret.is_empty() && hi >= best_hi.saturating_sub(15)) || hi == best_hi {
                if !ret.contains(&dir) {
                    ret.push(dir);
                }
            }
        }
    }
    ControlFlow::Break(ret)
}

fn read_tolerant_meta(path: &Path) -> Option<serde_json::Value> {
    let zip = Zip::open(path).ok()?;
    let mut raw_bytes = zip.read("pack.mcmeta").ok();
    if raw_bytes.is_none() {
        let mut names = zip.names();
        let mut found = None;
        while let Some(n) = names.next() {
            if n.ends_with("/pack.mcmeta") {
                found = Some(n.to_owned());
                break;
            }
        }
        if let Some(name) = found {
            raw_bytes = zip.read(&name).ok();
        }
    }
    if let Some(bytes) = raw_bytes {
        let text = String::from_utf8_lossy(&bytes);
        let cleaned = m::sanitize_json_text(&text);
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&cleaned) {
            return Some(val);
        }
    }
    Some(serde_json::json!({
        "pack": {
            "pack_format": RESOURCE_FORMAT.0,
            "description": "Server Resource Pack"
        }
    }))
}

/// Cancellable HEAD hook on `horizon_client_render::packs::server_pack`:
/// Recovers from BOM-prefixed, trailing-comma, or missing `pack.mcmeta` in server pack ZIPs, and
/// marks loaded server packs compatible so multi-pack servers never abort the pack stack.
pub fn on_server_pack_head(id: u128, path: &Path) -> ControlFlow<Option<Pack>> {
    let s = m::current_settings();
    if !s.enabled || !s.pack_compat {
        return ControlFlow::Continue(());
    }
    let Some(os_name) = path.file_name() else {
        return ControlFlow::Break(None);
    };
    let file = os_name.to_string_lossy();
    let Some(meta) = read_tolerant_meta(path) else {
        return ControlFlow::Continue(());
    };
    let (raw_description, description_style) =
        crate::packs::component_root(&meta["pack"]["description"]);
    let overlays = match on_overlays_of_head(&meta) {
        ControlFlow::Break(v) => v,
        ControlFlow::Continue(()) => Vec::new(),
    };
    let filter: Vec<Filter> = Vec::new();
    ControlFlow::Break(Some(Pack {
        id: format!("server/{id:032x}/{file}"),
        title: Text::translate("resourcePack.server.name", &[]),
        description: raw_description.resolve_legacy(),
        raw_description,
        description_style,
        compatible: true,
        compatibility: Compat::Compatible,
        overlays,
        filter,
        source: Source::Zip(path.to_path_buf()),
    }))
}

/// Collects legacy `assets/minecraft/models/item/<item>.json` files from `packs` that define
/// `"overrides"` with `"predicate": {"custom_model_data": N}` and synthesizes 1.21.4+
/// `assets/minecraft/items/<item>.json` `range_dispatch` definitions when no pack provides one.
pub fn on_build_overlay(
    vanilla: &Path,
    packs: &[Pack],
    out: &Path,
    ret: io::Result<()>,
) -> io::Result<()> {
    if ret.is_err() {
        return ret;
    }
    let s = m::current_settings();
    if !s.enabled || !s.pack_compat || packs.is_empty() {
        return ret;
    }

    let mut explicit_items: BTreeSet<String> = BTreeSet::new();
    let mut model_bytes: BTreeMap<String, Vec<u8>> = BTreeMap::new();

    for pack in packs {
        let mut roots = Vec::with_capacity(1 + pack.overlays.len());
        roots.push(String::new());
        for d in &pack.overlays {
            roots.push(format!("{d}/"));
        }
        match &pack.source {
            Source::Dir(d) => {
                for root in &roots {
                    let base = d.join(root.trim_end_matches('/'));
                    let items_dir = base.join("assets/minecraft/items");
                    if let Ok(entries) = fs::read_dir(&items_dir) {
                        for e in entries.flatten() {
                            let name = e.file_name().to_string_lossy().into_owned();
                            if let Some(stem) = name.strip_suffix(".json") {
                                explicit_items.insert(stem.to_owned());
                            }
                        }
                    }
                    let models_dir = base.join("assets/minecraft/models/item");
                    if let Ok(entries) = fs::read_dir(&models_dir) {
                        for e in entries.flatten() {
                            let path = e.path();
                            let name = e.file_name().to_string_lossy().into_owned();
                            if let Some(stem) = name.strip_suffix(".json")
                                && let Ok(bytes) = fs::read(&path)
                            {
                                model_bytes.insert(stem.to_owned(), bytes);
                            }
                        }
                    }
                }
            }
            Source::Zip(z) => {
                let Ok(zip) = Zip::open(z) else {
                    continue;
                };
                let mut names = Vec::new();
                for n in zip.names() {
                    names.push(n.to_owned());
                }
                for root in &roots {
                    let items_prefix = format!("{root}assets/minecraft/items/");
                    let models_prefix = format!("{root}assets/minecraft/models/item/");
                    for name in &names {
                        if let Some(rest) = name.strip_prefix(&items_prefix) {
                            if let Some(stem) = rest.strip_suffix(".json")
                                && !stem.contains('/')
                            {
                                explicit_items.insert(stem.to_owned());
                            }
                        } else if let Some(rest) = name.strip_prefix(&models_prefix)
                            && let Some(stem) = rest.strip_suffix(".json")
                            && !stem.contains('/')
                            && let Ok(bytes) = zip.read(name)
                        {
                            model_bytes.insert(stem.to_owned(), bytes);
                        }
                    }
                }
            }
        }
    }

    let mut synthesized: Vec<(String, Vec<u8>)> = Vec::new();
    for (stem, bytes) in model_bytes {
        if explicit_items.contains(&stem) {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        let cleaned = m::sanitize_json_text(&text);
        let Ok(val) = serde_json::from_str::<serde_json::Value>(&cleaned) else {
            continue;
        };
        let Some(overrides) = val["overrides"].as_array() else {
            continue;
        };
        let mut entries: Vec<(f64, String)> = Vec::new();
        for ov in overrides {
            let cmd = ov["predicate"]["custom_model_data"].as_f64();
            let model = ov["model"].as_str();
            if let (Some(threshold), Some(model_id)) = (cmd, model)
                && threshold.is_finite()
            {
                entries.push((threshold, m::normalize_resource_id(model_id)));
            }
        }
        if entries.is_empty() {
            continue;
        }
        entries.sort_by(|a, b| a.0.total_cmp(&b.0));

        let vanilla_item_path = vanilla
            .join("assets/minecraft/items")
            .join(format!("{stem}.json"));
        let fallback_node = match fs::read_to_string(&vanilla_item_path) {
            Ok(s) => match serde_json::from_str::<serde_json::Value>(&s) {
                Ok(v) => v.get("model").cloned().unwrap_or_else(|| {
                    serde_json::json!({
                        "type": "minecraft:model",
                        "model": format!("minecraft:item/{stem}")
                    })
                }),
                Err(_) => serde_json::json!({
                    "type": "minecraft:model",
                    "model": format!("minecraft:item/{stem}")
                }),
            },
            Err(_) => serde_json::json!({
                "type": "minecraft:model",
                "model": format!("minecraft:item/{stem}")
            }),
        };

        let mut dispatch_entries = Vec::with_capacity(entries.len());
        for (threshold, model_id) in entries {
            dispatch_entries.push(serde_json::json!({
                "threshold": threshold,
                "model": {
                    "type": "minecraft:model",
                    "model": model_id
                }
            }));
        }

        let item_def = serde_json::json!({
            "model": {
                "type": "minecraft:range_dispatch",
                "property": "minecraft:custom_model_data",
                "fallback": fallback_node,
                "entries": dispatch_entries
            }
        });
        if let Ok(json_bytes) = serde_json::to_vec(&item_def) {
            synthesized.push((stem, json_bytes));
        }
    }

    m::record_bridged_items(synthesized.len() as u64);
    if synthesized.is_empty() {
        return Ok(());
    }

    let out_mc = out.join("assets/minecraft");
    if is_reparse_point(&out_mc) {
        let _ = fs::remove_dir(&out_mc).or_else(|_| fs::remove_file(&out_mc));
        let _ = fs::create_dir_all(&out_mc);
        let vanilla_mc = vanilla.join("assets/minecraft");
        if let Ok(entries) = fs::read_dir(&vanilla_mc) {
            for e in entries.flatten() {
                let src = e.path();
                let dst = out_mc.join(e.file_name());
                if src.is_file() {
                    let _ = fs::hard_link(&src, &dst).or_else(|_| fs::copy(&src, &dst).map(|_| ()));
                } else if src.is_dir() {
                    let _ = copy_dir_hardlinked(&src, &dst);
                }
            }
        }
    }

    let out_items = out_mc.join("items");
    let vanilla_items = vanilla.join("assets/minecraft/items");
    if is_reparse_point(&out_items) {
        let _ = fs::remove_dir(&out_items).or_else(|_| fs::remove_file(&out_items));
        let _ = copy_dir_hardlinked(&vanilla_items, &out_items);
    } else if !out_items.exists() {
        let _ = copy_dir_hardlinked(&vanilla_items, &out_items);
    }

    for (stem, json_bytes) in synthesized {
        let dst = out_items.join(format!("{stem}.json"));
        let _ = fs::remove_file(&dst);
        let _ = fs::write(&dst, json_bytes);
    }
    Ok(())
}

fn is_reparse_point(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|meta| {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
                meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 || meta.is_symlink()
            }
            #[cfg(not(windows))]
            {
                meta.is_symlink()
            }
        })
        .unwrap_or(false)
}

fn copy_dir_hardlinked(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    let Ok(entries) = fs::read_dir(src) else {
        return Ok(());
    };
    for e in entries.flatten() {
        let s = e.path();
        let d = dst.join(e.file_name());
        if s.is_dir() {
            copy_dir_hardlinked(&s, &d)?;
        } else if s.is_file() {
            let _ = fs::hard_link(&s, &d).or_else(|_| fs::copy(&s, &d).map(|_| ()));
        }
    }
    Ok(())
}

/// Cancellable HEAD hook on `horizon_client_render::ui::nine_slice`:
/// Stretches custom GUI button sprites that omit `.png.mcmeta` instead of tiling them.
pub fn on_nine_slice(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    sprite: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: u32,
    has_meta: bool,
    meta_loaded: bool,
) -> ControlFlow<()> {
    let s = m::current_settings();
    if !s.enabled || !s.gui_buttons {
        return ControlFlow::Continue(());
    }
    if meta_loaded && !has_meta {
        if let Some(r) = atlas.sprites.get(sprite) {
            batch.rect_uv(x, y, w, h, *r, color);
        }
        return ControlFlow::Break(());
    }
    ControlFlow::Continue(())
}

/// Cancellable HEAD hook on `horizon_client_render::ui::centered_text`:
/// Never character-clips button labels that contain custom PUA banner glyphs or negative-space
/// offsets.
pub fn on_centered_text(
    batch: &mut HudBatch,
    atlas: &HudAtlas,
    text: &Text,
    x: f32,
    y: f32,
    w: f32,
    color: u32,
) -> ControlFlow<()> {
    let s = m::current_settings();
    if !s.enabled || !s.gui_buttons {
        return ControlFlow::Continue(());
    }
    let resolved = text.resolved();
    let mut has_custom = false;
    for (c, st) in resolved.chars() {
        if m::is_custom_pack_char(c) || atlas.font.advance(c, st) < 0.0 {
            has_custom = true;
            break;
        }
    }
    if !has_custom {
        return ControlFlow::Continue(());
    }
    let width = atlas.font.width_text(&resolved);
    let draw_x = x + ((w - width) / 2.0).floor();
    batch.text_styled(atlas, &resolved, draw_x, y, color, true);
    ControlFlow::Break(())
}

/// Cancellable HEAD hook on `horizon_client_render::font::Region::place`:
/// Wraps the dynamic font glyph region cursor back to `(0, 0)` when full so subsequent custom
/// glyphs continue rendering instead of becoming permanent missing boxes.
pub fn on_region_place(
    cursor: &mut (u32, u32),
    row: &mut u32,
    w: u32,
    h: u32,
    atlas: (u32, u32),
    region: (u32, u32),
) -> ControlFlow<Option<(u32, u32)>> {
    let s = m::current_settings();
    if !s.enabled || !s.gui_buttons {
        return ControlFlow::Continue(());
    }
    let (x, mut y) = *cursor;
    if x + w + 1 > atlas.0 {
        y += *row + 1;
    }
    if y + h > region.1 && h <= region.1 && w + 1 <= atlas.0 {
        *cursor = (0, 0);
        *row = 0;
    }
    ControlFlow::Continue(())
}

/// Cancellable HEAD hook on `horizon_client_render::font::Font::split_formatted`:
/// Resolves `Translate` spans and prevents mid-line width wrapping on PUA / negative-space
/// hologram banners.
pub fn on_split_formatted(
    font: &Font,
    t: &Text,
    max_width: f32,
) -> ControlFlow<Vec<Text>> {
    let s = m::current_settings();
    if !s.enabled || !s.holograms {
        return ControlFlow::Continue(());
    }
    let resolved = t.resolved();
    let mut has_custom_or_neg = false;
    for (c, st) in resolved.chars() {
        if m::is_custom_pack_char(c) || font.advance(c, st) < 0.0 {
            has_custom_or_neg = true;
            break;
        }
    }
    let mut has_empty_span = false;
    for sp in &t.spans {
        if sp.text.is_empty() {
            has_empty_span = true;
            break;
        }
    }
    let has_translate = has_empty_span && !resolved.spans.is_empty();
    if !has_custom_or_neg && !has_translate {
        return ControlFlow::Continue(());
    }

    let effective_width = if has_custom_or_neg {
        max_width.max(16_384.0)
    } else {
        max_width.max(1.0)
    };

    let mut parts: Vec<(Vec<char>, Style)> = Vec::new();
    let mut flat: Vec<char> = Vec::new();
    for sp in &resolved.spans {
        if !sp.text.is_empty() {
            let chars: Vec<char> = sp.text.chars().collect();
            flat.extend_from_slice(&chars);
            parts.push((chars, sp.style.clone()));
        }
    }
    let mut out = Vec::new();
    let mut force_new_line = false;
    while let Some((at, split_style)) = line_break_resolved(font, &parts, effective_width) {
        let tail = flat[at];
        let new_line = tail == '\n';
        let skip = usize::from(new_line || tail == ' ');
        force_new_line = new_line;
        out.push(split_at_parts(&mut parts, at, skip, &split_style));
        flat.drain(..at + skip);
    }
    if !parts.is_empty() {
        let mut rest = Text::default();
        for (text, style) in parts {
            rest.spans.push(Span {
                text: text.into_iter().collect(),
                style,
            });
        }
        out.push(rest);
    } else if force_new_line {
        out.push(Text::empty());
    }
    if out.is_empty() {
        out.push(Text::empty());
    }
    ControlFlow::Break(out)
}

fn line_break_resolved(
    font: &Font,
    parts: &[(Vec<char>, Style)],
    max_width: f32,
) -> Option<(usize, Style)> {
    let (mut width, mut had_visible, mut last_space) = (0.0f32, false, None);
    let mut offset = 0;
    for (contents, part_style) in parts {
        let mut style = part_style.clone();
        let mut i = 0;
        while i < contents.len() {
            let c = contents[i];
            if c == '§' {
                let Some(&code) = contents.get(i + 1) else {
                    break;
                };
                style = legacy_format(&style, code, &Style::default());
                i += 2;
                continue;
            }
            let at = offset + i;
            if c == '\n' {
                return Some((at, style));
            }
            if c == ' ' {
                last_space = Some((at, style.clone()));
            }
            let advance = font.advance(c, &style);
            width += advance;
            if had_visible && width > max_width {
                return Some(last_space.unwrap_or((at, style)));
            }
            had_visible |= advance != 0.0;
            i += 1;
        }
        offset += contents.len();
    }
    None
}

fn split_at_parts(
    parts: &mut Vec<(Vec<char>, Style)>,
    at: usize,
    skip: usize,
    style: &Style,
) -> Text {
    let mut line = Text::default();
    let (mut position, mut in_skip) = (at, false);
    while let Some((text, part_style)) = parts.first() {
        let size = text.len();
        if !in_skip {
            if position > size {
                line.spans.push(Span {
                    text: text.iter().collect(),
                    style: part_style.clone(),
                });
                parts.remove(0);
                position -= size;
                continue;
            }
            if position > 0 {
                line.spans.push(Span {
                    text: text[..position].iter().collect(),
                    style: part_style.clone(),
                });
            }
            position += skip;
            in_skip = true;
        }
        if position <= size {
            if position == size {
                parts.remove(0);
            } else {
                parts[0] = (text[position..].to_vec(), style.clone());
            }
            break;
        }
        parts.remove(0);
        position -= size;
    }
    line
}

/// Helper called by `HudBatch::text_styled_world` HEAD hook:
/// Adds a tiny monotonic depth step per quad in 3D world text mode so negative-space overlaid
/// text never Z-fights with custom PUA background banner glyphs on leaderboard holograms.
pub fn on_text_styled_world_after(batch: &mut HudBatch, start_len: usize) {
    let s = m::current_settings();
    if !s.enabled || !s.holograms {
        return;
    }
    let _ = rgba;
    let slice = if start_len <= batch.vertices.len() {
        &mut batch.vertices[start_len..]
    } else {
        &mut batch.vertices[..]
    };
    for (quad_idx, quad) in slice.chunks_exact_mut(6).enumerate() {
        let offset = (quad_idx as f32) * 0.0001;
        for v in quad {
            if v.depth >= 0.0 {
                v.depth += offset;
            }
        }
    }
}
