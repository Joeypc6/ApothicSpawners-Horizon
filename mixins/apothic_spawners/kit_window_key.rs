fn jei_window_key(game: &crate::game::Game<'_>, screen: &crate::game::native_hooks::NativeScreenView<'_>, key: &str, canceled: &mut bool) {
    apothic_spawners_optional::if_kit_jei!("apothic_spawners", {
    if game.player.is_none() || *canceled {
        return;
    }
    crate::game::hz_kit_window_apothic_spawners::key(game, screen.mouse, key, canceled);
    if !*canceled {
        crate::game::hz_kit_creative_apothic_spawners::key(game, screen.mouse, key, canceled);
    }
} else { let _ = (game, screen, key, canceled); () })
}
