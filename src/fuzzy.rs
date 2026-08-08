const START_BONUS: i32 = 8;
const BOUNDARY_BONUS: i32 = 6;
const CASE_BONUS: i32 = 1;
/// Charged for every letter skipped between two matched ones, which is what
/// keeps a scattered match from beating a tight one.
const GAP_PENALTY: i32 = 1;

/// Scores a candidate against a query, or reports that the query is not a
/// subsequence of it. Higher is better; matching is case-insensitive but
/// matching the case is worth a little more.
pub fn score(candidate: &str, query: &str) -> Option<i32> {
    let mut wanted = query.chars().peekable();
    let mut score = 0;
    let mut streak = 0;
    let mut previous = None;

    for (index, actual) in candidate.chars().enumerate() {
        let Some(&next) = wanted.peek() else {
            break;
        };
        if !same_letter(actual, next) {
            if streak > 0 || score > 0 {
                score -= GAP_PENALTY;
            }
            streak = 0;
            previous = Some(actual);
            continue;
        }

        wanted.next();
        // A run of adjacent matches is what makes an abbreviation feel right,
        // so each further letter of the run is worth more than the last.
        streak += 1;
        score += 1 + streak;
        if index == 0 {
            score += START_BONUS;
        } else if starts_a_word(previous, actual) {
            score += BOUNDARY_BONUS;
        }
        if actual == next {
            score += CASE_BONUS;
        }
        previous = Some(actual);
    }

    // Among equals, the shorter candidate is the better guess.
    let length = candidate.chars().count() as i32;
    wanted.peek().is_none().then(|| score - length / 4)
}

fn same_letter(actual: char, wanted: char) -> bool {
    actual == wanted || actual.to_lowercase().eq(wanted.to_lowercase())
}

fn starts_a_word(previous: Option<char>, actual: char) -> bool {
    match previous {
        None => true,
        Some(previous) => {
            matches!(previous, '/' | '\\' | '_' | '-' | '.' | ' ' | ':')
                || (previous.is_lowercase() && actual.is_uppercase())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_query_must_be_a_subsequence() {
        assert!(score("src/main.rs", "smr").is_some());
        assert!(score("src/main.rs", "rsm").is_none());
        assert!(score("short", "longer").is_none());
    }

    #[test]
    fn an_empty_query_matches_everything() {
        assert!(score("anything", "").is_some());
    }

    #[test]
    fn case_is_ignored_but_matching_it_scores_higher() {
        let exact = score("Cargo", "C").unwrap();
        let loose = score("Cargo", "c").unwrap();
        assert!(exact > loose);
    }

    #[test]
    fn word_starts_beat_letters_buried_inside() {
        let boundary = score("src/main.rs", "m").unwrap();
        let buried = score("summary.rs", "m").unwrap();
        assert!(boundary > buried);
    }

    #[test]
    fn adjacent_letters_beat_scattered_ones() {
        let adjacent = score("filename", "file").unwrap();
        let scattered = score("fixed_lane_end", "file").unwrap();
        assert!(adjacent > scattered);
    }

    #[test]
    fn the_shorter_of_two_matches_wins() {
        let short = score("main.rs", "main").unwrap();
        let long = score("main_helper_module.rs", "main").unwrap();
        assert!(short > long);
    }

    #[test]
    fn camel_case_counts_as_a_word_start() {
        let camel = score("parseValue", "v").unwrap();
        let buried = score("parsevalue", "v").unwrap();
        assert!(camel > buried);
    }
}
