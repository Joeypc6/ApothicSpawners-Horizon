fn kit_guide_render_post_apothic_spawners(game: &crate::game::Game<'_>, batch: &mut horizon_client::hud::HudBatch, screen: &crate::game::native_hooks::NativeScreenView<'_>) {
    apothic_spawners_optional::if_kit!("apothic_spawners", {
    if game.player.is_none() {
        return;
    }
    crate::game::hz_kit_guide_apothic_spawners::screen_frame(screen.id, screen.menu);
    crate::game::hz_kit_guide_apothic_spawners::draw(game, batch, screen.mouse);
} else { let _ = (game, batch, screen); () })
}
