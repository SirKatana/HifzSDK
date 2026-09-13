use serde_json::{json, Map, Value};

pub const BASE_JSON: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/data/quran.base.json");

const VERSES_PER_DAY: usize = 10;
const SABQI_WINDOW: usize = 5;
const MANZIL_DAYS: usize = 15;

#[derive(Debug, Clone, PartialEq)]
pub struct VerseRef {
    pub chapter: u32,
    pub verse: u32,
}

/// Load the ordered mushaf (chapter map form from quran.base.json).
/// Shape: { "1": [ {chapter, verse, text}, ... ], ... }
pub fn load_verses() -> Vec<VerseRef> {
    let raw = std::fs::read_to_string(BASE_JSON).expect("data/quran.base.json missing");
    let root: Value = serde_json::from_str(&raw).expect("quran.base.json invalid JSON");
    let mut keys: Vec<u32> = root
        .as_object()
        .map(|m| m.keys().filter_map(|k| k.parse().ok()).collect())
        .unwrap_or_default();
    keys.sort_unstable();

    let mut out: Vec<VerseRef> = Vec::new();
    if let Some(map) = root.as_object() {
        for num in keys {
            let arr = map
                .get(&num.to_string())
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for v in arr {
                out.push(VerseRef {
                    chapter: v["chapter"].as_u64().unwrap_or(num as u64) as u32,
                    verse: v["verse"].as_u64().unwrap_or(0) as u32,
                });
            }
        }
    }
    out
}
