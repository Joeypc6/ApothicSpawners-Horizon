fn apothic_spawners_stopping(dedicated: bool) {
    let _ = dedicated;
    crate::game::apothic_spawners::server_stopping();
}
