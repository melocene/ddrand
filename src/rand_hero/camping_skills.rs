use rand::rngs::StdRng;
use rand::seq::IteratorRandom;
use serde_json::Value;
use std::{
    collections::HashMap,
    error::Error,
    fs::File,
    io::{BufReader, BufWriter},
    path::PathBuf,
};

pub fn parse_from_file(file_path: &PathBuf) -> Result<Value, Box<dyn Error>> {
    let file = File::open(file_path);
    let reader = BufReader::new(file?);
    serde_json::from_reader(reader).map_err(|e| e.into())
}

pub fn randomize(mut skills_data: Value, rng: StdRng) -> Result<Value, Box<dyn Error>> {
    let mut seed_rng: StdRng = rng;
    let mut class_assigned_count: HashMap<String, u8> = HashMap::new();

    // The first camping skill should be `Encourage` which is available to all heroes.
    // Extract the full list of heroes for later assignment from this skill.
    let class_list: Option<Vec<String>> = skills_data
        .pointer("/skills/0/hero_classes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|val| val.as_str().map(|s| s.to_string()))
                .collect()
        });
    if let Some(class_names) = class_list {
        for name in class_names {
            class_assigned_count.insert(name, 0);
        }
    }

    if let Some(skill_list) = skills_data
        .pointer_mut("/skills")
        .and_then(|v| v.as_array_mut())
    {
        for skill in skill_list {
            // Skip the skills assigned to all classes.
            if let Some(id) = skill.get_mut("id").and_then(|v| v.as_str())
                && ["encourage", "first_aid", "pep_talk", "hobby"].contains(&id)
            {
                continue;
            }
            if let Some(hero_classes) = skill.get_mut("hero_classes").and_then(|c| c.as_array_mut())
            {
                // Skip any skill that may never have been assigned to any class in the base game.
                let count = hero_classes.len();
                if count == 0 {
                    continue;
                }

                hero_classes.clear();
                for _ in 0..count {
                    // We ignore common skills leaving only 4 slots as each class can have at most 7 skills.
                    // If a class already has 4 assigned skills, it cannot be assigned more so ignore it in the remaining choices.
                    let mut available_classes: Vec<_> = class_assigned_count
                        .iter()
                        .filter(|(_, v)| **v < 4)
                        .map(|(k, _)| k.clone())
                        .collect();
                    // Make sure to sort the results to always ensure consistent output for the same seed.
                    available_classes.sort();
                    if let Some(selected_class) =
                        available_classes.into_iter().choose(&mut seed_rng)
                    {
                        hero_classes.push(Value::String(selected_class.clone()));
                        if let Some(entry) = class_assigned_count.get_mut(&selected_class) {
                            *entry += 1;
                        }
                    }
                }
            }
        }
    }

    // Return the modified skill data.
    Ok(skills_data)
}

pub fn write_to_file(skills_data: &Value, file_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    let file = File::create(file_path)?;
    let buf_writer = BufWriter::new(file);
    serde_json::to_writer_pretty(buf_writer, skills_data)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed;

    #[test]
    fn test_randomize_skills_same_length() {
        let test_input_json = r#"
            {
                "skills": [
                    {
                        "id": "encourage",
                        "hero_classes": ["class1", "class2", "class3"]
                    },
                    {
                        "id": "skill1",
                        "hero_classes": ["class1", "class2"]
                    },
                    {
                        "id": "skill2",
                        "hero_classes": ["class2", "class3"]
                    }
                ]
            }
        "#;

        let expected_output_json: Value = serde_json::json!({
            "skills": [
                {
                    "id": "encourage",
                    "hero_classes": ["class1", "class2", "class3"]
                },
                {
                    "id": "skill1",
                    "hero_classes": ["class1", "class1"]
                },
                {
                    "id": "skill2",
                    "hero_classes": ["class1", "class2"]
                }
            ]
        });

        let test_seed_rng = seed::create_rng("testseed00");
        let skills_data: Value = serde_json::from_str(test_input_json).unwrap();
        let randomized_data = randomize(skills_data, test_seed_rng).unwrap();

        assert_eq!(randomized_data, expected_output_json);
    }

    #[test]
    fn test_randomize_skills_invalid_length() {
        let test_input_json = r#"
            {
                "skills": [
                    {
                        "id": "encourage",
                        "hero_classes": ["class1", "class2", "class3"]
                    },
                    {
                        "id": "skill1",
                        "hero_classes": ["class1", "class2"]
                    },
                    {
                        "id": "skill2",
                        "hero_classes": ["class2", "class3"]
                    }
                ]
            }
        "#;

        let expected_output_json: Value = serde_json::json!({
            "skills": [
                {
                    "id": "encourage",
                    "hero_classes": ["class1", "class2", "class3"]
                },
                {
                    "id": "skill1",
                    "hero_classes": ["class1"]
                },
                {
                    "id": "skill2",
                    "hero_classes": ["class1", "class2"]
                }
            ]
        });

        let test_seed_rng = seed::create_rng("testseed00");
        let skills_data: Value = serde_json::from_str(test_input_json).unwrap();
        let randomized_data = randomize(skills_data, test_seed_rng).unwrap();

        assert_ne!(randomized_data, expected_output_json);
    }

    #[test]
    fn test_randomize_skills_empty_length() {
        let test_input_json = r#"
            {
                "skills": [
                    {
                        "id": "encourage",
                        "hero_classes": ["class1", "class2", "class3"]
                    },
                    {
                        "id": "skill1",
                        "hero_classes": []
                    },
                    {
                        "id": "skill2",
                        "hero_classes": ["class2", "class3"]
                    }
                ]
            }
        "#;

        let expected_output_json: Value = serde_json::json!({
            "skills": [
                {
                    "id": "encourage",
                    "hero_classes": ["class1", "class2", "class3"]
                },
                {
                    "id": "skill1",
                    "hero_classes": []
                },
                {
                    "id": "skill2",
                    "hero_classes": ["class1", "class1"]
                }
            ]
        });

        let test_seed_rng = seed::create_rng("testseed00");
        let skills_data: Value = serde_json::from_str(test_input_json).unwrap();
        let randomized_data = randomize(skills_data, test_seed_rng).unwrap();

        assert_eq!(randomized_data, expected_output_json);
    }
}
