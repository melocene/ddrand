// Prevent a console window opening when running the application.
// Ref: https://github.com/slint-ui/slint/issues/3235
#![windows_subsystem = "windows"]

slint::include_modules!();

use chrono::Datelike;
use clap::Parser;
use log::*;
use rand::{distributions::Alphanumeric, rngs::StdRng, thread_rng, Rng};
use rand_pcg::Pcg64;
use rand_seeder::Seeder;
use rfd::FileDialog;
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

use crate::helpers::GamePath;

mod cli;
mod helpers;
mod logger;
mod rand_hero;
mod rand_mash;

fn main() -> Result<(), slint::PlatformError> {
    let app_window = AppWindow::new().unwrap();
    let opts = cli::Opts::parse();

    if opts.version {
        println!("{} v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    // Window title should match the binary's name and include the version information.
    let window_title = format!("{} v{}", env!("CARGO_BIN_NAME"), env!("CARGO_PKG_VERSION"));
    app_window.set_app_window_title(window_title.into());
    app_window.set_status_text("Application started.".into());

    // Setup the logger to use the application name and current date.
    // Log lines will append to the same days logs to avoid cluttering up the directory with small log files.
    let log_date = chrono::Local::now().format("%Y%m%d").to_string();
    let log_filename = format!("{}_{}.log", env!("CARGO_BIN_NAME"), log_date);
    // If compiled in `debug` mode or if provided the default flag print debug information to the log file.
    let _ = logger::init(
        if opts.debug || cfg!(debug_assertions) {
            "debug"
        } else {
            "info"
        },
        Some(&log_filename),
    );
    debug!("Debug mode enabled.");

    // Attempt to load the seed from the Windows registry otherwise return a default.
    // Clicking the `...` will allow the user to choose some other directory if automatic detection
    // fails or is incorrect.
    // Do not allow user to directly input strings for safety.
    let install_path = match helpers::get_install_path() {
        Ok(path_result) => {
            info!("Autodetected installation path: \'{}\'", &path_result);
            path_result
        }
        Err(e) => {
            error!("Autodetection failed for game installation.\nReason: {}", e);
            String::new()
        }
    };
    // Early exit if the game install directory does not exist or is not found.
    let tmp_ipath = Path::new(&install_path);
    if !tmp_ipath.exists() || !tmp_ipath.is_dir() {
        error!("Installation path does not exist or is not a directory. Exiting...");
        std::process::exit(1);
    }

    // Return a dictionarty of paths for the various game and mod data directories.
    // This shouldn't fail but if it does exit early since these paths are required.
    let gpaths: GamePath = match helpers::get_data_dirs(&install_path) {
        Ok(paths) => {
            debug!("{:#?}", paths);
            info!(
                "Mod directory will be:\'{}\'",
                paths.mod_dir.display().to_string()
            );
            app_window.set_mod_dir(paths.mod_dir.display().to_string().into());
            // If the mod directory exist, assume the mod is installed and enable disable button.
            if paths.mod_dir.exists() && paths.mod_dir.is_dir() {
                app_window.set_is_mod_installed(true);
            }
            paths
        }
        Err(e) => {
            error!(
                "Unable to obtain assemble required game and mod paths\nReason: {}",
                e
            );
            std::process::exit(1);
        }
    };

    let ui_handle = app_window.as_weak();
    ui_handle.unwrap().set_game_dir(install_path.clone().into());
    // If the user cancels the open file dialog a `None` result is returned which is invalid.
    // In these cases rather than crash store and reuse the last selected directory.
    // When a new valid directory is selected the mod directory will be assembled from it.
    let mut previous_game_dir = install_path;
    let ui_handle = app_window.as_weak();
    app_window.on_select_dir(move || {
        let game_dir = match FileDialog::new().pick_folder() {
            Some(selected_dir) => {
                previous_game_dir = selected_dir.display().to_string();
                selected_dir
            }
            None => PathBuf::from(previous_game_dir.clone()),
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
    let game_paths = gpaths.clone();
    app_window.on_enable_clicked(move || {
        let handle = ui_handle.unwrap();
        handle.set_status_text("Starting randomization, please wait.".into());
        enable_mod(&handle, &game_paths);
        handle.set_is_mod_installed(true);
        handle.set_status_text("ddrand mod installed successfully.".into());
    });

    let ui_handle = app_window.as_weak();
    let game_paths = gpaths.clone();
    app_window.on_enable_clicked_confirmed(move || {
        let handle = ui_handle.unwrap();
        handle.set_status_text("Starting randomization, please wait.".into());
        enable_mod(&handle, &game_paths);
        handle.set_is_mod_installed(true);
        handle.set_status_text("ddrand mod installed successfully.".into());
    });
    let ui_handle = app_window.as_weak();
    app_window.on_disable_clicked_confirmed(move || {
        let handle = ui_handle.unwrap();
        handle.set_status_text("Starting uninstallation, please wait.".into());
        helpers::uninstall_mod(Path::new(&ui_handle.unwrap().get_mod_dir().to_string()));
        handle.set_is_mod_installed(false);
        handle.set_status_text("ddrand mod uninstalled successfully.".into());
    });

    app_window.run()
}

/// Callback to generate a 32 character string to use as an input seed for the random number generator.
fn generate_clicked() -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .map(char::from)
        .take(32)
        .collect::<String>()
}

/// Callback to generate a 32 character string based on the year and week of the year.
fn weekly_clicked() -> String {
    let current_date = chrono::Local::now().date_naive();
    let current_week = current_date.iso_week().week0();
    debug!("Current week: {}", current_week);
    let week_seed = format!("{}{}seedoftheweek", current_date.year(), current_week);
    debug!("Weekly base seed: {}", &week_seed);
    let week_rng: Pcg64 = Seeder::from(week_seed).make_rng();
    let wseed = week_rng
        .sample_iter(&Alphanumeric)
        .map(char::from)
        .take(32)
        .collect::<String>();
    debug!("Seed of week {}: {}", current_week, wseed);

    wseed
}

fn enable_mod(handle: &AppWindow, gpaths: &GamePath) {
    let game_dir = gpaths.base.display().to_string();
    let mod_dir = gpaths.mod_dir.display().to_string();
    let mode_localization_dir = gpaths.mod_localization.display().to_string();

    if handle.get_is_mod_installed() {
        helpers::uninstall_mod(&gpaths.mod_dir);
    }

    // Attempt to write the seed to a file in the rand_hero mod directory.
    // If this fails just warn and continue as it is not required and is already displayed in the GUI.
    let seed_val = handle.get_seed_value().to_string();
    info!("Using seed: {}", &seed_val);
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

    // create the new StdRng from the provided seed value
    let seed_rng: StdRng = Seeder::from(&seed_val).make_rng();

    helpers::install_mod(&mod_dir, &mode_localization_dir);

    if handle.get_rand_combat_skills() {
        // Attempt to create the nessesary directory and exit if this fails as it is required.
        match fs::create_dir_all(&gpaths.mod_heroes) {
            Ok(_) => debug!("Created directory: {}", &gpaths.mod_heroes.display()),
            Err(e) => {
                error!(
                    "Could not create directory: {}\nReason: {}",
                    &gpaths.mod_dungeon.display(),
                    e
                );
                std::process::exit(1);
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
                            std::process::exit(1);
                        } else {
                            info!(
                                "{} written to \'{}\'",
                                &localization_filename,
                                &localization_xml_path.display()
                            );
                        }
                    }
                    Err(e) => {
                        // exit if the localization xml cannot be rendered as it is required by the game
                        error!("Unable to render localization data\nReason: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            Err(e) => {
                error!(
                    "Unable to read default string table to build localization\nReason: {}",
                    e
                );
                std::process::exit(1);
            }
        }
    }

    if handle.get_rand_boss() || handle.get_rand_monster() {
        // Attempt to create the nessesary directory and exit if this fails as it is required.
        match fs::create_dir_all(&gpaths.mod_dungeon) {
            Ok(_) => debug!("Created directory: {}", &gpaths.mod_dungeon.display()),
            Err(e) => {
                error!(
                    "Could not create directory: {}\nReason: {}",
                    &gpaths.mod_dungeon.display(),
                    e
                );
                std::process::exit(1);
            }
        }
        if let Ok(files) = rand_mash::get_data_files(&gpaths.base_dungeon, &None) {
            if let Ok(mashes) = rand_mash::extract_data(&files) {
                rand_mash::randomize(
                    &gpaths.mod_dungeon,
                    mashes,
                    seed_rng,
                    handle.get_rand_boss(),
                    handle.get_rand_monster(),
                );
            }
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
                        warn!("Unable to write mod audio data, audio for altered spawns may be missing");
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
                std::process::exit(1);
            } else {
                info!("project.xml written to '{}'", &project_xml_path.display());
            }
        }
        Err(e) => {
            // exit if the project.xml cannot be rendered as it is required by the game
            // without it the mod will fail to be recognized and loaded
            error!("Unable to render project data\nReason: {}", e);
            std::process::exit(1);
        }
    }

    // attempt to write the seed to a file in the rand_hero mod directory
    // if this fails just warn and continue as it is not required and is already logged to the console
    let seed_file_path = Path::join(&PathBuf::from(&mod_dir), "seed.txt");
    if let Err(e) = fs::File::create(&seed_file_path).and_then(|mut seed_file| {
        seed_file.write_all(handle.get_seed_value().to_string().as_bytes())
    }) {
        warn!(
            "Unable to write seed to file {}\n Reason: {}",
            &seed_file_path.to_str().unwrap(),
            e
        );
    } else {
        info!("Seed written to '{}'", &seed_file_path.display());
    }

    // Attempt to create directory structure for boss and spawn randomization.
    // These are required for this option so exit early if not successful.
    match fs::create_dir_all(&gpaths.mod_dungeon) {
        Ok(_) => debug!("Created directory: {}", &gpaths.mod_dungeon.display()),
        Err(e) => {
            error!(
                "Could not create directory: {}\nReason: {}",
                &gpaths.mod_dungeon.display(),
                e
            );
            std::process::exit(1);
        }
    }

    helpers::run_workshop_tool(&PathBuf::from(game_dir), &PathBuf::from(mod_dir));
}
