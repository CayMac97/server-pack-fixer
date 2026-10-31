// Server Pack & SMP Fixer runtime state and pure logic.
// Compiles both standalone (`cargo test`) and inside `horizon-client-render` / `horizon-client`.

use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

pub const ID: &str = "server_pack_fixer";
pub const NAME: &str = "Server Pack & SMP Fixer";
pub const VERSION: &str = "1.0.0";
pub const DESCRIPTION: &str = "Fixes multi-pack server downloads, 1.21 pack overlays, custom GUI buttons, PUA/negative-space fonts, leaderboard holograms, legacy custom_model_data items, and multiplayer chunk holes (HugoSMP & custom SMPs).";

pub const KEY_TOGGLE: &str = "toggle";
pub const KEY_TOGGLE_LABEL: &str = "Toggle Server Pack & SMP Fixer";
pub const KEY_TOGGLE_DEFAULT: &str = "key.keyboard.f9";

pub const BACKGROUND: u32 = 0x7800_0000;
pub const COLOR_ON: u32 = 0x0055_FF88;
pub const COLOR_OFF: u32 = 0x00AA_AAAA;

/// Minimum number of server pack files kept in `downloads/` so multi-pack servers never evict
/// their own sub-packs.
pub const MIN_CACHED_SERVER_PACKS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Settings {
    pub enabled: bool,
    pub download_retries: bool,
    pub pack_compat: bool,
    pub gui_buttons: bool,
    pub holograms: bool,
    pub chunk_fixer: bool,
    pub show_hud: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            download_retries: true,
            pack_compat: true,
            gui_buttons: true,
            holograms: true,
            chunk_fixer: true,
            show_hud: false,
        }
    }
}

impl Settings {
    pub fn load<'a>(mut get: impl FnMut(&str) -> Option<String> + 'a) -> Self {
        let mut s = Self::default();
        let parse_bool = |v: &str| v != "false" && v != "0";
        if let Some(v) = get("enabled") {
            s.enabled = parse_bool(&v);
        }
        if let Some(v) = get("download_retries") {
            s.download_retries = parse_bool(&v);
        }
        if let Some(v) = get("pack_compat") {
            s.pack_compat = parse_bool(&v);
        }
        if let Some(v) = get("gui_buttons") {
            s.gui_buttons = parse_bool(&v);
        }
        if let Some(v) = get("holograms") {
            s.holograms = parse_bool(&v);
        }
        if let Some(v) = get("chunk_fixer") {
            s.chunk_fixer = parse_bool(&v);
        }
        if let Some(v) = get("show_hud") {
            s.show_hud = v == "true" || v == "1";
        }
        s
    }

    pub fn stored(&self) -> [(&'static str, String); 7] {
        let b = |v: bool| if v { "true" } else { "false" }.to_owned();
        [
            ("enabled", b(self.enabled)),
            ("download_retries", b(self.download_retries)),
            ("pack_compat", b(self.pack_compat)),
            ("gui_buttons", b(self.gui_buttons)),
            ("holograms", b(self.holograms)),
            ("chunk_fixer", b(self.chunk_fixer)),
            ("show_hud", b(self.show_hud)),
        ]
    }

    pub fn labels(&self) -> Vec<String> {
        let on_off = |v: bool| if v { "ON" } else { "OFF" };
        vec![
            format!("Server Pack & SMP Fixer: {}", on_off(self.enabled)),
            format!(
                "Multi-Pack Download Retries & 256-Pack Cache: {}",
                on_off(self.download_retries)
            ),
            format!(
                "1.21 Pack Overlays & CustomModelData Bridge: {}",
                on_off(self.pack_compat)
            ),
            format!(
                "Custom GUI Button Stretch & PUA Label Fix: {}",
                on_off(self.gui_buttons)
            ),
            format!(
                "Leaderboard Hologram & World Text Fix: {}",
                on_off(self.holograms)
            ),
            format!(
                "Multiplayer Chunk Hole Fixer: {}",
                on_off(self.chunk_fixer)
            ),
            format!("Show Status HUD Badge: {}", on_off(self.show_hud)),
        ]
    }

    pub fn click(&mut self, button: usize) {
        match button {
            0 => self.enabled = !self.enabled,
            1 => self.download_retries = !self.download_retries,
            2 => self.pack_compat = !self.pack_compat,
            3 => self.gui_buttons = !self.gui_buttons,
            4 => self.holograms = !self.holograms,
            5 => self.chunk_fixer = !self.chunk_fixer,
            6 => self.show_hud = !self.show_hud,
            _ => {}
        }
    }
}

