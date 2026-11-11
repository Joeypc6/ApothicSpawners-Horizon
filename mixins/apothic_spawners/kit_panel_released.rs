fn jei_window_released(screen: &crate::game::native_hooks::NativeScreenView<'_>) {
    apothic_spawners_optional::if_kit_jei!("apothic_spawners", {
    crate::game::hz_kit_window_apothic_spawners::release(screen.mouse);
} else { let _ = screen; () })
}
