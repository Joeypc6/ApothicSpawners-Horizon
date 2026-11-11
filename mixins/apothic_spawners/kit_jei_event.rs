fn ae2_jei_event(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    event: &crate::game::native_hooks::NativePanelEvent,
) {
    apothic_spawners_optional::if_kit_jei!("apothic_spawners", {
    crate::game::hz_kit_layer_apothic_spawners::event(game, mod_id, event);
} else { let _ = (game, mod_id, event); () })
}
