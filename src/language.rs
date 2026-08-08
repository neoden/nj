use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indent {
    Tabs,
    Spaces(usize),
}

impl Indent {
    /// What one level looks like in the text.
    pub fn unit(self) -> String {
        match self {
            Indent::Tabs => "\t".to_owned(),
            Indent::Spaces(width) => " ".repeat(width),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
    /// For the few languages known by name rather than by suffix.
    pub filenames: &'static [&'static str],
    /// What one level of indentation is made of in a fresh file.
    pub indent: Indent,
    pub tab_width: usize,
}

pub const PLAIN: Language = Language {
    name: "Text",
    extensions: &[],
    filenames: &[],
    indent: Indent::Spaces(4),
    tab_width: 4,
};

pub const LANGUAGES: &[Language] = &[
    Language {
        name: "Rust",
        extensions: &["rs"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
    },
    Language {
        name: "Python",
        extensions: &["py", "pyi"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
    },
    Language {
        name: "JavaScript",
        extensions: &["js", "mjs", "cjs", "jsx"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "TypeScript",
        extensions: &["ts", "tsx"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "HTML",
        extensions: &["html", "htm"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "CSS",
        extensions: &["css", "scss"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "JSON",
        extensions: &["json"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "TOML",
        extensions: &["toml"],
        filenames: &["Cargo.lock"],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        // Tabs are illegal in YAML, so this one can never be Indent::Tabs.
        name: "YAML",
        extensions: &["yaml", "yml"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "Markdown",
        extensions: &["md", "markdown"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "Go",
        extensions: &["go"],
        filenames: &[],
        indent: Indent::Tabs,
        tab_width: 4,
    },
    Language {
        name: "C",
        extensions: &["c", "h"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
    },
    Language {
        name: "C++",
        extensions: &["cc", "cpp", "cxx", "hpp", "hh"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
    },
    Language {
        name: "Shell",
        extensions: &["sh", "bash", "zsh"],
        filenames: &[".bashrc", ".zshrc", ".profile"],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        // Make demands a literal tab before every recipe line.
        name: "Makefile",
        extensions: &["mk"],
        filenames: &["Makefile", "makefile", "GNUmakefile"],
        indent: Indent::Tabs,
        tab_width: 8,
    },
];

pub fn detect(path: &Path) -> &'static Language {
    let name = path.file_name().and_then(|name| name.to_str());
    if let Some(language) = name.and_then(by_filename) {
        return language;
    }

    path.extension()
        .and_then(|extension| extension.to_str())
        .and_then(by_extension)
        .unwrap_or(&PLAIN)
}

pub fn by_name(name: &str) -> Option<&'static Language> {
    LANGUAGES
        .iter()
        .chain([&PLAIN])
        .find(|language| language.name == name)
}

fn by_filename(name: &str) -> Option<&'static Language> {
    LANGUAGES
        .iter()
        .find(|language| language.filenames.contains(&name))
}

fn by_extension(extension: &str) -> Option<&'static Language> {
    LANGUAGES
        .iter()
        .find(|language| language.extensions.contains(&extension))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detect_name(path: &str) -> &'static str {
        detect(Path::new(path)).name
    }

    #[test]
    fn extensions_decide_the_language() {
        assert_eq!(detect_name("src/main.rs"), "Rust");
        assert_eq!(detect_name("/tmp/a/b/script.py"), "Python");
        assert_eq!(detect_name("Cargo.toml"), "TOML");
    }

    #[test]
    fn some_languages_are_known_by_name() {
        assert_eq!(detect_name("Makefile"), "Makefile");
        assert_eq!(detect_name("project/GNUmakefile"), "Makefile");
    }

    #[test]
    fn anything_unknown_is_plain_text() {
        assert_eq!(detect_name("notes"), "Text");
        assert_eq!(detect_name("archive.tar.gz"), "Text");
    }

    #[test]
    fn languages_carry_their_own_indentation() {
        assert_eq!(by_name("Go").unwrap().indent, Indent::Tabs);
        assert_eq!(by_name("Makefile").unwrap().tab_width, 8);
        assert_eq!(by_name("JavaScript").unwrap().indent, Indent::Spaces(2));
    }

    #[test]
    fn every_language_is_reachable_by_name() {
        for language in LANGUAGES {
            assert_eq!(by_name(language.name), Some(language));
        }
        assert!(by_name("Text").is_some());
    }
}
