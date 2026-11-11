fn kit_guide_frame_apothic_spawners(game: &mut crate::game::Game<'_>) {
    apothic_spawners_optional::if_kit!("apothic_spawners", {
    if game.player.is_some() {
        crate::game::hz_kit_guide_apothic_spawners::frame_hook(game);
    }
} else { let _ = game; () })
}
