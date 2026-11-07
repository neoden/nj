//! The grammars linked into this build, each made once for the process and
//! shared by every buffer: its queries compiled, which is the dear part,
//! dearer than parsing a screenful with them. A grammar is found by the
//! name of its language exactly as `language` spells it; finding that
//! name whatever the case is `language::by_name`'s business. A grammar no
//! file is written in, only pieces of one, has a name of its own:
//! `markdown_inline`, for the text inside a Markdown block.

use std::sync::OnceLock;

use tree_sitter::{Language, Query};

pub struct Grammar {
    pub language: Language,
    pub highlights: Query,
    /// Which pieces of the text another grammar reads, and which one.
    pub injections: Option<Query>,
}

/// A grammar as linked in. The queries are from `queries`, not the
/// crates': those do not agree on which of two patterns naming a node wins.
struct Linked {
    name: &'static str,
    language: fn() -> Language,
    highlights: &'static str,
    injections: Option<&'static str>,
}

/// Every grammar of the build, each by a feature of its own.
const LINKED: &[Linked] = &[
    #[cfg(feature = "json")]
    Linked {
        name: "JSON",
        language: || tree_sitter_json::LANGUAGE.into(),
        highlights: include_str!("../../queries/json/highlights.scm"),
        injections: None,
    },
    // The blocks, with what is inside them left to the next one.
    #[cfg(feature = "markdown")]
    Linked {
        name: "Markdown",
        language: || tree_sitter_md::LANGUAGE.into(),
        highlights: include_str!("../../queries/markdown/highlights.scm"),
        injections: Some(include_str!("../../queries/markdown/injections.scm")),
    },
    #[cfg(feature = "markdown")]
    Linked {
        name: "markdown_inline",
        language: || tree_sitter_md::INLINE_LANGUAGE.into(),
        highlights: include_str!("../../queries/markdown_inline/highlights.scm"),
        injections: None,
    },
    #[cfg(feature = "python")]
    Linked {
        name: "Python",
        language: || tree_sitter_python::LANGUAGE.into(),
        highlights: include_str!("../../queries/python/highlights.scm"),
        injections: None,
    },
    #[cfg(feature = "rust")]
    Linked {
        name: "Rust",
        language: || tree_sitter_rust::LANGUAGE.into(),
        highlights: include_str!("../../queries/rust/highlights.scm"),
        injections: None,
    },
];

/// A grammar linked in, made when first asked for.
struct Entry {
    linked: &'static Linked,
    made: OnceLock<Grammar>,
}

impl Entry {
    fn grammar(&self) -> &Grammar {
        self.made.get_or_init(|| {
            let linked = self.linked;
            let language = (linked.language)();
            // Both are linked in, and tests/highlights loads every query of
            // the build: one that does not fit is a bug of the build.
            let compile = |source| {
                Query::new(&language, source).unwrap_or_else(|error| {
                    panic!("a {} query does not fit its grammar: {error}", linked.name)
                })
            };
            Grammar {
                highlights: compile(linked.highlights),
                injections: linked.injections.map(compile),
                language,
            }
        })
    }
}

/// One entry for each grammar linked in, for the process to share.
fn entries() -> &'static [Entry] {
    static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        LINKED
            .iter()
            .map(|linked| Entry {
                linked,
                made: OnceLock::new(),
            })
            .collect()
    })
}

fn entry(name: &str) -> Option<&'static Entry> {
    entries().iter().find(|entry| entry.linked.name == name)
}

/// The grammar of this name, made on first use.
pub fn by_name(name: &str) -> Option<&'static Grammar> {
    entry(name).map(Entry::grammar)
}

/// Whether the build has the grammar of this name, without making it.
pub fn is_linked(name: &str) -> bool {
    entry(name).is_some()
}
