use crate::language::Indent;
use crate::text::Text;

const SAMPLE: usize = 500;
const WIDEST: usize = 8;

/// What the file is indented with, which beats what its language prefers.
pub fn detect(text: &Text, fallback: Indent) -> Indent {
    let mut tabs = 0;
    let mut spaces = 0;
    let mut step = 0;

    for line in 0..text.line_count().min(SAMPLE) {
        let content = text.line_str(line);
        match content.chars().next() {
            Some('\t') => tabs += 1,
            Some(' ') => {
                spaces += 1;
                step = gcd(step, content.chars().take_while(|&c| c == ' ').count());
            }
            _ => {}
        }
    }

    if tabs > spaces {
        return Indent::Tabs;
    }
    // A step of one means the file disagrees with itself, so it says nothing.
    match spaces > 0 && (2..=WIDEST).contains(&step) {
        true => Indent::Spaces(step),
        false => fallback,
    }
}

fn gcd(a: usize, b: usize) -> usize {
    match b {
        0 => a,
        _ => gcd(b, a % b),
    }
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
    fn the_step_is_the_common_divisor_of_the_indents() {
        assert_eq!(detected("a\n  b\n    c\n      d\n"), Indent::Spaces(2));
        assert_eq!(detected("a\n    b\n        c\n"), Indent::Spaces(4));
        assert_eq!(detected("a\n   b\n      c\n"), Indent::Spaces(3));
    }

    #[test]
    fn a_file_that_disagrees_with_itself_says_nothing() {
        // A stray one-space line makes every step plausible, so it is ignored.
        assert_eq!(detected("a\n    b\n c\n"), FALLBACK);
    }

    #[test]
    fn a_file_without_indentation_says_nothing() {
        assert_eq!(detected("one\ntwo\nthree\n"), FALLBACK);
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
