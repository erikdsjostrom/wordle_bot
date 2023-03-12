use crate::error::{Error, Result};
use nom::sequence::{preceded, separated_pair};
use nom::{bytes::complete::tag, IResult};
use nom::{character, combinator};

// Assert that line starts with Wordle, but drop the parsed value
fn parse_wordle_str(s: &str) -> IResult<&str, ()> {
    combinator::map(tag("Wordle "), drop)(s)
}

// Parse number terminated by a space
fn parse_day(s: &str) -> IResult<&str, i64> {
    character::complete::i64(s)
}

fn parse_score(s: &str) -> IResult<&str, i64> {
    combinator::map(
        nom::character::complete::one_of("X123456"),
        |score| match score {
            'X' => 0, // Ok, since there's no 0 guess score
            '1' => 1,
            '2' => 2,
            '3' => 3,
            '4' => 4,
            '5' => 5,
            '6' => 6,
            _ => unreachable!(),
        }
    )(s)
}

fn parse_day_and_score(s: &str) -> IResult<&str, (i64, i64)> {
    separated_pair(parse_day, tag(" "), parse_score)(s)
}

fn parse_wordle(s: &str) -> IResult<&str, (i64, i64)> {
    preceded(parse_wordle_str, parse_day_and_score)(s)
}

/// Parses a wordle msg "Wordle day score/6" into (day, score)
pub(crate) fn parse_msg(s: &str) -> Result<(i64, i64)> {
    parse_wordle(s)
        .map(|(_, res)| res)
        .map_err(|err| Error::Message(format!("Parse error: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wordle_str() {
        assert_eq!(parse_wordle_str("Wordle "), Ok(("", ())));
        assert!(parse_wordle_str("wordle").is_err());
    }

    #[test]
    fn test_parse_day() {
        assert_eq!(parse_day("547 bla"), Ok((" bla", 547)));
        assert!(parse_day("blabla").is_err());
        assert!(parse_day("").is_err());
    }

    #[test]
    fn test_parse_score_char() {
        for x in ['X', '1', '2', '3', '4', '5', '6'] {
            assert!(parse_score(&format!("{x}/6")).is_ok());
        }
        assert!(parse_score("7/6").is_err())
    }

    #[test]
    fn test_parse_day_and_score() {
        assert_eq!(parse_day_and_score("547 3/6"), Ok(("/6", (547, 3))));

        assert!(parse_day_and_score("547 7/6").is_err());
    }

    #[test]
    fn test_parse_wordle() {
        assert_eq!(parse_wordle("Wordle 547 3/6"), Ok(("/6", (547, 3))));
        assert!(parse_wordle("Wordle 547 7/6").is_err());
        assert!(parse_wordle("wordle 547 7/6").is_err());
        assert!(parse_wordle("Wordle foo 7/6").is_err());
    }
}
