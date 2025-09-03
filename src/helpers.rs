use log::*;
use regex::Regex;
use remove_dir_all::*;
use std::collections::HashMap;
use std::error::Error;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{exit, Command, Stdio},
    thread,
};

/// Collection of paths for the base game and randomizer mod
#[derive(Debug, Clone)]
pub struct GamePath {
    pub base: PathBuf,
    pub base_dungeon: HashMap<String, PathBuf>,
    pub base_heroes: HashMap<String, PathBuf>,
    pub mod_dir: PathBuf,
    pub mod_dungeon: PathBuf,
    pub mod_localization: PathBuf,
    pub mod_heroes: PathBuf,
}

/// Attempt automated discovery of the game installation path based on OS
pub fn get_install_path() -> Result<String, Box<dyn std::error::Error>> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let install_path = match hklm
        .open_subkey(r#"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\Steam App 262060"#)
    {
        Ok(key_path) => key_path.get_value("InstallLocation")?,
        Err(_) => {
            // attempt to use the default installation location if unable to read from the registry
            // if the path doesn't exist or is not a proper directory return an empty string for later filtering
            let default_dir_str = r#"C:\Program Files (x86)\Steam\steamapps\common\DarkestDungeon"#;
            let default_dir_path = Path::new(&default_dir_str);
            if Path::exists(default_dir_path) && Path::is_dir(default_dir_path) {
                String::from(default_dir_str)
            } else {
                String::new()
            }
        }
    };

    Ok(install_path)
}

