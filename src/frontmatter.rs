use anyhow::Result;
use serde_json::Value;

pub fn parse_matter(matter: String) -> Result<Option<Value>> {
    if matter.is_empty() {
        Ok(None)
    } else {
        Ok(Some(serde_yaml_ng::from_str(&matter)?))
    }
}
