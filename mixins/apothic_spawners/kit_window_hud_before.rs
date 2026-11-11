fn jei_hud_before(game: &mut crate::game::Game<'_>) {
    apothic_spawners_optional::if_kit_jei!("apothic_spawners", {
    crate::game::hz_kit_window_apothic_spawners::hud_before(game);
} else { let _ = game; () })
}