/// Get all hero directories from the install path
pub fn get_data_dirs(install_dir: &String) -> Result<GamePath, Box<dyn Error>> {
    let valid_dungeon_names: &[String] = &[
        String::from("cove"),
        String::from("crypts"),
        String::from("warrens"),
        String::from("weald"),
    ];
    // this is a mess but assemble all the paths where the base hero data can be found
    let dungeon_subdir: PathBuf = Path::join(Path::new(&install_dir), Path::new("dungeons"));
    let hero_subdir: PathBuf = Path::join(Path::new(&install_dir), Path::new("heroes"));
    let dlc_subdir: PathBuf = Path::join(Path::new(&install_dir), Path::new("dlc"));

    let mut dmap: HashMap<String, PathBuf> = HashMap::new();
    match fs::read_dir(dungeon_subdir) {
        Ok(sd) => {
            for item in sd {
                match item {
                    Ok(dir) => {
                        let dir_path = dir.path();
                        if dir_path.exists() && dir_path.is_dir() {
                            let dir_name = dir.file_name().to_str().unwrap().to_string();
                            if valid_dungeon_names.contains(&dir_name) {
                                dmap.insert(dir_name, dir_path);
                            }
                        }
                    }
                    // TODO: add addition error details
                    Err(e) => {
                        error!("{}", e);
                    }
                }
            }
        }
        // TODO: add addition error details
        Err(e) => {
            error!("{}", e);
        }
    }

    // use the hero name as a key for its associated data directory
    // using this the skill icons can be properly copied around later
    let mut hmap: HashMap<String, PathBuf> = HashMap::new();
    for sd in &[&hero_subdir, &dlc_subdir] {
        match fs::read_dir(sd) {
            Ok(dir_read) => {
                for item in dir_read {
                    match item {
                        Ok(dir) => {
                            let dir_path = &dir.path();
                            if dir_path.exists() && dir_path.is_dir() {
                                let dir_name = dir.file_name().to_str().unwrap().to_string();
                                debug!("Checking {} for hero data", &dir_path.to_string_lossy());
                                // DLC heroes have odd paths so sort them out separately if they are not excluded
                                if dir_name.contains("musketeer") {
                                    let msubdir: PathBuf =
                                        vec!["445700_musketeer", "heroes", "musketeer"]
                                            .into_iter()
                                            .collect();
                                    let msubdir_path = Path::join(&dlc_subdir, msubdir);
                                    hmap.insert(String::from("musketeer"), msubdir_path);
                                } else if dir_name.contains("crimson_court") {
                                    let fsubdir: PathBuf = vec![
                                        "580100_crimson_court",
                                        "features",
                                        "flagellant",
                                        "heroes",
                                        "flagellant",
                                    ]
                                    .into_iter()
                                    .collect();
                                    let fsubdir_path = Path::join(&dlc_subdir, &fsubdir);
                                    hmap.insert(String::from("flagellant"), fsubdir_path);
                                } else if dir_name.contains("shieldbreaker") {
                                    let ssubdir: PathBuf =
                                        vec!["702540_shieldbreaker", "heroes", "shieldbreaker"]
                                            .into_iter()
                                            .collect();
                                    let ssubdir_path = Path::join(&dlc_subdir, &ssubdir);
                                    hmap.insert(String::from("shieldbreaker"), ssubdir_path);
                                } else if dir_name.contains("arena") || dir_name.contains("madness")
                                {
                                    continue;
                                } else {
                                    hmap.insert(
                                        dir.file_name().to_str().unwrap().to_string(),
                                        dir.path(),
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            // TODO: add addition error details
                            error!("{}", e);
                        }
                    }
                }
            }
            Err(e) => {
                // TODO: add addition error details
                error!("{}", e);
            }
        }
    }

    // avoid Rust typing issues and use a temporary variable for the proper mod subdirectory
    let tmp_subdirs: PathBuf = vec!["mods", "ddrand"].into_iter().collect();
    let randomizer_path: PathBuf = Path::join(&PathBuf::from(&install_dir), tmp_subdirs);
    let mod_localization_path: PathBuf = Path::join(&randomizer_path, Path::new("localization"));
    let mod_heroes_paths: PathBuf = Path::join(&randomizer_path, Path::new("heroes"));
    let mod_dungeon_paths = Path::join(Path::new(&randomizer_path), Path::new("dungeons"));

    // new object holding all of the paths needed for the new mod files
    let game_paths: GamePath = GamePath {
        base: PathBuf::from(install_dir),
        base_dungeon: dmap,
        base_heroes: hmap,
        mod_dir: randomizer_path,
        mod_localization: mod_localization_path,
        mod_heroes: mod_heroes_paths,
        mod_dungeon: mod_dungeon_paths,
    };

    debug!("{:#?}", &game_paths);

    Ok(game_paths)
}

pub fn install_mod(mod_dir: &str, mod_locale_path: &str) {
    info!("Starting randomizer mod generation");
    info!("Creating randomizer mod directory structure");
    for dir in &[mod_dir, mod_locale_path] {
        match fs::create_dir_all(dir) {
            Ok(_) => debug!("Created directory: {}", &dir),
            Err(e) => error!("Could not create directory: {}\nReason: {}", &dir, e),
        }
    }
}

/// Uninstall existing randomizer mod
pub fn uninstall_mod(mod_dir: &Path) {
    // to avoid issues remove any previous version of the randomizer mod
    // if unsuccessful after three attempts inform the user and exit to allow for manual cleanup
    let mut attempt = 1;
    let retry_limit = 3;
    let retry_delay: u64 = 100;

    if mod_dir.exists() {
        while attempt <= retry_limit {
            info!(
                "Attempting uninstallation of existing randomizer mod (attempt {}/{})",
                attempt, retry_limit
            );
            attempt += 1;

            match remove_dir_all(mod_dir) {
                Ok(_) => {
                    info!("Previous randomizer mod successfully uninstalled");
                    break;
                }
                Err(e) => {
                    error!("Unable to uninstall existing randomizer\nReason {}", e);

                    if attempt == retry_limit {
                        error!(
                            "Unable to remove mod directory at \"{}\"\nPlease remove it manually and retry",
                            &mod_dir.display()
                        );

                        exit(1);
                    } else {
                        // pause to let any running operation finish and clean up before retrying
                        thread::sleep(std::time::Duration::from_millis(retry_delay));
                        continue;
                    }
                }
            }
        }
    }
}

/// Render mod project.xml template
pub fn render_project_xml(install_path: &Path, mod_path: &Path) -> Result<String, Box<dyn Error>> {
    let base_xml_path = Path::join(
        install_path,
        vec!["_windows", "sample_project.xml"]
            .into_iter()
            .collect::<PathBuf>(),
    );

    // automatically run steam_workshop_tool.exe to generate a sample project file
    // this saved embedding it and adding extra dependencies for a project.xml template
    if !Path::exists(&base_xml_path) {
        let bin_dir = Path::join(install_path, Path::new("_windows"));
        let bin_path = Path::join(&bin_dir, Path::new("steam_workshop_upload.exe"));

        info!("Running steam_workshop_upload.exe to generate sample_project.xml");
        debug!("_windows directory path: {}", &bin_dir.display());
        debug!("steam_workshop_upload.exe path: {}", &bin_path.display());

        match Command::new(bin_path)
            .current_dir(&bin_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .status()
        {
            Ok(_) => {
                info!("Successfully generated sample_project.xml");
            }
            Err(_) => {
                error!("Unable to generate required sample_project.xml automatically, exiting...");
                std::process::exit(1);
            }
        }
    }

    // read template content for later updating
    let base_xml_content = match fs::read_to_string(base_xml_path) {
        Ok(content) => content,
        Err(_) => {
            error!("Missing sample_project.xml, cannot generate mod project.xml file");
            exit(1);
        }
    };

    let mod_data_path = &format!(
        "<ModDataPath>{}</ModDataPath>",
        &mod_path.display().to_string()
    );

    // use the version number from the application version information for the mod version
    let version = env!("CARGO_PKG_VERSION").split('.').collect::<Vec<&str>>();
    let version_major = &format!("<VersionMajor>{}</VersionMajor>", version[0]);
    let version_minor = &format!("<VersionMinor>{}</VersionMinor>", version[1]);

    // map regex patterns to the desired text in order to update the sample XML to be mod specific
    let replacements: Vec<(Regex, &str)> = vec![
        (
            Regex::new(r"<ModDataPath>(.*)</ModDataPath>")?,
            mod_data_path,
        ),
        (Regex::new(r"<Title>(.*)</Title>")?, "<Title>ddrand</Title>"),
        (
            Regex::new(r"<UpdateDetails>(.*)</UpdateDetails>")?,
            "<UpdateDetails />",
        ),
        (
            Regex::new(r"<UploadMode>(.*)</UploadMode>")?,
            "<UploadMode>dont_submit</UploadMode>",
        ),
        (
            Regex::new(r"<VersionMajor>(.*)</VersionMajor>")?,
            version_major,
        ),
        (
            Regex::new(r"<VersionMinor>(.*)</VersionMinor>")?,
            version_minor,
        ),
        (Regex::new(r"<Tags>\n(?sm)(.*)</Tags>")?, "<Tags />"),
        (
            Regex::new(r"<ItemDescription>(?s)(.*)</ItemDescription>")?,
            "<ItemDescription />",
        ),
    ];

    let mut rendered = base_xml_content;

    // find and replace for the affected lines to output
    for (cap, repl) in replacements {
        rendered = cap.replace_all(&rendered, repl).to_string();
    }

    Ok(rendered)
}

/// Read the default game audio load order JSON
pub fn extract_audio_json(base_dir: &Path) -> Result<String, Box<dyn Error>> {
    let base_audio_path = Path::join(base_dir, "audio");
    let base_audio_file = Path::join(Path::new(&base_audio_path), "base.dungeon.load_order.json");

    // input file is tiny, no harm reading in the whole thing at once
    let content = fs::read_to_string(base_audio_file)?;

    Ok(content)
}

/// Render the audio load order JSON for the mod, removing excess items
pub fn render_audio_json(data: String) -> String {
    let mut rendered = String::new();
    for line in data.split_whitespace() {
        // remove items not relevant to the dungeons supported by this mod
        if !line.contains("props") && !line.contains("darkestdungeon") && !line.contains("town") {
            // strip the trailing comma from the weald line since it should be the last line in the output
            if line.contains("weald") {
                let end_pos = line.len() - 1;
                let sub = &line[..end_pos];
                rendered = format!("{}\n{}", rendered, sub);
            } else {
                rendered = format!("{}\n{}", rendered, line);
            }
        }
    }

    rendered
}

/// Convert and export localization strings to the proper game file
pub fn run_workshop_tool(install_path: &Path, mod_path: &Path) {
    let bin_dir = Path::join(install_path, Path::new("_windows"));
    let bin_path = Path::join(&bin_dir, Path::new("steam_workshop_upload.exe"));

    debug!("{}", &bin_path.display());

    if bin_path.exists() {
        match Command::new(bin_path)
            .arg("project.xml")
            .current_dir(mod_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .status()
        {
            Ok(_) => {
                // the default output filename, which is not configurable, conflicts with the base game
                // rename the mod localization output file to override only those values
                // TODO: support non-English languages
                let loc_path = Path::join(mod_path, Path::new("localization"));
                let from_path = Path::join(&loc_path, Path::new("0_english.loc2"));
                if from_path.exists() {
                    let to_path = Path::join(&loc_path, Path::new("randomizer_english.loc2"));
                    fs::rename(from_path, to_path).unwrap();
                } else {
                    debug!("No localization data, skipping rename of non-existant file")
                }

                // if modfiles.txt is present renaming the localization file causes the game to crash due to name mismatch
                // just remove it since it is only required if the mod is uploaded to the Steam workshop
                let modfiles_txt_path = Path::join(mod_path, Path::new("modfiles.txt"));
                if fs::remove_file(modfiles_txt_path).is_ok() {
                    debug!("Removed modfiles.txt");
                }

                info!("Successfully finalized mod data")
            }
            Err(e) => {
                error!("Unable to finalize mod data\nReason: {}", e);
                exit(1);
            }
        }
    } else {
        error!("steam_workshop_upload.exe is not present and is required, exiting...");
        exit(1);
    }
}
