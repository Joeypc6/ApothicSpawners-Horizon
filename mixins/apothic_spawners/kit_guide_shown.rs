fn kit_guide_shown_apothic_spawners(stack: &ItemStack) {
    apothic_spawners_optional::if_kit!("apothic_spawners", {
    crate::game::hz_kit_guide_apothic_spawners::hovered(&stack.item);
} else { let _ = stack; () })
}
