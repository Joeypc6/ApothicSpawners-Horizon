fn jei_window_pressed(game: &crate::game::Game<'_>, screen: &crate::game::native_hooks::NativeScreenView<'_>, button: i32, canceled: &mut bool) {
    apothic_spawners_optional::if_kit_jei!("apothic_spawners", {
    if game.player.is_none() || *canceled {
        return;
    }
    crate::game::hz_kit_window_apothic_spawners::press(game, screen.mouse, button, canceled);
    if !*canceled {
        crate::game::hz_kit_creative_apothic_spawners::press(game, screen.mouse, button, canceled);
    }
} else { let _ = (game, screen, button, canceled); () })
}