static STATE: Mutex<Option<Settings>> = Mutex::new(None);
static LAST_FONT_EPOCH: AtomicU64 = AtomicU64::new(u64::MAX);
static BRIDGED_ITEMS: AtomicU64 = AtomicU64::new(0);
static PARTIAL_CHUNKS_MESHED: AtomicU64 = AtomicU64::new(0);

pub fn settings<'a>(get: impl FnMut(&str) -> Option<String> + 'a) -> Settings {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(s) = *guard {
        return s;
    }
    let loaded = Settings::load(get);
    *guard = Some(loaded);
    loaded
}

pub fn current_settings() -> Settings {
    STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .unwrap_or_default()
}

pub fn set_settings(s: Settings) {
    let mut guard = STATE.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(s);
}

pub fn check_font_epoch_changed(epoch: u64) -> bool {
    let prev = LAST_FONT_EPOCH.swap(epoch, Ordering::Relaxed);
    prev != epoch
}

pub fn force_hologram_relayout() {
    LAST_FONT_EPOCH.store(u64::MAX, Ordering::Relaxed);
}

pub fn record_bridged_items(count: u64) {
    BRIDGED_ITEMS.store(count, Ordering::Relaxed);
}

pub fn add_partial_chunks_meshed(count: u64) {
    PARTIAL_CHUNKS_MESHED.fetch_add(count, Ordering::Relaxed);
}

pub fn stats() -> (u64, u64) {
    (
        BRIDGED_ITEMS.load(Ordering::Relaxed),
        PARTIAL_CHUNKS_MESHED.load(Ordering::Relaxed),
    )
}

/// Returns true if `c` is in a Unicode Private Use Area commonly used by server resource packs
/// (ItemsAdder, Oraxen, Nexo, HugoSMP) for custom GUI banners, ranks, and negative-space offsets.
pub fn is_custom_pack_char(c: char) -> bool {
    let code = c as u32;
    (0xE000..=0xF8FF).contains(&code)
        || (0xF0000..=0xFFFFD).contains(&code)
        || (0x100000..=0x10FFFD).contains(&code)
}

/// Returns true if `s` contains any custom Private Use Area character.
pub fn has_custom_pack_chars(s: &str) -> bool {
    s.chars().any(is_custom_pack_char)
}

