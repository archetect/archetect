use crate::{core::parse_input, shortcut::input, InputBuild, InputBuilder};
use std::str::FromStr;

fn parse_with_builder<T: FromStr>(builder: InputBuilder<T>, input: String) -> Result<T, String> {
    parse_input(input, &builder.err, &builder.tests, &*builder.err_match)
}

#[test]
fn test_range() {
    assert_eq!(
        parse_with_builder(input().inside(4..9).err("1"), "3".to_string()),
        Err("1".to_string())
    );
    assert_eq!(
        parse_with_builder(input().inside(4..9).err("1"), "4".to_string()),
        Ok(4)
    );
    assert_eq!(
        parse_with_builder(input().inside(4..9).err("1"), "8".to_string()),
        Ok(8)
    );
    assert_eq!(
        parse_with_builder(input().inside(4..9).err("1"), "9".to_string()),
        Err("1".to_string())
    );
}

#[test]
fn test_range_from() {
    assert_eq!(
        parse_with_builder(input().inside(6..).err("1"), "5".to_string()),
        Err("1".to_string())
    );
    assert_eq!(
        parse_with_builder(input().inside(6..).err("1"), "6".to_string()),
        Ok(6)
    );
    assert_eq!(
        parse_with_builder(input().inside(6..).err("1"), "10".to_string()),
        Ok(10)
    );
}

#[test]
fn test_range_inclusive() {
    assert_eq!(
        parse_with_builder(input().inside(4..=9).err("1"), "3".to_string()),
        Err("1".to_string())
    );
    assert_eq!(
        parse_with_builder(input().inside(4..=9).err("1"), "4".to_string()),
        Ok(4)
    );
    assert_eq!(
        parse_with_builder(input().inside(4..=9).err("1"), "8".to_string()),
        Ok(8)
    );
    assert_eq!(
        parse_with_builder(input().inside(4..=9).err("1"), "9".to_string()),
        Ok(9)
    );
    assert_eq!(
        parse_with_builder(input().inside(4..=9).err("1"), "10".to_string()),
        Err("1".to_string())
    );
}

#[test]
fn test_range_to() {
    assert_eq!(
        parse_with_builder(input().inside(..6).err("1"), "2".to_string()),
        Ok(2)
    );
    assert_eq!(
        parse_with_builder(input().inside(..6).err("1"), "5".to_string()),
        Ok(5)
    );
    assert_eq!(
        parse_with_builder(input().inside(..6).err("1"), "6".to_string()),
        Err("1".to_string())
    );
    assert_eq!(
        parse_with_builder(input().inside(..6).err("1"), "7".to_string()),
        Err("1".to_string())
    );
}

#[test]
fn test_range_to_inclusive() {
    assert_eq!(
        parse_with_builder(input().inside(..=6).err("1"), "2".to_string()),
        Ok(2)
    );
    assert_eq!(
        parse_with_builder(input().inside(..=6).err("1"), "5".to_string()),
        Ok(5)
    );
    assert_eq!(
        parse_with_builder(input().inside(..=6).err("1"), "6".to_string()),
        Ok(6)
    );
    assert_eq!(
        parse_with_builder(input().inside(..=6).err("1"), "7".to_string()),
        Err("1".to_string())
    );
}

#[test]
fn test_range_full() {
    assert_eq!(
        parse_with_builder(input().inside(..).err("1"), "2".to_string()),
        Ok(2)
    );
    assert_eq!(
        parse_with_builder(input().inside(..).err("1"), "5".to_string()),
        Ok(5)
    );
}

#[test]
fn test_array() {
    assert_eq!(
        parse_with_builder(input().inside(vec![2, 6, 7]).err("1"), "2".to_string()),
        Ok(2)
    );
    assert_eq!(
        parse_with_builder(input().inside(vec![2, 6, 7]).err("1"), "6".to_string()),
        Ok(6)
    );
    assert_eq!(
        parse_with_builder(input().inside(vec![2, 6, 7]).err("1"), "3".to_string()),
        Err("1".to_string())
    );
}
