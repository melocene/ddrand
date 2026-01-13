// Prevent a console window opening when running the application.
// Ref: https://github.com/slint-ui/slint/issues/3235
#![windows_subsystem = "windows"]

slint::include_modules!();

use clap::Parser;
use rfd::FileDialog;
use std::collections::HashMap;
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};
use tracing::{debug, error, info, warn};
use tracing_subscriber::filter::LevelFilter;

use crate::helpers::GamePath;

mod cli;
mod helpers;
mod logger;
mod rand_hero;
mod rand_mash;
mod seed;
mod steam;

const DARKEST_DUNGEON_APP_ID: u32 = 262060;

// #[cfg(target_os = "windows")]
// const STEAM_BIN: &str = "steam.exe";

// #[cfg(not(target_os = "windows"))]
// const STEAM_BIN: &str = "steam";

fn main() -> Result<(), slint::PlatformError> {
    let bin_version = if cfg!(debug_assertions) {
        format!("{} DEVELOPMENT BUILD", env!("CARGO_BIN_NAME"))
    } else {
        format!("{} v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"))
    };
    
    let app_window = AppWindow::new().unwrap();
    let opts = cli::Opts::parse();

    if opts.version {
        println!("{}",bin_version);
        std::process::exit(0);
    }

    app_window.set_app_window_title(bin_version.into());
    app_window.set_status_text("Application started.".into());

    // Setup the logger to use the application name and current date.
    // Log lines will append to the same days logs to avoid cluttering up the directory with small log files.
    let log_date = chrono::Local::now().format("%Y%m%d").to_string();
    let log_filename = format!("{}_{}.log", env!("CARGO_BIN_NAME"), log_date);
    // If compiled in `debug` mode or if provided the default flag print debug information to the log file.
    // The guard must be kept alive to ensure logs are flushed on exit.
    let _log_guard = logger::init(
        if opts.debug || cfg!(debug_assertions) {
            LevelFilter::DEBUG
        } else {
            LevelFilter::INFO
        },
        Some(&log_filename),
    );
    debug!("Debug mode enabled.");

    // Clicking the `...` will allow the user to choose some other directory if automatic detection
    // fails or is incorrect.
    // Do not allow user to directly input strings for safety.
    let install_path = match steam::get_darkest_dungeon_install_path(DARKEST_DUNGEON_APP_ID) {
        Ok(path_result) => {
            // Canonicalize to normalize path format on Windows
            let normalized = dunce::canonicalize(&path_result).unwrap_or(path_result);
            info!("Autodetected game installation path: \'{}\'", &normalized.display());
            normalized
        }
        Err(e) => {
            warn!("Autodetection failed for game installation.\nReason: {}", e);
            PathBuf::new()
        }
    };
    if !install_path.exists() || !install_path.is_dir() {
        warn!(
            "Installation path does not exist or is not a directory. Please use '...' button to select installation directory."
        );
        app_window.set_game_dir("AUTODETECT_FAILED".into());
        app_window.set_status_text("Autodetection failed for game installation.".into());
    } else {
        app_window.set_game_dir(install_path.display().to_string().into());
    }

    // Return a dictionary of paths for the various game and mod data directories.
    // Used for initial UI setup; paths are regenerated when Enable is clicked.
    match helpers::get_data_dirs(&install_path) {
        Ok(paths) => {
            info!("Mod directory will be:\'{}\'", paths.mod_dir.display());
            app_window.set_mod_dir(paths.mod_dir.display().to_string().into());
            // If the mod directory exist, assume the mod is installed and enable disable button.
            if paths.mod_dir.exists() && paths.mod_dir.is_dir() {
                app_window.set_is_mod_installed(true);
            }
            paths
        }
        Err(e) => {
            warn!(
                "Unable to obtain assemble required game and mod paths\nReason: {}",
                e
            );
            GamePath {
                base: PathBuf::new(),
                base_dungeon: HashMap::new(),
                base_heroes: HashMap::new(),
                mod_dir: PathBuf::new(),
                mod_dungeon: PathBuf::new(),
                mod_localization: PathBuf::new(),
                mod_heroes: PathBuf::new(),
            }
        }
    };

    // If the user cancels the open file dialog a `None` result is returned which is invalid.
    // In these cases rather than crash store and reuse the last selected directory.
    // When a new valid directory is selected the mod directory will be assembled from it.
    let mut previous_game_dir = install_path.clone();
    let ui_handle = app_window.as_weak();
    app_window.on_select_dir(move || {
        let game_dir = match FileDialog::new().pick_folder() {
            Some(selected_dir) => {
                previous_game_dir = selected_dir.to_path_buf();
                selected_dir
            }
            None => previous_game_dir.clone(),
        };
        ui_handle
            .unwrap()
            .set_game_dir(game_dir.clone().as_os_str().to_str().unwrap().into());
        let new_mod_dir = Path::join(
            &PathBuf::from(game_dir.clone().as_os_str().to_str().unwrap()),
            "mods",
        )
        .join("ddrand")
        .display()
        .to_string();
        ui_handle.unwrap().set_mod_dir(new_mod_dir.into());
    });

    // Set initial placeholder seed value. When clicked the `Generate` button will replace the placeholder.
    // The user can still edit the seed if desired this just provides automatic generation if desired.
    // Weekly seed button will allow a consistent seed based on the week number.
    let ui_handle = app_window.as_weak();
    ui_handle.unwrap().set_seed_value(generate_clicked().into());

    let ui_handle = app_window.as_weak();
    app_window.on_generate_clicked(move || {
        let app_window = ui_handle.unwrap();
        app_window.set_seed_value(generate_clicked().into());
    });

    let ui_handle = app_window.as_weak();
    app_window.on_weekly_clicked(move || {
        let app_window = ui_handle.unwrap();
        app_window.set_seed_value(weekly_clicked().into());
    });

    let ui_handle = app_window.as_weak();
    app_window.on_enable_clicked({
        let ui_handle = ui_handle.clone();
        move || enable_handler(&ui_handle.unwrap())
    });

    let ui_handle = app_window.as_weak();
    app_window.on_enable_clicked_confirmed({
        let ui_handle = ui_handle.clone();
        move || enable_handler(&ui_handle.unwrap())
    });

    let ui_handle = app_window.as_weak();
    app_window.on_disable_clicked_confirmed(move || {
        let handle = ui_handle.unwrap();
        handle.set_status_text("Starting uninstallation, please wait.".into());
        match helpers::uninstall_mod(Path::new(&handle.get_mod_dir().to_string())) {
            Ok(_) => {
                handle.set_is_mod_installed(false);
                handle.set_status_text("ddrand mod uninstalled successfully.".into());
            }
            Err(e) => {
                handle.set_status_text(format!("Error: {}", e).into());
            }
        }
    });

    let ui_handle = app_window.as_weak();
    app_window.on_launch_game(move || {
        let handle = ui_handle.unwrap();
        handle.set_status_text("Launching game via Steam, please wait...".into());
        info!("Launching game via Steam...");
        launch_game();
    });

    app_window.run()
}

