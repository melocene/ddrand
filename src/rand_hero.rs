use indexmap::IndexMap;
use log::*;
use rand::{Rng, rngs::StdRng};
use regex::{Captures, Regex};
use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::BufReader;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

/// Translations for numeric positions to strings
const POS_STR: &[&str] = &["one", "two", "three", "four", "five", "six", "seven"];

/// Data read from the various hero class files
#[derive(Debug, Clone)]
pub struct Hero {
    name: String,
    data: Vec<String>,
    sknames: Vec<String>,
    skills: Vec<Skill>,
}

/// Information for each skills read in from various hero classes
#[derive(Debug, Clone, PartialEq)]
pub struct Skill {
    pos: usize,
    class: String,
    name: String,
    data: Vec<String>,
}

/// Object for matching old skill names to new, used for templating
#[derive(Debug, Serialize)]
pub struct SkillLocalization {
    class: String,
    map: HashMap<String, String>,
}

#[derive(Debug)]
pub struct Translation {
    lang: String,
    map: HashMap<String, String>,
}

/// Helper function to get all files to extract hero data from
pub fn get_data_files(
    hero_paths: &HashMap<String, PathBuf>,
    excludes: &Option<Vec<String>>,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut datafiles: Vec<PathBuf> = Vec::new();
    for hdir in hero_paths.values() {
        match fs::read_dir(hdir) {
            Ok(fread) => {
                for item in fread {
                    match item {
                        Ok(i) => {
                            if !i.path().is_dir() {
                                let filename: String =
                                    String::from(i.file_name().to_str().unwrap());
                                if let Some(excludes) = &excludes {
                                    let fname = filename.split('.').collect::<Vec<&str>>()[0];
                                    if excludes.contains(&String::from(fname)) {
                                        continue;
                                    } else if filename.contains("info") {
                                        datafiles.push(i.path());
                                    }
                                } else if filename.contains("info") {
                                    datafiles.push(i.path());
                                }
                            }
                        }
                        Err(e) => {
                            error!("Could not read data\nReason: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                error!(
                    "Unable to access directory {}\nReason: {}",
                    &hdir.to_string_lossy(),
                    e
                );
            }
        }
    }

    Ok(datafiles)
}

/// Extract hero specific data from the appropriate files
pub fn extract_data(datafiles: &[PathBuf]) -> Vec<Hero> {
    let mut heroes: Vec<Hero> = Vec::new();
    let re: Regex = Regex::new(r#"id\s"(\w*)"\s(.*)"#).unwrap();
    for hpath in datafiles {
        let cname = hpath.file_stem().unwrap().to_str().unwrap();
        let cname: Vec<&str> = cname.split('.').collect();

        // default empty Hero object which gets updated with proper data later
        let mut hero: Hero = Hero {
            name: String::from(cname[0]),
            data: Vec::new(),
            sknames: Vec::new(),
            skills: Vec::new(),
        };

        // files are not large but use a BufReader anyway for safety
        let cfile = File::open(hpath).unwrap();
        let mut buf: String = String::new();
        let mut reader: BufReader<File> = BufReader::new(cfile);
        reader.read_to_string(&mut buf).unwrap();

        // temporary variable to hold skill data to avoid duplicates since each skill has multiple
        // lines in the associated class info file
        // collect the names as the key and then each skill level's data as an array for the value
        let mut tmp_data: IndexMap<String, Vec<String>> = IndexMap::new();
        for line in buf.lines() {
            if line.starts_with("combat_skill") {
                let caps: Captures = re.captures(line).unwrap();
                let skill_name: String = caps[1].to_string();

                // need to keep track of the original skill names for each class
                hero.sknames.push(skill_name.clone());

                // check if the skill name already exists in the map
                // if so then copy the existing data, append the new line, and put it back
                // otherwise add the new key and line
                match tmp_data.get(&skill_name) {
                    Some(pr) => {
                        let mut data = pr.to_owned();
                        data.push(caps[2].to_string());
                        tmp_data.insert(skill_name, data);
                    }
                    None => {
                        tmp_data.insert(skill_name, vec![caps[2].to_string()]);
                    }
                }
            } else {
                // for any non-combat skill related data just add it to its own attribute
                hero.data.push(line.to_string());
            }
        }

        // for each skill we read in create a new Skill object and assign it to the hero
        for (idx, skill_name) in tmp_data.keys().enumerate() {
            let skill = Skill {
                pos: idx,
                class: hero.name.clone(),
                name: skill_name.to_string(),
                data: tmp_data.get(skill_name).unwrap().to_owned(),
            };

            hero.sknames.dedup();
            hero.skills.push(skill);
        }

        heroes.push(hero);
    }

    // return a list of all heros with their associated data
    heroes
}

/// Render localization strings template
pub fn render_localizations(
    translation: Translation,
    cmap: Vec<SkillLocalization>,
) -> Result<String, Box<dyn Error>> {
    // header information for properly structured XML used by the game for the mod strings
    let mut rendered = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>".to_string();
    rendered = format!("{}\n<language id=\"{}\">", rendered, translation.lang);

    // loop through the randomized skill map and lookup the proper display text for the new skills
    // the default skill name is used as a key for the lookup in the translation table
    for sk in cmap {
        for (old, new) in sk.map {
            rendered = format!(
                "{}\n<entry id=\"combat_skill_name_{}_{}\"><![CDATA[{}]]></entry>",
                rendered, &sk.class, old, &translation.map[&new]
            );
            rendered = format!(
                "{}\n<entry id=\"upgrade_tree_name_{}.{}\"><![CDATA[{}]]></entry>",
                rendered, &sk.class, old, &translation.map[&new]
            );
        }
    }

    // close the language and root tags to finalize the XML structure
    rendered = format!("{}\n</language></root>", rendered);

    Ok(rendered)
}

/// Randomize the hero skills and write the appropriate files to the mod directory
pub fn randomize(
    base_hpaths: &HashMap<String, PathBuf>,
    mod_hpath: &Path,
    heroes: Vec<Hero>,
    rng: StdRng,
) -> Vec<SkillLocalization> {
    info!("Randomizing skills");
    let mut seed_rng: StdRng = rng;

    // master collection holding all of the skills for all hero classes
    let mut skill_collection: Vec<Skill> = Vec::new();
    for hero in heroes.clone() {
        for skill in hero.skills {
            skill_collection.push(skill);
        }
    }

    // shuffle the skills into smaller groups to ease the randomization process
    let mut skill_groups = shuffle_skills(skill_collection, heroes.len(), &mut seed_rng);
    let mut skloc: Vec<SkillLocalization> = Vec::new();

    for hero in heroes {
        let hdir = Path::join(mod_hpath, Path::new(&hero.name));
        let hpath = Path::join(&hdir, Path::new(&format!("{}.info.darkest", &hero.name)));
        fs::create_dir_all(hdir).unwrap();
        let mut of = OpenOptions::new()
            .append(true)
            .create(true)
            .open(hpath)
            .unwrap();

        for line in &hero.data {
            // beast skills mean all 7 skills are selected at once causing a ui overflow and disables
            // skill selection, adjust the applicable lines to solve this
            let out_line = if &hero.name == "abomination" && line.starts_with("skill_selection") {
                line.replace("false", "true").replace('7', "4")
            } else if &hero.name == "abomination" && line.starts_with("generation") {
                line.replace('7', "4")
            } else {
                // ignore lines which are not applicable and write them out as is to the new hero file
                line.to_string()
            };
            of.write_fmt(format_args!("{}\n", out_line))
                .expect("could not write hero data");
        }

        // map of default skill names to their new randomized value
        // needed as the game requires each hero to have the proper keys in the files otherwise
        // it cannot properly render skill names
        let mut align: HashMap<String, String> = HashMap::new();

        // pick a skill group randomly for the current hero and write the new skill data
        // remove the chosen groups to avoid duplicates being assigned
        let gidx = seed_rng.random_range(0..skill_groups.len());
        let hgroup = &skill_groups[gidx].clone();
        skill_groups.remove(gidx);

        for (idx, hsname) in hero.sknames.iter().enumerate() {
            // update the skill alignment map and then write the skill data to the mod hero file
            align.insert(hsname.to_string(), hgroup[idx].name.clone());
            for line in &hgroup[idx].data {
                of.write_fmt(format_args!(
                    r#"combat_skill: .id "{}" {}"#,
                    hero.sknames[idx], line
                ))
                .unwrap();
                of.write_all(b"\n").unwrap();
            }

            // copy skills icons for the randomized skills to the appropriate hero for in game alignment
            let sk_class = &hgroup[idx].class;
            let sk_pos = hgroup[idx].pos;
            let from_fname = format!("{}.ability.{}.png", &sk_class, POS_STR[sk_pos]);
            let from_path = Path::join(base_hpaths.get(sk_class).unwrap(), Path::new(&from_fname));
            let to_fname: PathBuf = vec![
                Path::new(&hero.name),
                Path::new(&format!("{}.ability.{}.png", &hero.name, POS_STR[idx])),
            ]
            .into_iter()
            .collect();
            let to_path = Path::join(mod_hpath, Path::new(&to_fname));

            debug!(
                "{} {} - {} {}",
                &hero.name,
                &hero.sknames[idx].clone(),
                &sk_class,
                &hgroup[idx].name.clone()
            );
            debug!("{:?} {:?}", &to_fname, &from_fname);

            fs::copy(&from_path, &to_path).unwrap();
        }

        // build the SkillLocalization object which is used to template the mod skill names
        skloc.push(SkillLocalization {
            class: hero.name,
            map: align,
        })
    }

    skloc
}

/// Read localization strings from default game data
pub fn extract_localizations(install_dir: &Path) -> Result<Translation, Box<dyn Error>> {
    let mut translation = Translation {
        lang: String::from("english"),
        map: HashMap::new(),
    };
    let mut lfiles: Vec<PathBuf> = Vec::new();

    // build paths to default string tables including dlc
    let base_ldir = Path::join(install_dir, "localization");
    let base_lfile = Path::join(&base_ldir, "heroes.string_table.xml");
    let cc_ldir = ["dlc", "580100_crimson_court", "localization"]
        .iter()
        .collect::<PathBuf>();
    let cc_lfile = Path::join(install_dir, Path::join(&cc_ldir, "CC.string_table.xml"));
    let sb_ldir = ["dlc", "702540_shieldbreaker", "localization"]
        .iter()
        .collect::<PathBuf>();
    let sb_lfile = Path::join(
        install_dir,
        Path::join(&sb_ldir, "shieldbreaker.string_table.xml"),
    );

    // verify game string tables exist at the known location and are real files
    // this handles ignoring dlc data if not present
    for lfile in [base_lfile, cc_lfile, sb_lfile] {
        if lfile.exists() && lfile.is_file() {
            lfiles.push(lfile);
        }
    }

    // since only English is supported for the moment search for it explicitly
    let re_lang = Regex::new(r#"\s+<language id="english">(?s)(.*)</language>"#)?;
    let re_map = Regex::new(r#"<entry id="combat_skill_name_(.*)"><!\[CDATA\[(.*)\]\]></entry>"#)?;

    for file in lfiles {
        let content = fs::read_to_string(file)?;

        // capture all English language data from valid string table files
        if let Some(lcaps) = re_lang.captures(&content) {
            let entries = lcaps[1].to_string();
            let entries = entries.split("\r\n");

            for entry in entries {
                // because the xml is processed by regex just stop processing when the closing language tag is reached
                // this can be removed when proper multi language support is added
                if entry.contains("</language>") {
                    break;
                }
                if let Some(line) = re_map.captures(entry) {
                    let name = line[1].to_string();
                    if name.contains("level") {
                        continue;
                    }

                    // set the correct position in the strings for the actual skill names
                    // the underscores in space separated class names cause splitting issues by default
                    let sp_pos = if name.contains("man_at_arms") {
                        3
                    } else if name.contains("grave_robber")
                        || name.contains("plague_doctor")
                        || name.contains("bounty_hunter")
                    {
                        2
                    } else {
                        1
                    };

                    // map the original skill name to its displayed text
                    let sk_name = name.split('_').collect::<Vec<&str>>()[sp_pos..].join("_");
                    let sk_value = line[2].to_string();

                    if sk_value != "Move" {
                        translation.map.insert(sk_name, sk_value);
                    }
                }
            }
        }
    }

    Ok(translation)
}

/// Shuffle the full skill list into smaller groups
fn shuffle_skills(
    skill_collection: Vec<Skill>,
    group_count: usize,
    seed_rng: &mut StdRng,
) -> Vec<Vec<Skill>> {
    // master collection holding all of the skills for all hero classes
    let mut skill_collection: Vec<Skill> = skill_collection;
    let mut skill_groups: Vec<Vec<Skill>> = Vec::new();

    // Abomination beast skills are REQUIRED to be together in a group, define them here
    let beast_skills = [
        "transform".to_string(),
        "rake".to_string(),
        "rage".to_string(),
        "slam".to_string(),
    ];
    for hero_idx in 0..group_count {
        let mut group: Vec<Skill> = Vec::new();
        // each group should contain 8 skills total, max number is a variable for safety
        while group.len() < beast_skills.len() + 3 {
            let rand_idx = seed_rng.random_range(0..skill_collection.len());
            let skname = &skill_collection[rand_idx].name;
            // skip beast skills here as they are assigned to the final group to keep them together
            if !beast_skills.contains(skname) {
                group.push(skill_collection[rand_idx].clone());
                skill_collection.remove(rand_idx);
            }
            // stop on the last group and just add the remaining skills to end the loop
            // this should simply be the beast skills that were ignored earlier
            if hero_idx == group_count - 1 {
                group.append(&mut skill_collection);
            }
        }
        skill_groups.push(group);
    }

    skill_groups
}
