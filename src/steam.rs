use std::path::PathBuf;
use steamlocate::SteamDir;
use tracing::debug;

const DARKEST_DUNGEON_APP_ID: u32 = 262060;

pub fn get_steam_dir_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let steam_dir_path = SteamDir::locate()?.path().to_path_buf();
    debug!(
        "Steam install directory discovered at: {}",
        steam_dir_path.display()
    );
    Ok(steam_dir_path)
}

pub fn get_darkest_dungeon_install_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let steam_dir = SteamDir::locate()?;
    let (app, library) = steam_dir
        .find_app(DARKEST_DUNGEON_APP_ID)?
        .ok_or("Darkest Dungeon not found in Steam library")?;
    let install_path = library.path().join("steamapps").join("common").join(&app.install_dir);
    debug!(
        "Darkest Dungeon install path discovered at: {}",
        install_path.display()
    );
    Ok(install_path)
}

// #[cfg(test)]
// This file does not require testing since it is a simple wrapper around the steamlocate library.
// Refer to https://github.com/williamvenner/steamlocate-rs for library specific documentation and tests.
