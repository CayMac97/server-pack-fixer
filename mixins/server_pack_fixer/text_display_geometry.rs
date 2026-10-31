// Hook helper: Game::text_display_geometry (HEAD)
fn server_pack_fixer_text_display_geometry(game: &mut crate::game::Game<'_>) {
    if crate::game::mod_server_pack_fixer_glue::should_relayout_holograms(game) {
        for display in game.text_displays.values_mut() {
            display.lines = None;
        }
    }
}
