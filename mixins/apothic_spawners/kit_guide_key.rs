fn kit_guide_key_apothic_spawners(game: &crate::game::Game<'_>, screen: &crate::game::native_hooks::NativeScreenView<'_>, key: &str, repeat: bool, canceled: &mut bool) {
    apothic_spawners_optional::if_kit!("apothic_spawners", {
    if game.player.is_none() {
        return;
    }
    crate::game::hz_kit_guide_apothic_spawners::key(game, screen.menu, key, repeat, canceled);
} else { let _ = (game, screen, key, repeat, canceled); () })
}
