
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

#[derive(Debug, Clone)]
pub struct Chapter {
    pub number: u32,
    pub verses: Vec<VerseRef>,
}

/// Parse quran.base.json -> ordered chapters.
/// Shape: { "1": [ {chapter, verse, text}, ... ], ... }
pub fn parse_quran(raw: &str) -> Vec<Chapter> {
    let root: Value = serde_json::from_str(raw).expect("quran.base.json invalid JSON");
    let mut keys: Vec<u32> = root
        .as_object()
        .map(|m| m.keys().filter_map(|k| k.parse().ok()).collect())
        .unwrap_or_default();
    keys.sort_unstable();

    let mut chapters: Vec<Chapter> = Vec::new();
    if let Some(map) = root.as_object() {
        for num in keys {
            let arr = map
                .get(&num.to_string())
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let verses = arr
                .iter()
                .map(|v| VerseRef {
                    chapter: v["chapter"].as_u64().unwrap_or(num as u64) as u32,
                    verse: v["verse"].as_u64().unwrap_or(0) as u32,
                })
                .collect();
            chapters.push(Chapter { number: num, verses });
        }
    }
    chapters
}

fn load_base() -> Vec<Chapter> {
    let raw = std::fs::read_to_string(BASE_JSON).expect("data/quran.base.json missing");
    parse_quran(&raw)
}

/// Flatten the whole mushaf into one ordered verse list.
pub fn flatten(chapters: &[Chapter]) -> Vec<VerseRef> {
    chapters
        .iter()
        .flat_map(|c| c.verses.iter().cloned())
        .collect()
}

/// Split the mushaf into daily "sabqi" lessons (~10 verses each),
/// never breaking a surah across a daily lesson.
pub fn chunk_daily(verses: &[VerseRef], verses_per_day: usize) -> Vec<Vec<VerseRef>> {
    let mut days: Vec<Vec<VerseRef>> = Vec::new();
    let mut current: Vec<VerseRef> = Vec::new();
    let mut i = 0usize;
    while i < verses.len() {
        let chapter = verses[i].chapter;
        let mut j = i;
        while j < verses.len() && verses[j].chapter == chapter {
            j += 1;
        }
        let surah = &verses[i..j];
        let mut k = 0usize;
        while k < surah.len() {
            let take = (verses_per_day - current.len()).min(surah.len() - k);
            current.extend_from_slice(&surah[k..k + take]);
            k += take;
            if current.len() == verses_per_day {
                days.push(std::mem::take(&mut current));
            }
        }
        i = j;
    }
    if !current.is_empty() {
        days.push(current);
    }
    days
}

fn refs(lesson: &[VerseRef]) -> Vec<Value> {
    lesson
        .iter()
        .map(|v| json!({ "chapter": v.chapter, "verse": v.verse }))
        .collect()
}

/// Build the Sabqi -> Manzil plan from the whole mushaf.
///  - sabqi_new:    today's new lesson
///  - sabqi_review: the previous SABQI_WINDOW lessons
///  - manzil_review: the rolling window before the sabqi window
pub fn build_plan(chapters: &[Chapter]) -> Value {
    let verses = flatten(chapters);
    let daily = chunk_daily(&verses, VERSES_PER_DAY);
    build_plan_from_days(&daily)
}

pub fn build_plan_from_days(daily: &[Vec<VerseRef>]) -> Value {
    let mut steps: Vec<Value> = Vec::new();
    for (idx, lesson) in daily.iter().enumerate() {
        let day = (idx + 1) as u64;

        let sabqi_new = refs(lesson);
        let mut sabqi_review: Vec<Value> = Vec::new();
        let mut manzil_review: Vec<Value> = Vec::new();

        let sabqi_start = if idx >= SABQI_WINDOW {
            idx - SABQI_WINDOW
        } else {
            0
        };
        let manzil_start = if idx >= SABQI_WINDOW + MANZIL_DAYS {
            idx - (SABQI_WINDOW + MANZIL_DAYS)
        } else {
            0
        };
        if manzil_start < sabqi_start {
            for k in manzil_start..sabqi_start {
                manzil_review.extend(refs(&daily[k]));
            }
        }
        for k in sabqi_start..idx {
            sabqi_review.extend(refs(&daily[k]));
        }

        steps.push(json!({
            "day": day,
            "sabqi_new": sabqi_new,
            "sabqi_review": sabqi_review,
            "manzil_review": manzil_review,
        }));
    }

    json!({
        "method": "Sabqi -> Manzil",
        "description": "Daily hifz routine: memorize a new Sabqi every day, revise the last few lessons as Sabqi, and keep a rolling Manzil review of the previous fortnight.",
        "verses_per_day": 10,
        "sabqi_review_days": SABQI_WINDOW,
        "manzil_review_days": MANZIL_DAYS,
        "total_days": steps.len(),
        "steps": steps,
    })
}

/// Combine the original chapter map with the generated plan.
/// Produces: { "1": [...], ..., "hifz_plan": {...} }
pub fn combined_json(chapters: &[Chapter], plan: &Value) -> Value {
    let raw = std::fs::read_to_string(BASE_JSON).expect("data/quran.base.json missing");
    let mut root: Map<String, Value> =
        serde_json::from_str(&raw).expect("quran.base.json not an object");

    let mut meta = Map::new();
    meta.insert("name".into(), json!("Moga Hifz Quran"));
    meta.insert(
        "sources".into(),
        json!({ "chapter_count": chapters.len(), "verse_count": flatten(chapters).len() }),
    );
    root.insert("meta".into(), Value::Object(meta));
    root.insert("hifz_plan".into(), plan.clone());

    Value::Object(root)
}

/// The full SDK payload: whole Quran + memorization plan.
/// This is what gets sent to a Swift app.
pub fn full_export() -> Value {
    let chapters = load_base();
    let plan = build_plan(&chapters);
    combined_json(&chapters, &plan)
}
