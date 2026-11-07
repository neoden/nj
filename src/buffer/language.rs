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
    /// For the few languages known by name rather than by suffix. A `*`
    /// stands for any run of characters, `Dockerfile.*`; a name without
    /// one is that name alone.
    pub filenames: &'static [&'static str],
    /// What one level of indentation is made of in a fresh file.
    pub indent: Indent,
    pub tab_width: usize,
    /// None for languages that only have block comments, until we do those.
    pub line_comment: Option<&'static str>,
}

pub const PLAIN: Language = Language {
    name: "Text",
    extensions: &[],
    filenames: &[],
    indent: Indent::Spaces(4),
    tab_width: 4,
    line_comment: None,
};

pub const LANGUAGES: &[Language] = &[
    Language {
        name: "Rust",
        extensions: &["rs"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
        line_comment: Some("//"),
    },
    Language {
        name: "Python",
        extensions: &["py", "pyi"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
        line_comment: Some("#"),
    },
    Language {
        name: "Java",
        extensions: &["java"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
        line_comment: Some("//"),
    },
    Language {
        name: "JavaScript",
        extensions: &["js", "mjs", "cjs", "jsx"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
        line_comment: Some("//"),
    },
    Language {
        name: "TypeScript",
        extensions: &["ts", "mts", "cts"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
        line_comment: Some("//"),
    },
    Language {
        // TypeScript with JSX, which a grammar of its own reads: `<T>x` is
        // a cast in TypeScript and a tag here.
        name: "TSX",
        extensions: &["tsx"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
        line_comment: Some("//"),
    },
    Language {
        name: "HTML",
        extensions: &["html", "htm"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
        line_comment: None,
    },
    Language {
        name: "CSS",
        extensions: &["css"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
        line_comment: None,
    },
    Language {
        name: "SCSS",
        extensions: &["scss"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
        line_comment: Some("//"),
    },
    Language {
        name: "JSON",
        extensions: &["json"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
        line_comment: None,
    },
    Language {
        // Sections of `key = value`: config files of many tools, systemd's
        // units, git's own config. Not a desktop entry, though it is INI:
        // its `Name[ru]=` keys are beyond the grammar, and each spoils
        // the line after it.
        name: "INI",
        extensions: &["ini", "cfg", "service", "socket", "timer", "mount", "target"],
        filenames: &[
            ".gitconfig",
            ".gitmodules",
            ".editorconfig",
            ".npmrc",
            ".flake8",
            ".pylintrc",
            ".coveragerc",
        ],
        indent: Indent::Spaces(4),
        tab_width: 4,
        line_comment: Some("#"),
    },
    Language {
        name: "SQL",
        extensions: &["sql"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
        line_comment: Some("--"),
    },
    Language {
        name: "TOML",
        extensions: &["toml"],
        filenames: &["Cargo.lock"],
        indent: Indent::Spaces(2),
        tab_width: 2,
        line_comment: Some("#"),
    },
    Language {
        // Tabs are illegal in YAML, so this one can never be Indent::Tabs.
        name: "YAML",
        extensions: &["yaml", "yml"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
        line_comment: Some("#"),
    },
    Language {
        name: "Markdown",
        extensions: &["md", "markdown"],
        filenames: &[],
        indent: Indent::Spaces(2),
        tab_width: 2,
        line_comment: None,
    },
    Language {
        // What `git diff` and `git format-patch` write, and the hunk `git
        // add -p` hands over to edit, `addp-hunk-edit.diff`.
        name: "Diff",
        extensions: &["diff", "patch"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
        line_comment: None,
    },
    Language {
        // Podman calls the same file a Containerfile.
        name: "Dockerfile",
        extensions: &["dockerfile", "containerfile"],
        filenames: &[
            "Dockerfile",
            "Containerfile",
            "Dockerfile.*",
            "*.Dockerfile",
            "Containerfile.*",
        ],
        indent: Indent::Spaces(4),
        tab_width: 4,
        line_comment: Some("#"),
    },
    Language {
        // The messages git asks for in the editor, by the files it writes
        // them to under `.git`.
        name: "Git commit",
        extensions: &[],
        filenames: &[
            "COMMIT_EDITMSG",
            "MERGE_MSG",
            "SQUASH_MSG",
            "TAG_EDITMSG",
            "NOTES_EDITMSG",
        ],
        indent: Indent::Spaces(4),
        tab_width: 4,
        line_comment: Some("#"),
    },
    Language {
        // The list `git rebase -i` hands over to edit.
        name: "Git rebase",
        extensions: &[],
        filenames: &["git-rebase-todo"],
        indent: Indent::Spaces(4),
        tab_width: 4,
        line_comment: Some("#"),
    },
    Language {
        name: "Go",
        extensions: &["go"],
        filenames: &[],
        indent: Indent::Tabs,
        tab_width: 4,
        line_comment: Some("//"),
    },
    Language {
        name: "C",
        extensions: &["c", "h"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
        line_comment: Some("//"),
    },
    Language {
        name: "C#",
        extensions: &["cs", "csx"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
        line_comment: Some("//"),
    },
    Language {
        name: "C++",
        extensions: &["cc", "cpp", "cxx", "hpp", "hh"],
        filenames: &[],
        indent: Indent::Spaces(4),
        tab_width: 4,
        line_comment: Some("//"),
    },
    Language {
        name: "Shell",
        // An environment file, `.env`, is Shell too: `KEY=value` lines a
        // shell sources, with `export`, quotes and `${OTHER}` in them.
        extensions: &["sh", "bash", "zsh", "env"],
        filenames: &[
            ".env",
            ".env.*",
            ".profile",
            ".bashrc",
            ".bash_profile",
            ".bash_login",
            ".bash_logout",
            ".bash_aliases",
            ".zshrc",
            ".zshenv",
            ".zprofile",
            ".zlogin",
            ".zlogout",
        ],
        indent: Indent::Spaces(2),
        tab_width: 2,
        line_comment: Some("#"),
    },
    Language {
        // Make demands a literal tab before every recipe line.
        name: "Makefile",
        extensions: &["mk"],
        filenames: &["Makefile", "makefile", "GNUmakefile"],
        indent: Indent::Tabs,
        tab_width: 8,
        line_comment: Some("#"),
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

/// Whatever the case: `json` on a command line is JSON.
pub fn by_name(name: &str) -> Option<&'static Language> {
    LANGUAGES
        .iter()
        .chain([&PLAIN])
        .find(|language| language.name.eq_ignore_ascii_case(name))
}

/// A name spelt out wins over a pattern, whichever language has it: a
/// `Dockerfile.*` does not take a name another language lists whole.
fn by_filename(name: &str) -> Option<&'static Language> {
    let exact = LANGUAGES
        .iter()
        .find(|language| language.filenames.contains(&name));
    exact.or_else(|| {
        LANGUAGES.iter().find(|language| {
            language
                .filenames
                .iter()
                .any(|pattern| pattern.contains('*') && matches(pattern, name))
        })
    })
}

/// Whether `name` fits `pattern`, whose `*`s each stand for any run of
/// characters, none too.
fn matches(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    // The first part starts the name and the last ends it; those between
    // come in order, each as early as it can.
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let mut parts = parts.peekable();
    while let Some(part) = parts.next() {
        if parts.peek().is_none() {
            return rest.ends_with(part);
        }
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    // No `*` at all: the name is the pattern.
    rest.is_empty()
}

/// The language whose files end in `.extension`, which is also how a
/// writer often names one: `py` after the ```` ``` ```` of a code block.
pub fn by_extension(extension: &str) -> Option<&'static Language> {
    LANGUAGES
        .iter()
        .find(|language| language.extensions.contains(&extension))
}

/// A language as a person names it, typed on a command line or after a
/// code block's ```` ``` ````: by its name or by its files' extension,
/// whatever the case. `cs` is C#, which is easier to type than `c#` in a
/// shell where `#` means something.
pub fn named(word: &str) -> Option<&'static Language> {
    by_name(word).or_else(|| by_extension(&word.to_ascii_lowercase()))
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
        assert_eq!(detect_name("site.css"), "CSS");
        assert_eq!(detect_name("site.scss"), "SCSS");
        assert_eq!(detect_name("app.ts"), "TypeScript");
        assert_eq!(detect_name("app.tsx"), "TSX");
    }

    #[test]
    fn some_languages_are_known_by_name() {
        assert_eq!(detect_name("Makefile"), "Makefile");
        assert_eq!(detect_name("project/GNUmakefile"), "Makefile");
        assert_eq!(detect_name("/home/u/.zshenv"), "Shell");
        assert_eq!(detect_name(".bash_aliases"), "Shell");
        assert_eq!(detect_name("repo/.git/COMMIT_EDITMSG"), "Git commit");
        assert_eq!(detect_name("repo/.git/rebase-merge/git-rebase-todo"), "Git rebase");
        assert_eq!(detect_name("app/Dockerfile"), "Dockerfile");
        assert_eq!(detect_name("app/Containerfile"), "Dockerfile");
    }

    #[test]
    fn a_name_can_be_a_pattern() {
        assert_eq!(detect_name("Dockerfile.dev"), "Dockerfile");
        assert_eq!(detect_name("deploy/prod.Dockerfile"), "Dockerfile");
        // A `*` stands for no characters too.
        assert_eq!(detect_name("Dockerfile."), "Dockerfile");
        assert_eq!(detect_name("NotADockerfile"), "Text");
        assert_eq!(detect_name("app/.env"), "Shell");
        assert_eq!(detect_name("app/.env.production"), "Shell");
        assert_eq!(detect_name("deploy/prod.env"), "Shell");
        assert_eq!(detect_name("/home/u/.gitconfig"), "INI");
        assert_eq!(detect_name("app.service"), "INI");
    }

    #[test]
    fn a_star_stands_for_any_run_of_characters() {
        assert!(matches("Dockerfile.*", "Dockerfile.dev"));
        assert!(matches("Dockerfile.*", "Dockerfile."));
        assert!(!matches("Dockerfile.*", "Dockerfile"));
        assert!(matches("*.Dockerfile", "prod.Dockerfile"));
        assert!(!matches("*.Dockerfile", "prod.dockerfile"));
        assert!(matches("a*b*c", "a-b-c"));
        assert!(matches("a*b*c", "abc"));
        assert!(!matches("a*b*c", "acb"));
        // The start and the end may not share a character.
        assert!(!matches("a*a", "a"));
        assert!(matches("Makefile", "Makefile"));
        assert!(!matches("Makefile", "Makefile.am"));
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
        // Found whatever the case, so two names that differ only in case
        // would find the first of them for both, and fail here.
        for language in LANGUAGES {
            assert_eq!(by_name(language.name), Some(language));
        }
        assert!(by_name("Text").is_some());
    }

    #[test]
    fn a_name_is_found_whatever_its_case() {
        assert_eq!(by_name("json").unwrap().name, "JSON");
        assert_eq!(by_name("rust").unwrap().name, "Rust");
        assert_eq!(by_name("c++").unwrap().name, "C++");
        assert!(by_name("rst").is_none());
    }

    #[test]
    fn a_person_names_a_language_by_its_name_or_its_extension() {
        assert_eq!(named("C#").unwrap().name, "C#");
        assert_eq!(named("cs").unwrap().name, "C#");
        assert_eq!(named("CS").unwrap().name, "C#");
        assert_eq!(named("py").unwrap().name, "Python");
        assert_eq!(named("rust").unwrap().name, "Rust");
        // A name wins over an extension: `c` is the language C.
        assert_eq!(named("c").unwrap().name, "C");
        assert!(named("rst").is_none());
    }
}