fn launch_game() {
    // Use the Steam protocol to launch the game via Steam using the user's specified settings if any.
    // This avoids the nees to call the Steam client directly or add complex parsing logic to find the correct binary to run.
    let launch_cmd = format!("steam://rungameid/{}", DARKEST_DUNGEON_APP_ID);
    debug!("Launch command: {}", launch_cmd);
    open::that(launch_cmd).unwrap();
}

fn enable_handler(handle: &AppWindow) {
    let game_dir = handle.get_game_dir().to_string();
    match helpers::get_data_dirs(Path::new(&game_dir)) {
        Ok(game_paths) => {
            handle.set_status_text("Starting randomization, please wait.".into());
            let handle_weak = handle.as_weak();
            slint::Timer::single_shot(std::time::Duration::from_millis(50), move || {
                let handle = handle_weak.unwrap();
                enable_mod(&handle, &game_paths);
                handle.set_is_mod_installed(true);
                handle.set_status_text("ddrand mod installed successfully.".into());
            });
        }
        Err(e) => {
            warn!("Unable to assemble game paths: {}", e);
            handle.set_status_text("Error: Invalid game directory.".into());
        }
    }
}

/// Callback to generate a 32 character string to use as an input seed for the random number generator.
fn generate_clicked() -> String {
    let seed = seed::generate_seed();
    debug!("Generated seed: {}", &seed);
    seed
}

