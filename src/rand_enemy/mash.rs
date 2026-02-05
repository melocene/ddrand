use log::*;
use rand::{Rng, rngs::StdRng};
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::BufReader;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Mash {
    name: String,
    id: String,
    hall: Vec<String>,
    room: Vec<String>,
    boss: Vec<String>,
    stall: Vec<String>,
    named: Vec<String>,
}

pub fn get_data_files(
    dungeon_paths: &HashMap<String, PathBuf>,
    excludes: &Option<Vec<String>>,
) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut dungeon_files: Vec<PathBuf> = Vec::new();
    for (k, p) in dungeon_paths.iter() {
        // range is inclusive on the low end, but exclusive on the high end
        for num in (1..6).step_by(2) {
            let fname = format!("{}.{}.mash.darkest", k, num);
            let fpath = Path::join(p, Path::new(&fname));
            if fpath.exists() && fpath.is_file() {
                if let Some(excludes) = &excludes {
                    let fname = fpath
                        .file_name()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .split('.')
                        .collect::<Vec<&str>>()[0];
                    if excludes.contains(&String::from(fname)) {
                        continue;
                    } else {
                        dungeon_files.push(fpath);
                    }
                } else {
                    dungeon_files.push(fpath);
                }
            }
        }
    }

    Ok(dungeon_files)
}

pub fn extract_data(datafiles: &[PathBuf]) -> Result<Vec<Mash>, Box<dyn Error>> {
    let mut mashes: Vec<Mash> = Vec::new();
    for dpath in datafiles {
        let fname = dpath.file_stem().unwrap().to_str().unwrap();
        let fname: Vec<&str> = fname.split('.').collect();

        let mut mash = Mash {
            name: String::from(fname[0]),
            id: String::from(fname[1]),
            hall: Vec::new(),
            room: Vec::new(),
            boss: Vec::new(),
            stall: Vec::new(),
            named: Vec::new(),
        };

        // files are not large but use a BufReader anyway for safety
        let dfile = File::open(dpath).unwrap();
        let mut buf: String = String::new();
        let mut reader: BufReader<File> = BufReader::new(dfile);
        reader.read_to_string(&mut buf).unwrap();

        // build the mash using the data from each line of the file
        // ignore lines that are not relevant
        for line in buf.lines() {
            match line[0..4].to_string().trim() {
                "hall" => mash.hall.push(String::from(line)),
                "room" => mash.room.push(String::from(line)),
                "boss" => mash.boss.push(String::from(line)),
                "stal" => mash.stall.push(String::from(line)),
                "name" => mash.named.push(String::from(line)),
                _ => (),
            }
        }

        mashes.push(mash);
    }

    Ok(mashes)
}

