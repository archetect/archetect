use std::ops::{RangeFrom, RangeInclusive, RangeToInclusive};

pub fn validate_text(min: Option<i64>, max: Option<i64>, input: &str) -> Result<(), String> {
    let length = input.len() as i64;
    match (min, max) {
        (Some(start), Some(end)) => {
            if !RangeInclusive::new(start, end).contains(&length) {
                return Err(format!("Answer must have between {} and {} characters", start, end));
            }
        }
        (Some(start), None) => {
            if !(RangeFrom { start }.contains(&length)) {
                return Err(format!("Answer must have greater than {} characters", start));
            }
        }
        (None, Some(end)) => {
            if !(RangeToInclusive { end }.contains(&length)) {
                return Err(format!("Answer must have no more than {} characters", end));
            }
        }
        (None, None) => return Ok(()),
    };

    Ok(())
}

pub fn validate_int(min: Option<i64>, max: Option<i64>, value: i64) -> Result<(), String> {
    match (min, max) {
        (Some(start), Some(end)) => {
            if !RangeInclusive::new(start, end).contains(&value) {
                return Err(format!("Answer must be between {} and {}", start, end));
            }
        }
        (Some(start), None) => {
            if !(RangeFrom { start }.contains(&value)) {
                return Err(format!("Answer must be greater than {}", start));
            }
        }
        (None, Some(end)) => {
            if !(RangeToInclusive { end }.contains(&value)) {
                return Err(format!("Answer must be less than or equal to {}", end));
            }
        }
        (None, None) => {}
    };

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::validations::validate_int;

    #[test]
    pub fn test_validate_int() {
        let result = validate_int(Some(1024), Some(65535), 8080);
        println!("{:?}", result);
    }
}