/// Strips a leading UTF-8 BOM (`\u{FEFF}`) and removes trailing commas before `}` or `]` outside
/// of string literals so quirky server `pack.mcmeta` files parse cleanly.
pub fn sanitize_json_text(raw: &str) -> String {
    let trimmed = raw.strip_prefix('\u{feff}').unwrap_or(raw).trim();
    let chars: Vec<char> = trimmed.chars().collect();
    let mut out = String::with_capacity(trimmed.len());
    let mut in_string = false;
    let mut escape = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == ',' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                i += 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Extracts `(min_major, max_major)` from a `pack.mcmeta` `overlays.entries[]` JSON entry.
/// Supports `min_format`/`max_format` (int or `[major, minor]`) and `formats` (int, `[min, max]`,
/// or `{"min_inclusive": min, "max_inclusive": max}`).
pub fn overlay_format_bounds(
    entry_min: Option<i64>,
    entry_max: Option<i64>,
    formats_int: Option<i64>,
    formats_arr: Option<(i64, i64)>,
    formats_obj: Option<(i64, i64)>,
) -> Option<(i64, i64)> {
    if let (Some(lo), Some(hi)) = (entry_min, entry_max) {
        return Some((lo, hi));
    }
    if let Some(n) = formats_int {
        return Some((n, n));
    }
    if let Some((lo, hi)) = formats_arr {
        return Some((lo, hi));
    }
    if let Some((lo, hi)) = formats_obj {
        return Some((lo, hi));
    }
    None
}

/// Normalizes a resource location so bare paths like `"item/custom_sword"` become
/// `"minecraft:item/custom_sword"`.
pub fn normalize_resource_id(id: &str) -> String {
    if id.contains(':') {
        id.to_owned()
    } else {
        format!("minecraft:{id}")
    }
}

const CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0usize;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0usize;
        while k < 8 {
            if (c & 1) != 0 {
                c = 0xEDB8_8320 ^ (c >> 1);
            } else {
                c >>= 1;
            }
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

pub fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        let idx = ((crc ^ (b as u32)) & 0xFF) as usize;
        crc = CRC32_TABLE[idx] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFFu32
}

/// Checks and recalculates CRC32 checksums for every chunk in a PNG file.
/// Server resource packs (like HugoSMP / Oraxen / ItemsAdder) frequently use custom PNG obfuscators
/// that intentionally corrupt chunk CRCs. Vanilla Java Minecraft ignores chunk CRCs, but Rust's
/// `png` crate strictly enforces them unless corrected or ignored.
pub fn fix_png_chunk_crcs(bytes: &mut [u8]) -> bool {
    const PNG_SIG: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    if bytes.len() < 8 || &bytes[0..8] != &PNG_SIG {
        return false;
    }
    let mut modified = false;
    let mut pos = 8;
    while pos + 12 <= bytes.len() {
        let len = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]) as usize;
        if pos + 12 + len > bytes.len() {
            break;
        }
        let chunk_type_and_data = &bytes[pos + 4..pos + 8 + len];
        let calculated = crc32_ieee(chunk_type_and_data);
        let stored = u32::from_be_bytes([
            bytes[pos + 8 + len],
            bytes[pos + 9 + len],
            bytes[pos + 10 + len],
            bytes[pos + 11 + len],
        ]);
        if stored != calculated {
            let be = calculated.to_be_bytes();
            bytes[pos + 8 + len..pos + 12 + len].copy_from_slice(&be);
            modified = true;
        }
        pos += 12 + len;
    }
    modified
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_png_chunk_crcs() {
        // Minimal valid 1x1 PNG with deliberately bad CRC
        let mut png = vec![
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // signature
            0x00, 0x00, 0x00, 0x0D, // IHDR len 13
            b'I', b'H', b'D', b'R',
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, // corrupt CRC
            0x00, 0x00, 0x00, 0x00, // IEND len 0
            b'I', b'E', b'N', b'D',
            0x00, 0x00, 0x00, 0x00, // corrupt CRC
        ];
        assert!(fix_png_chunk_crcs(&mut png));
        // Second pass should detect nothing to change
        assert!(!fix_png_chunk_crcs(&mut png));
    }

    #[test]
    fn settings_roundtrip_and_clicks() {
        let mut s = Settings::default();
        assert!(s.enabled);
        assert!(s.download_retries);
        assert!(s.pack_compat);
        assert!(s.gui_buttons);
        assert!(s.holograms);
        assert!(s.chunk_fixer);
        assert!(!s.show_hud);

        s.click(0);
        s.click(6);
        let map: std::collections::HashMap<_, _> = s.stored().into_iter().collect();
        let loaded = Settings::load(|k| map.get(k).cloned());
        assert!(!loaded.enabled);
        assert!(loaded.show_hud);
        assert_eq!(loaded.labels().len(), 7);
    }

    #[test]
    fn custom_pack_pua_detection() {
        assert!(is_custom_pack_char('\u{E000}'));
        assert!(is_custom_pack_char('\u{F8FF}'));
        assert!(is_custom_pack_char('\u{F0001}'));
        assert!(!is_custom_pack_char('A'));
        assert!(has_custom_pack_chars("Options \u{E123}"));
        assert!(!has_custom_pack_chars("Options"));
    }

    #[test]
    fn sanitize_json_strips_bom_and_trailing_commas() {
        let quirky = "\u{feff}{\"pack\": {\"pack_format\": 34, \"description\": \"Hugo,\",},}";
        let clean = sanitize_json_text(quirky);
        assert_eq!(
            clean,
            "{\"pack\": {\"pack_format\": 34, \"description\": \"Hugo,\"}}"
        );
    }

    #[test]
    fn normalize_resource_id_adds_default_namespace() {
        assert_eq!(
            normalize_resource_id("item/ruby_sword"),
            "minecraft:item/ruby_sword"
        );
        assert_eq!(
            normalize_resource_id("hugosmp:item/crown"),
            "hugosmp:item/crown"
        );
    }
}
