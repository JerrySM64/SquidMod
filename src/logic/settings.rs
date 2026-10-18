use crate::logic::config::{AppConfig, ConnectionMode, NetworkMode, save_config};

pub fn persist_settings(
    connection_mode: ConnectionMode,
    network_mode: NetworkMode,
    check_for_updates: bool,
) {
    save_config(&AppConfig {
        connection_mode,
        network_mode,
        check_for_updates,
    });
}
