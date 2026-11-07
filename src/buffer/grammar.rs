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
    /// One query, or the query of the language this one grows from and
    /// then its own: C's and then C++'s, whose patterns win.
    highlights: &'static [&'static str],
    injections: Option<&'static str>,
}

/// Every grammar of the build, each by a feature of its own.
const LINKED: &[Linked] = &[
    #[cfg(feature = "c")]
    Linked {
        name: "C",
        language: || tree_sitter_c::LANGUAGE.into(),
        highlights: &[include_str!("../../queries/c/highlights.scm")],
        injections: Some(include_str!("../../queries/c/injections.scm")),
    },
    // C's patterns hold in C++ too, which has every node they name. The
    // injections are its own: a macro's body here is C++.
    #[cfg(feature = "cpp")]
    Linked {
        name: "C++",
        language: || tree_sitter_cpp::LANGUAGE.into(),
        highlights: &[
            include_str!("../../queries/c/highlights.scm"),
            include_str!("../../queries/cpp/highlights.scm"),
        ],
        injections: Some(include_str!("../../queries/cpp/injections.scm")),
    },
    #[cfg(feature = "css")]
    Linked {
        name: "CSS",
        language: || tree_sitter_css::LANGUAGE.into(),
        highlights: &[
            include_str!("../../queries/css/highlights.scm"),
            include_str!("../../queries/css/css-only.scm"),
        ],
        injections: None,
    },
    #[cfg(feature = "go")]
    Linked {
        name: "Go",
        language: || tree_sitter_go::LANGUAGE.into(),
        highlights: &[include_str!("../../queries/go/highlights.scm")],
        injections: None,
    },
    #[cfg(feature = "json")]
    Linked {
        name: "JSON",
        language: || tree_sitter_json::LANGUAGE.into(),
        highlights: &[include_str!("../../queries/json/highlights.scm")],
        injections: None,
    },
    #[cfg(feature = "makefile")]
    Linked {
        name: "Makefile",
        language: || tree_sitter_make::LANGUAGE.into(),
        highlights: &[include_str!("../../queries/makefile/highlights.scm")],
        injections: Some(include_str!("../../queries/makefile/injections.scm")),
    },
    // The blocks, with what is inside them left to the next one.
    #[cfg(feature = "markdown")]
    Linked {
        name: "Markdown",
        language: || tree_sitter_md::LANGUAGE.into(),
        highlights: &[include_str!("../../queries/markdown/highlights.scm")],
        injections: Some(include_str!("../../queries/markdown/injections.scm")),
    },
    #[cfg(feature = "markdown")]
    Linked {
        name: "markdown_inline",
        language: || tree_sitter_md::INLINE_LANGUAGE.into(),
        highlights: &[include_str!("../../queries/markdown/inline-highlights.scm")],
        injections: None,
    },
    #[cfg(feature = "python")]
    Linked {
        name: "Python",
        language: || tree_sitter_python::LANGUAGE.into(),
        highlights: &[include_str!("../../queries/python/highlights.scm")],
        injections: None,
    },
    #[cfg(feature = "rust")]
    Linked {
        name: "Rust",
        language: || tree_sitter_rust::LANGUAGE.into(),
        highlights: &[include_str!("../../queries/rust/highlights.scm")],
        injections: None,
    },
    // CSS's patterns hold in SCSS too, but for the few in `css-only.scm`,
    // whose nodes its older grammar does not have.
    #[cfg(feature = "scss")]
    Linked {
        name: "SCSS",
        language: tree_sitter_scss::language,
        highlights: &[
            include_str!("../../queries/css/highlights.scm"),
            include_str!("../../queries/scss/highlights.scm"),
        ],
        injections: None,
    },
    #[cfg(feature = "shell")]
    Linked {
        name: "Shell",
        language: || tree_sitter_bash::LANGUAGE.into(),
        highlights: &[include_str!("../../queries/shell/highlights.scm")],
        injections: None,
    },
    #[cfg(feature = "toml")]
    Linked {
        name: "TOML",
        language: || tree_sitter_toml_ng::LANGUAGE.into(),
        highlights: &[include_str!("../../queries/toml/highlights.scm")],
        injections: None,
    },
    #[cfg(feature = "yaml")]
    Linked {
        name: "YAML",
        language: || tree_sitter_yaml::LANGUAGE.into(),
        highlights: &[include_str!("../../queries/yaml/highlights.scm")],
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
            let compile = |source: &str| {
                Query::new(&language, source).unwrap_or_else(|error| {
                    panic!("a {} query does not fit its grammar: {error}", linked.name)
                })
            };
            Grammar {
                highlights: compile(&linked.highlights.concat()),
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
