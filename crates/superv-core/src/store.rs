use crate::filter::{self, Reject, MAX_ITEM_BYTES};
use crate::model::{Content, Item, NewItem};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("rejected: {0:?}")]
    Rejected(Reject),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct StoreConfig {
    /// Max unpinned items kept.
    pub max_items: usize,
    pub max_item_bytes: usize,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            max_items: 25,
            max_item_bytes: MAX_ITEM_BYTES,
        }
    }
}

pub struct Store {
    conn: Connection,
    cfg: StoreConfig,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl Store {
    pub fn open(path: impl AsRef<Path>, cfg: StoreConfig) -> Result<Self> {
        Self::init(Connection::open(path)?, cfg)
    }
    pub fn open_in_memory(cfg: StoreConfig) -> Result<Self> {
        Self::init(Connection::open_in_memory()?, cfg)
    }
    fn init(conn: Connection, cfg: StoreConfig) -> Result<Self> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS items(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hash TEXT NOT NULL UNIQUE,
                kind TEXT NOT NULL,
                data BLOB NOT NULL,
                pinned INTEGER NOT NULL DEFAULT 0,
                updated_ms INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS items_updated ON items(updated_ms DESC);",
        )?;
        Ok(Self { conn, cfg })
    }

    /// Record a copy. A duplicate is bumped to the top instead of re-added.
    /// Returns the item id.
    pub fn add(&mut self, item: &NewItem) -> Result<i64> {
        filter::check(item, self.cfg.max_item_bytes).map_err(Error::Rejected)?;
        let hash = item.content.hash();
        let now = self.next_ts();
        let existing: Option<i64> = self
            .conn
            .query_row("SELECT id FROM items WHERE hash=?", [&hash], |r| r.get(0))
            .optional()?;
        let id = if let Some(id) = existing {
            self.conn
                .execute("UPDATE items SET updated_ms=? WHERE id=?", params![now, id])?;
            id
        } else {
            self.conn.execute(
                "INSERT INTO items(hash,kind,data,pinned,updated_ms) VALUES(?,?,?,0,?)",
                params![hash, item.content.kind(), item.content.bytes(), now],
            )?;
            self.conn.last_insert_rowid()
        };
        self.prune()?;
        Ok(id)
    }

    /// Strictly increasing timestamp so ordering is stable within a millisecond.
    fn next_ts(&self) -> i64 {
        let max: i64 = self
            .conn
            .query_row("SELECT COALESCE(MAX(updated_ms),0) FROM items", [], |r| {
                r.get(0)
            })
            .unwrap_or(0);
        now_ms().max(max + 1)
    }

    fn prune(&mut self) -> Result<()> {
        self.conn.execute(
            "DELETE FROM items WHERE pinned=0 AND id NOT IN (
                SELECT id FROM items WHERE pinned=0 ORDER BY updated_ms DESC LIMIT ?)",
            [self.cfg.max_items as i64],
        )?;
        Ok(())
    }

    /// Pinned items first, then newest first.
    pub fn list(&self) -> Result<Vec<Item>> {
        let mut st = self.conn.prepare(
            "SELECT id,kind,data,pinned,updated_ms FROM items ORDER BY pinned DESC, updated_ms DESC",
        )?;
        let rows = st.query_map([], |r| {
            let kind: String = r.get(1)?;
            let data: Vec<u8> = r.get(2)?;
            let content = if kind == "image" {
                Content::Image(data)
            } else {
                Content::Text(String::from_utf8_lossy(&data).into_owned())
            };
            Ok(Item {
                id: r.get(0)?,
                content,
                pinned: r.get::<_, i64>(3)? != 0,
                updated_ms: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    pub fn get(&self, id: i64) -> Result<Option<Item>> {
        Ok(self.list()?.into_iter().find(|i| i.id == id))
    }

    pub fn set_pinned(&mut self, id: i64, pinned: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE items SET pinned=? WHERE id=?",
            params![pinned as i64, id],
        )?;
        self.prune()
    }

    pub fn delete(&mut self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM items WHERE id=?", [id])?;
        Ok(())
    }

    /// "Clear all": removes everything except pinned items.
    pub fn clear(&mut self) -> Result<()> {
        self.conn.execute("DELETE FROM items WHERE pinned=0", [])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Store {
        Store::open_in_memory(StoreConfig::default()).unwrap()
    }

    #[test]
    fn caps_at_25_unpinned() {
        let mut s = mem();
        for i in 0..40 {
            s.add(&NewItem::text(format!("item {i}"))).unwrap();
        }
        let l = s.list().unwrap();
        assert_eq!(l.len(), 25);
        assert_eq!(l[0].content, Content::Text("item 39".into()));
    }

    #[test]
    fn pinned_exempt_from_cap_and_clear() {
        let mut s = mem();
        let keep = s.add(&NewItem::text("keep")).unwrap();
        s.set_pinned(keep, true).unwrap();
        for i in 0..30 {
            s.add(&NewItem::text(format!("x{i}"))).unwrap();
        }
        let l = s.list().unwrap();
        assert_eq!(l.len(), 26);
        assert!(l[0].pinned);
        s.clear().unwrap();
        let l = s.list().unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].id, keep);
    }

    #[test]
    fn duplicate_bumps_to_top() {
        let mut s = mem();
        let a = s.add(&NewItem::text("a")).unwrap();
        s.add(&NewItem::text("b")).unwrap();
        let a2 = s.add(&NewItem::text("a")).unwrap();
        assert_eq!(a, a2);
        let l = s.list().unwrap();
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].id, a);
    }

    #[test]
    fn rejects_oversize_and_empty_but_keeps_passwords() {
        let mut s = mem();
        let big = NewItem::text("x".repeat(MAX_ITEM_BYTES + 1));
        assert!(matches!(
            s.add(&big),
            Err(Error::Rejected(Reject::TooLarge))
        ));
        assert!(matches!(
            s.add(&NewItem::text("")),
            Err(Error::Rejected(Reject::Empty))
        ));
        // Password-manager copies are recorded like anything else.
        let mut sens = NewItem::text("hunter2");
        sens.mimes = vec!["x-kde-passwordManagerHint".into()];
        s.add(&sens).unwrap();
        assert_eq!(s.list().unwrap().len(), 1);
    }

    #[test]
    fn delete_and_images() {
        let mut s = mem();
        let id = s.add(&NewItem::image(vec![1, 2, 3])).unwrap();
        assert_eq!(
            s.get(id).unwrap().unwrap().content,
            Content::Image(vec![1, 2, 3])
        );
        s.delete(id).unwrap();
        assert!(s.get(id).unwrap().is_none());
    }

    #[test]
    fn persists_across_reopen() {
        let dir = std::env::temp_dir().join(format!("superv-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("h.db");
        {
            let mut s = Store::open(&path, StoreConfig::default()).unwrap();
            let id = s.add(&NewItem::text("persist")).unwrap();
            s.set_pinned(id, true).unwrap();
        }
        let s = Store::open(&path, StoreConfig::default()).unwrap();
        let l = s.list().unwrap();
        assert_eq!(l.len(), 1);
        assert!(l[0].pinned);
        std::fs::remove_dir_all(dir).ok();
    }
}
