fn jei_panel_tint(game: &Game<'_>, batch: &mut HudBatch) {
    apothic_spawners_optional::if_kit_jei!("apothic_spawners", {
    crate::game::hz_kit_window_apothic_spawners::tint(game, batch);
} else { let _ = (game, batch); () })
}
