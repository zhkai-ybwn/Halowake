use super::*;
use std::{
    borrow::Cow,
    sync::{Arc, Mutex, OnceLock},
    time::SystemTime,
};

const CACHE_BYTES: usize = 64 * 1024 * 1024;
const FILE_BYTES: usize = 4 * 1024 * 1024;
const CACHE_FILES: usize = 4096;

#[derive(Clone, PartialEq)]
struct Fingerprint {
    len: u64,
    modified: SystemTime,
    created: Option<SystemTime>,
}
fn fingerprint(path: &Path) -> Option<Fingerprint> {
    let meta = fs::metadata(path).ok()?;
    Some(Fingerprint {
        len: meta.len(),
        modified: meta.modified().ok()?,
        created: meta.created().ok(),
    })
}
struct Cached {
    fingerprint: Fingerprint,
    entries: Arc<Vec<Value>>,
    weight: usize,
    used: u64,
}
#[derive(Default)]
struct Cache {
    files: HashMap<PathBuf, Cached>,
    bytes: usize,
    tick: u64,
}
static CACHE: OnceLock<Mutex<Cache>> = OnceLock::new();
// Same-file misses share a gate. Different files may still read in parallel.
static GATES: OnceLock<[Mutex<()>; 16]> = OnceLock::new();

pub(super) enum HistoryRecords {
    Cached(Arc<Vec<Value>>),
    Streaming(Box<dyn BufRead>),
}
impl HistoryRecords {
    pub(super) fn iter(&mut self) -> Box<dyn Iterator<Item = Cow<'_, Value>> + '_> {
        match self {
            Self::Cached(entries) => Box::new(entries.iter().map(Cow::Borrowed)),
            Self::Streaming(reader) => Box::new(
                reader
                    .lines()
                    .map_while(Result::ok)
                    .filter_map(|line| parse_record(&line))
                    .map(Cow::Owned),
            ),
        }
    }
}

fn parse_record(line: &str) -> Option<Value> {
    if ![
        "\"session_meta\"",
        "\"user\"",
        "\"assistant\"",
        "\"user_message\"",
        "\"agent_message\"",
        "\"USER_INPUT\"",
        "\"PLANNER_RESPONSE\"",
    ]
    .iter()
    .any(|tag| line.contains(tag))
    {
        return None;
    }
    serde_json::from_str(line).ok()
}

pub(super) fn records(path: &Path) -> Option<HistoryRecords> {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hasher);
    let gates = GATES.get_or_init(|| std::array::from_fn(|_| Mutex::new(())));
    let _gate = gates[hasher.finish() as usize % gates.len()].lock().ok()?;
    let before = fingerprint(path)?;
    let cache = CACHE.get_or_init(|| Mutex::new(Cache::default()));
    {
        let mut cache = cache.lock().ok()?;
        cache.tick += 1;
        let tick = cache.tick;
        if let Some(found) = cache.files.get_mut(path) {
            if found.fingerprint == before {
                found.used = tick;
                return Some(HistoryRecords::Cached(Arc::clone(&found.entries)));
            }
        }
        if let Some(old) = cache.files.remove(path) {
            cache.bytes -= old.weight;
        }
    }
    let mut entries = Vec::new();
    let mut weight = 0;
    for line in history_reader(path)?.lines() {
        let line = match line {
            Ok(line) => line,
            // Preserve the original streaming parser's usable prefix; never cache a partial read.
            Err(_) => return Some(HistoryRecords::Streaming(history_reader(path)?)),
        };
        if let Some(record) = parse_record(&line) {
            // Conservatively account for JSON allocation overhead, not just source bytes.
            weight += line.len().saturating_mul(4) + 128;
            if weight > FILE_BYTES {
                return Some(HistoryRecords::Streaming(history_reader(path)?));
            }
            entries.push(record);
        }
    }
    let entries = Arc::new(entries);
    if fingerprint(path).as_ref() == Some(&before) {
        let mut cache = cache.lock().ok()?;
        while cache.bytes + weight > CACHE_BYTES || cache.files.len() >= CACHE_FILES {
            let oldest = cache
                .files
                .iter()
                .min_by_key(|(_, entry)| entry.used)
                .map(|(path, _)| path.clone());
            let Some(oldest) = oldest else {
                break;
            };
            if let Some(old) = cache.files.remove(&oldest) {
                cache.bytes -= old.weight;
            }
        }
        cache.tick += 1;
        let used = cache.tick;
        cache.bytes += weight;
        cache.files.insert(
            path.to_path_buf(),
            Cached {
                fingerprint: before,
                entries: Arc::clone(&entries),
                weight,
                used,
            },
        );
    }
    Some(HistoryRecords::Cached(entries))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn read_errors_keep_the_usable_prefix_without_caching_it() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("partial.jsonl");
        fs::write(&path, b"{\"type\":\"user\"}\n\xff\n").unwrap();
        assert!(matches!(records(&path), Some(HistoryRecords::Streaming(_))));
        assert_eq!(records(&path).unwrap().iter().count(), 1);
    }

    #[test]
    fn oversized_files_stream_without_retaining_their_records() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("large.jsonl");
        let record =
            serde_json::json!({"type":"user", "text":"x".repeat(FILE_BYTES / 4)}).to_string();
        fs::write(&path, record).unwrap();
        assert!(matches!(records(&path), Some(HistoryRecords::Streaming(_))));
        assert_eq!(records(&path).unwrap().iter().count(), 1);
        assert!(!CACHE
            .get()
            .unwrap()
            .lock()
            .unwrap()
            .files
            .contains_key(&path));
    }

    #[test]
    fn reuses_unchanged_records_and_invalidates_append_truncate_delete() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("history.jsonl");
        let line = "{\"type\":\"user\",\"message\":\"one\"}\n";
        fs::write(&path, line).unwrap();
        let HistoryRecords::Cached(a) = records(&path).unwrap() else {
            panic!()
        };
        let HistoryRecords::Cached(b) = records(&path).unwrap() else {
            panic!()
        };
        assert!(Arc::ptr_eq(&a, &b));
        fs::write(&path, format!("{line}{line}")).unwrap();
        assert_eq!(records(&path).unwrap().iter().count(), 2);
        fs::write(&path, "").unwrap();
        assert_eq!(records(&path).unwrap().iter().count(), 0);
        fs::remove_file(&path).unwrap();
        assert!(records(&path).is_none());
        fs::write(&path, line).unwrap();
        assert_eq!(records(&path).unwrap().iter().count(), 1);
    }
    #[test]
    fn simultaneous_readers_share_one_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("history.jsonl");
        fs::write(&path, "{\"type\":\"user\"}\n").unwrap();
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        let HistoryRecords::Cached(entries) = records(&path).unwrap() else {
                            panic!()
                        };
                        entries
                    })
                })
                .collect();
            let snapshots: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
            assert!(snapshots
                .iter()
                .all(|entries| Arc::ptr_eq(entries, &snapshots[0])));
        });
    }
}
