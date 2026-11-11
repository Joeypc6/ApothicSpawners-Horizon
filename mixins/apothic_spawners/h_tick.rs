fn apothic_spawners_tick(game: &mut crate::game::Game<'_>, key: (i32, i32, i32), spawner: &mut horizon_client::mobs::spawner::Spawner, cart: Option<u32>) -> core::ops::ControlFlow<Result<()>> {
    crate::game::apothic_spawners::spawner_tick(game, key, spawner, cart)
}
