// Support module injected into `crates/horizon-client/src/game.rs` as
// `mod mod_server_pack_fixer_glue;`.
// Implements Game-level hooks for `apply_staged_pack`, `text_display_geometry`,
// `remote_chunk_in_range`, keybinds, HUD badge, and Options > Mods settings.

use crate::game::Game;
use crate::game::native_hooks::NativeMod;
use horizon_client::{extras::Corner, extras_hud::Corners, hud::HudBatch};
use horizon_client_render::mod_server_pack_fixer as m;

pub(super) fn sync_settings(game: &Game<'_>) -> m::Settings {
    let s = m::settings(|key| game.native_setting(m::ID, key));
    let mut effective = s;
    if !game.native_on(m::ID) {
        effective.enabled = false;
    }
    m::set_settings(effective);
    effective
}

/// HEAD hook on `Game::apply_staged_pack`:
/// When server resource packs finish loading and are applied, invalidate cached `display.lines` on
/// all active `TextDisplay` entities so leaderboard holograms re-measure and re-split with the
/// server pack's custom glyph widths and translations.
pub(super) fn on_apply_staged_pack_head(game: &mut Game<'_>) {
    let s = sync_settings(game);
    if s.enabled && s.holograms {
        m::force_hologram_relayout();
    }
}

/// Helper for HEAD hook on `Game::text_display_geometry`:
/// Returns true when `TextDisplay` entities should invalidate cached `display.lines` (after a
/// server pack reload or font epoch change).
pub(super) fn should_relayout_holograms(game: &mut Game<'_>) -> bool {
    let s = sync_settings(game);
    if !s.enabled || !s.holograms {
        return false;
    }
    let epoch = game.gui.atlas.font.epoch();
    m::check_font_epoch_changed(epoch)
}

/// HEAD hook on `Game::remote_chunk_in_range`:
/// Prevents dropping multiplayer chunk packets that arrive before `SetChunkCacheCenter` initializes
/// `r.chunk_center` from `(0, 0)`, or while `r.chunk_center` lags behind the player's actual chunk
/// during spawn / hub / RTP teleportation.
pub(super) fn on_remote_chunk_in_range_head(
    game: &Game<'_>,
    x: i32,
    z: i32,
    center: Option<((i32, i32), i32)>,
) -> Option<bool> {
    let s = m::current_settings();
    if !s.enabled || !s.chunk_fixer || !game.native_on(m::ID) {
        return None;
    }
    let Some((chunk_center, view_distance)) = center else {
        return Some(true);
    };
    let range = view_distance.max(16) + 8;
    if chunk_center == (0, 0) {
        if let Some(p) = game.player.as_ref() {
            let px = (p.position.x.floor() as i32) >> 4;
            let pz = (p.position.z.floor() as i32) >> 4;
            if (x - px).abs() <= range && (z - pz).abs() <= range {
                return Some(true);
            }
        } else {
            return Some(true);
        }
    }
    if let Some(p) = game.player.as_ref() {
        let px = (p.position.x.floor() as i32) >> 4;
        let pz = (p.position.z.floor() as i32) >> 4;
        if (x - px).abs() <= range && (z - pz).abs() <= range {
            return Some(true);
        }
    }
    let generous = view_distance.max(12) + 6;
    if (x - chunk_center.0).abs() <= generous && (z - chunk_center.1).abs() <= generous {
        return Some(true);
    }
    None
}

pub(super) fn on_key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if !pressed || !game.native_on(m::ID) || !game.native_playing() {
        return;
    }
    let bound = game.native_mod_key(m::ID, m::KEY_TOGGLE, m::KEY_TOGGLE_DEFAULT);
    if key != bound {
        return;
    }
    let mut s = {
        let g = &*game;
        m::settings(|k| g.native_setting(m::ID, k))
    };
    s.enabled = !s.enabled;
    m::set_settings(s);
    for (k, v) in s.stored() {
        game.native_set_setting(m::ID, k, &v);
    }
    let state_label = if s.enabled { "ON" } else { "OFF" };
    game.native_chat(&format!("[Server Pack & SMP Fixer] Toggled {state_label}"));
}

pub(super) fn on_hud(game: &mut Game<'_>, batch: &mut HudBatch, corners: &mut Corners) {
    let s = sync_settings(game);
    if !game.native_on(m::ID) || !s.show_hud || !game.native_corner_shown(Corner::TopLeft) {
        return;
    }
    let (bridged_items, meshed_chunks) = m::stats();
    let text = if s.enabled {
        format!("SMP Fixer: ON ({bridged_items} items, {meshed_chunks} chunks)")
    } else {
        "SMP Fixer: OFF".to_owned()
    };
    let color = if s.enabled { m::COLOR_ON } else { m::COLOR_OFF };
    let w = game.native_text_width(&text) + 8.0;
    let h = 14.0;
    let (x, y) = corners.place(Corner::TopLeft, w, h);
    game.native_fill(batch, [x, y, w, h], m::BACKGROUND);
    game.native_text(batch, &text, [x + 4.0, y + 3.0], color, true);
}

pub(super) fn on_list(mods: &mut Vec<NativeMod>) {
    mods.push(NativeMod {
        id: m::ID.into(),
        title: format!("{} {}", m::NAME, m::VERSION),
        description: m::DESCRIPTION.into(),
        keys: vec![[
            m::KEY_TOGGLE.into(),
            m::KEY_TOGGLE_LABEL.into(),
            m::KEY_TOGGLE_DEFAULT.into(),
        ]],
    });
}

pub(super) fn on_settings(game: &mut Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id != m::ID {
        return;
    }
    let s = sync_settings(game);
    buttons.extend(s.labels());
}

pub(super) fn on_click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != m::ID {
        return;
    }
    let mut s = {
        let g = &*game;
        m::settings(|key| g.native_setting(m::ID, key))
    };
    s.click(button);
    m::set_settings(s);
    for (key, value) in s.stored() {
        game.native_set_setting(m::ID, key, &value);
    }
}
