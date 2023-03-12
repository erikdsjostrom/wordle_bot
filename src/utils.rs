use std::cmp::Ordering;
use std::time::Duration;
use std::time::UNIX_EPOCH;

use chrono::DateTime;
use chrono::Datelike;
use chrono::Utc;

pub fn cup_number_from_unixtime(unixtime: i64) -> String {
    // Creates a new SystemTime from the specified number of whole seconds
    let d = UNIX_EPOCH + Duration::from_secs(unixtime as u64);
    // Create DateTime from SystemTime
    let datetime = DateTime::<Utc>::from(d);
    datetime.year().to_string() + &datetime.month().to_string()
}

pub fn current_cup_number() -> String {
    let current_date = chrono::Utc::now().date_naive();
    current_date.year().to_string() + &current_date.month().to_string()
}

pub fn current_cup_number_cute_format() -> String {
    let current_date = chrono::Utc::now().date_naive();
    format!("{}/{}", current_date.year(), current_date.month())
}

// Upserts a new potential high-score into a sorted list of high-scores of length three
pub fn recalcualate_high_scores(high_scores: [Option<i64>; 3], score: i64) -> [Option<i64>; 3] {
    match (high_scores[0], high_scores[1], high_scores[2]) {
        (None, None, None) => [Some(score), None, None],
        (Some(gold), None, None) => match gold.cmp(&score) {
            Ordering::Greater => [Some(score), Some(gold), None],
            Ordering::Equal => [Some(gold), None, None],
            Ordering::Less => [Some(gold), Some(score), None],
        },
        (Some(gold), Some(silver), None) => match (gold.cmp(&score), silver.cmp(&score)) {
            (Ordering::Greater, _) => [Some(score), Some(gold), Some(silver)],
            (Ordering::Equal, _) => [Some(gold), Some(silver), None],
            (Ordering::Less, Ordering::Greater) => [Some(gold), Some(score), Some(silver)],
            (Ordering::Less, Ordering::Equal) => [Some(gold), Some(silver), None],
            (Ordering::Less, Ordering::Less) => [Some(gold), Some(silver), Some(score)],
        },
        (Some(gold), Some(silver), Some(bronze)) => {
            match (gold.cmp(&score), silver.cmp(&score), bronze.cmp(&score)) {
                (Ordering::Greater, _, _) => [Some(score), Some(gold), Some(silver)],
                (Ordering::Equal, _, _) => [Some(gold), Some(silver), Some(bronze)],
                (Ordering::Less, Ordering::Equal, _) => [Some(gold), Some(silver), Some(bronze)],
                (Ordering::Less, Ordering::Greater, _) => [Some(gold), Some(score), Some(silver)],
                (Ordering::Less, Ordering::Less, Ordering::Less) => {
                    [Some(gold), Some(silver), Some(bronze)]
                }
                (Ordering::Less, Ordering::Less, Ordering::Equal) => {
                    [Some(gold), Some(silver), Some(bronze)]
                }
                (Ordering::Less, Ordering::Less, Ordering::Greater) => {
                    [Some(gold), Some(silver), Some(score)]
                }
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_recalculate_scores() {
        assert_eq!(
            recalcualate_high_scores([None, None, None], 1),
            [Some(1), None, None]
        );

        assert_eq!(
            recalcualate_high_scores([Some(1), None, None], 1),
            [Some(1), None, None]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), None, None], 1),
            [Some(1), Some(2), None]
        );

        assert_eq!(
            recalcualate_high_scores([Some(1), None, None], 2),
            [Some(1), Some(2), None]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), None], 6),
            [Some(2), Some(4), Some(6)]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), None], 1),
            [Some(1), Some(2), Some(4)]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), None], 3),
            [Some(2), Some(3), Some(4)]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), None], 2),
            [Some(2), Some(4), None]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), None], 4),
            [Some(2), Some(4), None]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), Some(6)], 5),
            [Some(2), Some(4), Some(5)]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), Some(6)], 6),
            [Some(2), Some(4), Some(6)]
        );

        assert_eq!(
            recalcualate_high_scores([Some(2), Some(4), Some(6)], 7),
            [Some(2), Some(4), Some(6)]
        );
    }
}
