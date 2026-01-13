use std::path::PathBuf;
use steamlocate::SteamDir;
use tracing::debug;

pub fn get_darkest_dungeon_install_path(app_id: u32) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let steam_dir = SteamDir::locate()?;
    let (app, library) = steam_dir
        .find_app(app_id)?
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
