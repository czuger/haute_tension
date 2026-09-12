//! Tests for the two dice everything else reads.
//!
//! Ported from `tests/test_core_dice.py`.

#[path = "common/mod.rs"]
mod common;

use common::{fixed, seeded};
use haute_tension::core::dice::{roll_2d6, Roll, DIE_FACES};
use std::collections::HashSet;

/// What the rules read off a throw.
mod roll {
    use super::*;

    #[test]
    fn the_total_is_the_sum_of_the_faces() {
        assert_eq!(Roll::new([4, 5]).total(), 9);
    }

    #[test]
    fn a_double_six_is_recognised() {
        assert!(Roll::new([6, 6]).is_double_six());
        assert!(!Roll::new([6, 5]).is_double_six());
    }

    #[test]
    fn a_double_one_is_recognised() {
        assert!(Roll::new([1, 1]).is_double_one());
        assert!(!Roll::new([1, 2]).is_double_one());
    }

    #[test]
    fn a_double_six_is_not_a_double_one() {
        assert!(!Roll::new([6, 6]).is_double_one());
    }

    /// A total of 8 could be 4+4 or 6+2, and only one of them is a double.
    #[test]
    fn the_faces_are_kept_apart() {
        assert_eq!(Roll::new([4, 4]).total(), Roll::new([6, 2]).total());
        assert_ne!(Roll::new([4, 4]).dice(), Roll::new([6, 2]).dice());
    }

    #[test]
    fn it_prints_its_working() {
        assert_eq!(Roll::new([3, 5]).to_string(), "3 + 5 = 8");
    }
}

/// Throwing them.
mod roll_2d6_tests {
    use super::*;

    #[test]
    fn two_dice_are_thrown() {
        assert_eq!(roll_2d6(Some(&seeded(1))).dice().len(), 2);
    }

    #[test]
    fn the_faces_come_from_the_generator() {
        assert_eq!(roll_2d6(Some(&fixed(&[2, 5]))).dice(), [2, 5]);
    }

    #[test]
    fn a_seed_makes_a_run_reproducible() {
        assert_eq!(
            roll_2d6(Some(&seeded(4))).dice(),
            roll_2d6(Some(&seeded(4))).dice()
        );
    }

    #[test]
    fn every_face_stays_on_the_die() {
        let faces: HashSet<u32> = (0..200)
            .flat_map(|seed| roll_2d6(Some(&seeded(seed))).dice().to_vec())
            .collect();

        assert_eq!(faces, (1..=DIE_FACES).collect::<HashSet<u32>>());
    }

    #[test]
    fn it_falls_back_to_its_own_generator() {
        let total = roll_2d6(None).total();

        assert!((2..=12).contains(&total));
    }
}
