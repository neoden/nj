use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indent {
    Tabs,
    Spaces(usize),
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
    name: "text",
    extensions: &[],
    filenames: &[],
    indent: Indent::Spaces(4),
    tab_width: 4,
};

pub const LANGUAGES: &[Language] = &[
    Language {
        name: "rust",
        extensions: &["rs"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
    },
    Language {
        name: "python",
        extensions: &["py", "pyi"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
    },
    Language {
        name: "javascript",
        extensions: &["js", "mjs", "cjs", "jsx"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "typescript",
        extensions: &["ts", "tsx"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "html",
        extensions: &["html", "htm"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "css",
        extensions: &["css", "scss"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "json",
        extensions: &["json"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "toml",
        extensions: &["toml"],
        filenames: &["Cargo.lock"],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        // Tabs are illegal in YAML, so this one can never be Indent::Tabs.
        name: "yaml",
        extensions: &["yaml", "yml"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "markdown",
        extensions: &["md", "markdown"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        name: "go",
        extensions: &["go"],
        filenames: &[],
        indent: Indent::Tabs,
        tab_width: 4,
    },
    Language {
        name: "c",
        extensions: &["c", "h"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
    },
    Language {
        name: "c++",
        extensions: &["cc", "cpp", "cxx", "hpp", "hh"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
    },
    Language {
        name: "shell",
        extensions: &["sh", "bash", "zsh"],
        filenames: &[".bashrc", ".zshrc", ".profile"],
        indent: Indent::Spaces(2),
        tab_width: 2,
    },
    Language {
        // Make demands a literal tab before every recipe line.
        name: "makefile",
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
        assert_eq!(detect_name("src/main.rs"), "rust");
        assert_eq!(detect_name("/tmp/a/b/script.py"), "python");
        assert_eq!(detect_name("Cargo.toml"), "toml");
    }

    #[test]
    fn some_languages_are_known_by_name() {
        assert_eq!(detect_name("Makefile"), "makefile");
        assert_eq!(detect_name("project/GNUmakefile"), "makefile");
    }

    #[test]
    fn anything_unknown_is_plain_text() {
        assert_eq!(detect_name("notes"), "text");
        assert_eq!(detect_name("archive.tar.gz"), "text");
    }

    #[test]
    fn languages_carry_their_own_indentation() {
        assert_eq!(by_name("go").unwrap().indent, Indent::Tabs);
        assert_eq!(by_name("makefile").unwrap().tab_width, 8);
        assert_eq!(by_name("javascript").unwrap().indent, Indent::Spaces(2));
    }

    #[test]
    fn every_language_is_reachable_by_name() {
        for language in LANGUAGES {
            assert_eq!(by_name(language.name), Some(language));
        }
        assert!(by_name("text").is_some());
    }
}
