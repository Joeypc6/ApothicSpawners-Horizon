fn kit_guide_pressed_apothic_spawners(game: &crate::game::Game<'_>, screen: &crate::game::native_hooks::NativeScreenView<'_>, button: i32, canceled: &mut bool) {
    apothic_spawners_optional::if_kit!("apothic_spawners", {
    if game.player.is_none() {
        return;
    }
    let _ = button;
    crate::game::hz_kit_guide_apothic_spawners::click(game, screen.mouse, canceled);
} else { let _ = (game, screen, button, canceled); () })
}