/// Callback to generate a 32 character string based on the year and week of the year.
fn weekly_clicked() -> String {
    let seed = seed::generate_weekly_seed();
    debug!("Generated weekly seed: {}", &seed);
    seed
}

fn enable_mod(handle: &AppWindow, gpaths: &GamePath) {
    let game_dir = gpaths.base.display().to_string();
    let mod_dir = gpaths.mod_dir.display().to_string();
    //let mode_localization_dir = gpaths.mod_localization.display().to_string();

    if handle.get_is_mod_installed()
        && let Err(e) = helpers::uninstall_mod(&gpaths.mod_dir) {
            handle.set_status_text(format!("Error: {}", e).into());
            return;
        }

    // Attempt to write the seed to a file in the rand_hero mod directory.
    // If this fails just warn and continue as it is not required and is already displayed in the GUI.
    let seed_val = handle.get_seed_value().to_string();
    info!("Using seed: {}", &seed_val);

    let seed_rng = seed::create_rng(&seed_val);

    helpers::install_mod(&gpaths.mod_dir, &gpaths.mod_localization);
    let seed_file_path = Path::join(&gpaths.mod_dir, "seed.txt");
    if let Err(e) = fs::File::create(&seed_file_path)
        .and_then(|mut seed_file| seed_file.write_all(seed_val.as_bytes()))
    {
        warn!(
            "Unable to write seed to file {}\n Reason: {}",
            &seed_file_path.to_str().unwrap(),
            e
        );
    } else {
        info!("Seed written to '{}'", &seed_file_path.display());
    }

    if handle.get_rand_combat_skills() {
        // Attempt to create the necessary directory and return if this fails as it is required.
        match fs::create_dir_all(&gpaths.mod_heroes) {
            Ok(_) => debug!("Created directory: {}", &gpaths.mod_heroes.display()),
            Err(e) => {
                error!(
                    "Could not create directory: {}\nReason: {}",
                    &gpaths.mod_heroes.display(),
                    e
                );
                handle.set_status_text(format!("Error: Could not create directory: {}", e).into());
                return;
            }
        }

        let files = rand_hero::get_data_files(&gpaths.base_heroes, &None).unwrap();
        let heroes = rand_hero::extract_data(&files);

        let localization_map = rand_hero::randomize(
            &gpaths.base_heroes,
            &gpaths.mod_heroes,
            heroes,
            seed_rng.clone(),
        );

        info!("Extracting localization data");
        match rand_hero::extract_localizations(&gpaths.base) {
            Ok(translation) => {
                info!("Rendering new localization XML");
                match rand_hero::render_localizations(translation, localization_map) {
                    Ok(rendered) => {
                        let localization_filename = "rand_hero_en.string_table.xml";
                        let localization_xml_path =
                            Path::join(&gpaths.mod_localization, Path::new(&localization_filename));
                        if let Err(e) = fs::File::create(&localization_xml_path).and_then(
                            |mut localization_xml| localization_xml.write_all(rendered.as_bytes()),
                        ) {
                            error!(
                                "Unable to write \'{}\'\n Reason: {}",
                                &localization_xml_path.to_str().unwrap(),
                                e
                            );
                            handle.set_status_text(format!("Error: Unable to write localization file: {}", e).into());
                            return;
                        } else {
                            info!(
                                "{} written to \'{}\'",
                                &localization_filename,
                                &localization_xml_path.display()
                            );
                        }
                    }
                    Err(e) => {
                        error!("Unable to render localization data\nReason: {}", e);
                        handle.set_status_text(format!("Error: Unable to render localization data: {}", e).into());
                        return;
                    }
                }
            }
            Err(e) => {
                error!(
                    "Unable to read default string table to build localization\nReason: {}",
                    e
                );
                handle.set_status_text(format!("Error: Unable to read localization data: {}", e).into());
                return;
            }
        }
    }

    if handle.get_rand_boss() || handle.get_rand_monster() {
        // Attempt to create the necessary directory and return if this fails as it is required.
        match fs::create_dir_all(&gpaths.mod_dungeon) {
            Ok(_) => debug!("Created directory: {}", &gpaths.mod_dungeon.display()),
            Err(e) => {
                error!(
                    "Could not create directory: {}\nReason: {}",
                    &gpaths.mod_dungeon.display(),
                    e
                );
                handle.set_status_text(format!("Error: Could not create directory: {}", e).into());
                return;
            }
        }
        if let Ok(files) = rand_mash::get_data_files(&gpaths.base_dungeon, &None)
            && let Ok(mashes) = rand_mash::extract_data(&files)
        {
            rand_mash::randomize(
                &gpaths.mod_dungeon,
                mashes,
                seed_rng,
                handle.get_rand_boss(),
                handle.get_rand_monster(),
            );
        }
    }

    // Define the paths for the output audio JSON data.
    let mod_audio_path = Path::join(&PathBuf::from(&mod_dir), "audio");
    let mod_audio_file = Path::join(
        Path::new(&mod_audio_path),
        "randomizer.raid.load_order.json",
    );

    // Creating the directory is required for additional operations on this file.
    // If it got created continue, otherwise just warn the user as this is not a fatal error for the mod.
    if fs::create_dir_all(mod_audio_path).is_ok() {
        if let Ok(json_out) = helpers::extract_audio_json(&gpaths.base) {
            let output = helpers::render_audio_json(json_out);
            match fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(mod_audio_file)
            {
                Ok(mut outfile) => {
                    if outfile.write_all(output.as_bytes()).is_ok() {
                        info!("Audio data successfully written for randomizer mod")
                    } else {
                        warn!(
                            "Unable to write mod audio data, audio for altered spawns may be missing"
                        );
                    }
                }
                Err(_) => {
                    warn!(
                        "Unable to create audio JSON file, audio for altered spawns may be missing"
                    );
                }
            }
        }
    } else {
        warn!("Unable to read audio JSON data, audio for altered spawns may be missing");
    }

    match helpers::render_project_xml(&PathBuf::from(&game_dir), &PathBuf::from(&mod_dir)) {
        Ok(rendered) => {
            let project_xml_path = Path::join(&PathBuf::from(&mod_dir), Path::new("project.xml"));
            if let Err(e) = fs::File::create(&project_xml_path)
                .and_then(|mut project_xml| project_xml.write_all(rendered.as_bytes()))
            {
                error!(
                    "Unable to write project.xml {}\n Reason: {}",
                    &project_xml_path.to_str().unwrap(),
                    e
                );
                handle.set_status_text(format!("Error: Unable to write project.xml: {}", e).into());
                return;
            } else {
                info!("project.xml written to '{}'", &project_xml_path.display());
            }
        }
        Err(e) => {
            error!("Unable to render project data\nReason: {}", e);
            handle.set_status_text(format!("Error: Unable to render project data: {}", e).into());
            return;
        }
    }

    if let Err(e) = helpers::run_workshop_tool(&PathBuf::from(game_dir), &PathBuf::from(mod_dir)) {
        handle.set_status_text(format!("Error: {}", e).into());
    }
}
