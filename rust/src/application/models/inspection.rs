//! The shape of a flagged page, as the routes and templates read it.
//!
//! The dict side of `core::models::page_inspection`, converted by `core::db` on
//! the way out, like `GameDict` for a play-through.

/// One remark on a flagged page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommentDict {
    pub text: String,
    pub created_at: String,
}

/// A flagged page and everything said about it, newest remark last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionDict {
    pub id: i64,
    pub book: String,
    pub path: String,
    pub page_title: Option<String>,
    pub status: String,
    pub comments: Vec<CommentDict>,
    pub created_at: String,
    pub updated_at: String,
}

impl InspectionDict {
    /// The last remark made, which is the one the list shows.
    pub fn last_comment(&self) -> Option<&CommentDict> {
        self.comments.last()
    }

    /// The last remark, or a blank one for a file that somehow holds none.
    ///
    /// A file is only ever opened by a remark, so the blank one is unreachable
    /// through the routes; the template asks for a value rather than a choice.
    pub fn last_comment_or_none(&self) -> CommentDict {
        self.comments.last().cloned().unwrap_or_else(|| CommentDict {
            text: String::new(),
            created_at: String::new(),
        })
    }

    /// What the page's tab said, falling back to the address itself.
    pub fn title_or_path(&self) -> &str {
        self.page_title
            .as_deref()
            .filter(|title| !title.is_empty())
            .unwrap_or(&self.path)
    }
}