pub fn randomize(
    mod_dpath: &Path,
    mashes: Vec<Mash>,
    rng: StdRng,
    rand_boss: bool,
    rand_mash: bool,
) {
    let mut seed_rng: StdRng = rng;
    let mut boss_groups: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    let mut hall_groups: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    let mut room_groups: HashMap<String, Vec<Vec<String>>> = HashMap::new();

    if rand_boss {
        info!("Randomizing boss spawns");
        let mut boss_collection: HashMap<String, Vec<String>> = HashMap::new();
        for mash in mashes.clone() {
            match boss_collection.get_mut(&mash.id) {
                Some(pr) => {
                    let mut tmp = pr.to_owned();
                    for s in mash.boss {
                        tmp.push(s);
                    }
                    boss_collection.insert(mash.id, tmp);
                }
                None => {
                    boss_collection.insert(mash.id, mash.boss);
                }
            }
        }
        for level in boss_collection.keys() {
            let group = shuffle_mash_loc(
                boss_collection.get(level).unwrap().clone(),
                4,
                &mut seed_rng,
            );
            boss_groups.insert(level.to_string(), group);
        }
    }

    if rand_mash {
        info!("Randomizing monster spawns");
        let mut hall_collection: HashMap<String, Vec<String>> = HashMap::new();
        for mash in mashes.clone() {
            match hall_collection.get_mut(&mash.id) {
                Some(pr) => {
                    let mut tmp = pr.to_owned();
                    for s in mash.hall {
                        tmp.push(s);
                    }
                    hall_collection.insert(mash.id, tmp);
                }
                None => {
                    hall_collection.insert(mash.id, mash.hall);
                }
            }
        }

        let mut room_collection: HashMap<String, Vec<String>> = HashMap::new();
        for mash in mashes.clone() {
            match room_collection.get_mut(&mash.id) {
                Some(pr) => {
                    let mut tmp = pr.to_owned();
                    for s in mash.room {
                        tmp.push(s);
                    }
                    room_collection.insert(mash.id, tmp);
                }
                None => {
                    room_collection.insert(mash.id, mash.room);
                }
            }
        }

        for level in hall_collection.keys() {
            let group = shuffle_mash_loc(
                hall_collection.get(level).unwrap().clone(),
                4,
                &mut seed_rng,
            );
            hall_groups.insert(level.to_string(), group);
        }

        for level in room_collection.keys() {
            let group = shuffle_mash_loc(
                room_collection.get(level).unwrap().clone(),
                4,
                &mut seed_rng,
            );
            room_groups.insert(level.to_string(), group);
        }
    }

    debug!("{:#?}", &boss_groups);
    debug!("{:#?}", &hall_groups);
    debug!("{:#?}", &room_groups);

    for mash in mashes {
        let data: Vec<String> = match (rand_boss, rand_mash) {
            // Both randomized: only named + stall (boss, hall, room added later)
            (true, true) => mash
                .named
                .into_iter()
                .chain(mash.stall.into_iter())
                .collect(),
            // Only boss randomized: include hall + room as-is
            (true, false) => mash
                .named
                .into_iter()
                .chain(mash.stall.into_iter())
                .chain(mash.hall.into_iter())
                .chain(mash.room.into_iter())
                .collect(),
            // Only monsters randomized: include boss as-is
            (false, true) => mash
                .boss
                .into_iter()
                .chain(mash.named.into_iter())
                .chain(mash.stall.into_iter())
                .collect(),
            // Neither (fallback, shouldn't happen)
            (false, false) => mash
                .boss
                .into_iter()
                .chain(mash.named.into_iter())
                .chain(mash.stall.into_iter())
                .chain(mash.hall.into_iter())
                .chain(mash.room.into_iter())
                .collect(),
        };
        let mdir = Path::join(mod_dpath, Path::new(&mash.name));
        let mpath = Path::join(
            &mdir,
            Path::new(&format!("{}.{}.mash.darkest", &mash.name, &mash.id)),
        );

        let mut data_lines = vec![data.into_iter()];
        if rand_mash && !hall_groups.is_empty() && !room_groups.is_empty() {
            let hgidx = seed_rng.random_range(0..hall_groups.get(&mash.id).unwrap().len());
            let hgroup = hall_groups
                .get_mut(&mash.id)
                .unwrap()
                .remove(hgidx)
                .into_iter();
            data_lines.push(hgroup);

            let rgidx = seed_rng.random_range(0..room_groups.get(&mash.id).unwrap().len());
            let rgroup = room_groups
                .get_mut(&mash.id)
                .unwrap()
                .remove(rgidx)
                .into_iter();
            data_lines.push(rgroup);
        }

        if rand_boss && !boss_groups.is_empty() {
            let bgidx = seed_rng.random_range(0..boss_groups.get(&mash.id).unwrap().len());
            let bgroup = boss_groups
                .get_mut(&mash.id)
                .unwrap()
                .remove(bgidx)
                .into_iter();
            data_lines.push(bgroup);
        }

        fs::create_dir_all(mdir).unwrap();
        let mut of = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(mpath)
            .unwrap();

        for g in data_lines {
            for line in g {
                of.write_fmt(format_args!("{}\n", line)).unwrap();
            }
        }
    }
}

fn shuffle_mash_loc(
    loc_mash: Vec<String>,
    group_count: usize,
    seed_rng: &mut StdRng,
) -> Vec<Vec<String>> {
    let item_count = loc_mash.len() / group_count;
    let mut tloc_mash = loc_mash;
    let mut groups: Vec<Vec<String>> = Vec::new();

    for _ in 0..group_count {
        let mut group: Vec<String> = Vec::new();
        while group.len() < item_count {
            let rand_idx = seed_rng.random_range(0..tloc_mash.len());
            let line = &tloc_mash[rand_idx];
            group.push(line.to_string());
            tloc_mash.remove(rand_idx);
        }

        groups.push(group);
    }

    groups
}
