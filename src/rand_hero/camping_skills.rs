use serde_json::Value;
use std::error::Error;

pub fn _get_camping_skills_data(data: &str) -> Result<Value, Box<dyn Error>> {
    let camping_skills_data = serde_json::from_str(data)?;
    Ok(camping_skills_data)
}

pub fn _randomize_camping_skills(camping_skills_data: Value) -> Result<Value, Box<dyn Error>> {
    Ok(camping_skills_data)
}

pub fn _write_randomized_camping_skills(_camping_skills_data: Value) -> Result<(), Box<dyn Error>> {
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use super::*;
// }
