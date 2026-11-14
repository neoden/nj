use super::language::Indent;
use crate::text::Text;
use crate::text::layout::indentation_of;

const SAMPLE: usize = 500;
const WIDEST: usize = 8;

/// What the file is indented with, which beats what its language prefers.
///
/// Tabs win by count. Among space widths the step is the one that divides
/// the most lines and is itself a depth some line has, so `4, 8, 12` says
/// four rather than two. A width no step accounts for — the one space of
/// a `*` continuation line in a block comment — is simply outvoted, where
/// a common divisor would have let it veto the lot.
pub fn detect(text: &Text, fallback: Indent) -> Indent {
    let mut tabs = 0;
    let mut widths = Vec::new();

    for line in 0..text.line_count().min(SAMPLE) {
        let lead = indentation_of(text, line);
        match lead.as_bytes().first() {
            Some(b'\t') => tabs += 1,
            Some(b' ') => widths.push(lead.bytes().take_while(|&c| c == b' ').count()),
            _ => {}
        }
    }

    if tabs > widths.len() {
        return Indent::Tabs;
    }

    let mut steps: Vec<usize> = widths
        .iter()
        .copied()
        .filter(|width| (2..=WIDEST).contains(width))
        .collect();
    steps.sort_unstable();
    steps.dedup();

    let explained = |step: usize| widths.iter().filter(|&&width| width % step == 0).count();
    steps
        .into_iter()
        .max_by_key(|&step| (explained(step), std::cmp::Reverse(step)))
        .map_or(fallback, Indent::Spaces)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FALLBACK: Indent = Indent::Spaces(4);

    fn detected(content: &str) -> Indent {
        detect(&Text::from_str(content), FALLBACK)
    }

    #[test]
    fn tabs_are_recognized() {
        assert_eq!(detected("fn a() {\n\tone();\n\ttwo();\n}"), Indent::Tabs);
    }

    #[test]
    fn the_step_is_the_depth_that_explains_the_most_lines() {
        assert_eq!(detected("a\n  b\n    c\n      d\n"), Indent::Spaces(2));
        assert_eq!(detected("a\n    b\n        c\n"), Indent::Spaces(4));
        assert_eq!(detected("a\n   b\n      c\n"), Indent::Spaces(3));
        // Two would explain every line too, but no line is two deep.
        assert_eq!(
            detected("a\n    b\n        c\n            d\n"),
            Indent::Spaces(4)
        );
    }

    #[test]
    fn a_stray_line_is_outvoted() {
        assert_eq!(detected("a\n    b\n c\n"), Indent::Spaces(4));
    }

    /// The `*` lines of a block comment are one space in, which is not an
    /// indentation level and must not decide anything.
    #[test]
    fn block_comment_continuations_do_not_decide() {
        let source = "/**\n * doc\n */\nfunction f() {\n  return 1;\n}\n";
        assert_eq!(detected(source), Indent::Spaces(2));
    }

    #[test]
    fn a_file_without_indentation_says_nothing() {
        assert_eq!(detected("one\ntwo\nthree\n"), FALLBACK);
        assert_eq!(detected("a\n b\n"), FALLBACK);
    }

    #[test]
    fn whichever_is_more_common_wins() {
        assert_eq!(detected("\ta\n\tb\n    c\n"), Indent::Tabs);
        assert_eq!(detected("\ta\n    b\n    c\n"), Indent::Spaces(4));
    }

    #[test]
    fn alignment_wider_than_a_level_is_not_a_step() {
        assert_eq!(detected("a\n            b\n"), FALLBACK);
    }
}
