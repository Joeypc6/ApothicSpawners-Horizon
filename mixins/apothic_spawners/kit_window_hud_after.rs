fn jei_hud_after(game: &mut crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch) {
    apothic_spawners_optional::if_kit_jei!("apothic_spawners", {
    crate::game::hz_kit_window_apothic_spawners::hud_after(game, batch);
} else { let _ = (game, batch); () })
}
