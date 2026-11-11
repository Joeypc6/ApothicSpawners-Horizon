fn apothic_spawners_list(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::native_hooks::NativeMod {
        id: "apothic_spawners".into(),
        title: "Apothic Spawners 2.0.1".into(),
        description: "Spawners you can pick up with Silk Touch and change with items, as in Apotheosis.".into(),
        keys: Vec::new(),
    });
}
