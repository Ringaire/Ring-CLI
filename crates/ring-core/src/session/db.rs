use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use tracing::{debug, warn};
use uuid::Uuid;

use crate::session::paths;
use crate::session::SessionMeta;

// ── JSON 索引文件路径 ────────────────────────────────────────────────────────

fn index_path() -> PathBuf {
    paths::sessions_dir().join("index.json")
}

// ── 内存索引（进程生命周期内缓存）────────────────────────────────────────────

type Index = Arc<Mutex<Vec<SessionMeta>>>;

static INDEX: OnceLock<Index> = OnceLock::new();

fn index() -> &'static Index {
    INDEX.get().expect("session index not initialized -- call init() first")
}

// ── 初始化 ───────────────────────────────────────────────────────────────────

pub fn init() {
    INDEX.get_or_init(|| {
        let entries = load_from_disk();
        debug!("session index loaded: {} entries", entries.len());
        Arc::new(Mutex::new(entries))
    });
}

fn load_from_disk() -> Vec<SessionMeta> {
    let path = index_path();
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_else(|e| {
            warn!("failed to parse session index: {e}, starting fresh");
            Vec::new()
        }),
        Err(_) => Vec::new(),
    }
}

fn flush_to_disk(entries: &[SessionMeta]) {
    let path = index_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // 原子写入：先写临时文件，再 rename
    let tmp = path.with_extension("json.tmp");
    match serde_json::to_vec_pretty(entries) {
        Ok(data) => {
            if let Err(e) = std::fs::write(&tmp, &data) {
                warn!("failed to write session index tmp: {e}");
                return;
            }
            if let Err(e) = std::fs::rename(&tmp, &path) {
                warn!("failed to rename session index: {e}");
                let _ = std::fs::remove_file(&tmp);
            }
        }
        Err(e) => warn!("failed to serialize session index: {e}"),
    }
}

// ── 公共 API（与原 SQLite 版本保持一致）──────────────────────────────────────

pub fn upsert_thread(meta: &SessionMeta) {
    let idx = index();
    let mut entries = idx.lock().unwrap();

    if let Some(existing) = entries.iter_mut().find(|e| e.id == meta.id) {
        *existing = meta.clone();
    } else {
        entries.push(meta.clone());
    }

    flush_to_disk(&entries);
}

pub fn delete_thread(id: Uuid) {
    let idx = index();
    let mut entries = idx.lock().unwrap();
    let before = entries.len();
    entries.retain(|e| e.id != id);

    if entries.len() < before {
        flush_to_disk(&entries);
    }
}

pub fn get_thread(id: Uuid) -> Option<SessionMeta> {
    let idx = index();
    let entries = idx.lock().unwrap();
    entries.iter().find(|e| e.id == id).cloned()
}

pub fn list_threads(limit: i64) -> Vec<SessionMeta> {
    let idx = index();
    let entries = idx.lock().unwrap();

    let mut sorted: Vec<SessionMeta> = entries
        .iter()
        .cloned()
        .collect();
    sorted.sort_by_key(|e| std::cmp::Reverse(e.updated_at));
    sorted.truncate(limit as usize);
    sorted
}

pub fn search_threads(query: &str) -> Vec<SessionMeta> {
    let idx = index();
    let entries = idx.lock().unwrap();
    let lower = query.to_lowercase();

    let mut matched: Vec<SessionMeta> = entries
        .iter()
        .filter(|e| {
            e.title
                .as_deref()
                .map(|t| t.to_lowercase().contains(&lower))
                .unwrap_or(false)
        })
        .cloned()
        .collect();

    matched.sort_by_key(|e| std::cmp::Reverse(e.updated_at));
    matched.truncate(100);
    matched
}

pub fn set_compressed(id: Uuid, compressed: bool) {
    // JSON 索引不追踪压缩状态 — 由文件系统 (.zst 后缀) 自描述。
    // 保留签名兼容，compression.rs 调用时不报错。
    let _ = (id, compressed);
}

pub fn set_archived(id: Uuid, archived: bool) {
    if archived {
        delete_thread(id);
    }
    let _ = archived;
}

pub fn count() -> usize {
    let idx = index();
    let entries = idx.lock().unwrap();
    entries.len()
}
