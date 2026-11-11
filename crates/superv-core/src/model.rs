use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    Text(String),
    /// PNG-encoded image bytes.
    Image(Vec<u8>),
}

impl Content {
    pub fn kind(&self) -> &'static str {
        match self {
            Content::Text(_) => "text",
            Content::Image(_) => "image",
        }
    }
    pub fn bytes(&self) -> &[u8] {
        match self {
            Content::Text(s) => s.as_bytes(),
            Content::Image(b) => b,
        }
    }
    pub fn len(&self) -> usize {
        self.bytes().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.kind().as_bytes());
        h.update(self.bytes());
        hex::encode(h.finalize())
    }
}

/// An item about to be recorded.
#[derive(Debug, Clone)]
pub struct NewItem {
    pub content: Content,
    /// Clipboard MIME types offered.
    pub mimes: Vec<String>,
}

impl NewItem {
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            content: Content::Text(s.into()),
            mimes: vec![],
        }
    }
    pub fn image(png: Vec<u8>) -> Self {
        Self {
            content: Content::Image(png),
            mimes: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: i64,
    pub content: Content,
    pub pinned: bool,
    /// Unix milliseconds of last copy.
    pub updated_ms: i64,
}
