# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com),
and this project adheres to [Semantic Versioning](https://semver.org).

## [0.3.1] - Unreleased

### Fixed

- Manual editing of the seed value no longer disables the `Generate` and `Weekly` buttons
- Duplicate or empty log messages are no longer written to the logfile

## [0.3.0] - 2024-05-27

### Changed

- CLI converted to a GUI using the `Slint` framework
- [dev] Swap `time` for `chrono` for increased compatibility and ease of use

### Removed

- Temporarily removed boss randomization due to conflicts with monster spawn randomization
- Temporarily removed hero and monster exclusion options to fit the new GUI

## [0.2.1] - Unreleased

### Added

- Add seed of the week option which will generate one seed per week for competitive play

### Changed

- Only alphanumeric characters can be used as input seed values, any other symbols will result in an error
- [dev] Swap deprecated `structopt` for `clap`
- [dev] Drop `chrono` dependency and replace it with `time`

### Fixed

- Fix errors when the `sample_project.xml` file is missing by using the `steam_workshop_tool.exe` application shipped with the game to create it if needed
- Properly order conditional checks when determining if a previous mod installation should be removed

## [0.2.0] - Unreleased

### Added

- Add option to randomize bosses independently of normal monster spawns

### Changed

- Console log messages no longer are color coded for clarity on some terminals such as Powershell

## [0.1.1] - Unreleased

### Added

- Add option for randomizing combat skills only in advance of adding camping skill randomization

### Fixed

- Fix abomination by enabling and limiting skill selection to 4 skills
- Fix invalid warning about failed audio directory creation when not randomizing monster spawns

## [0.1.0] - Unreleased

### Added
- Base hero, including DLC, skill randomization
- Spawn randomization for Cove, Ruins, Warrens, and Weald
- Randomized skills get the correct names and icons within the game
- Ability to specify a seed used for reproducible randomization
- Seed value is written to `seed.txt` in the mod directory for later reference
- Option to uninstall the randomizer mod, with confirmation prompt
- Option to log all output to a file
- Option to exclude classes or dungeons from randomization
- Option to get current seed and output its value
