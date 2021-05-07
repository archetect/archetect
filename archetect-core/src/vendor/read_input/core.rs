use std::{
    io::{self, Write},
    str::FromStr,
    string::ToString,
};
use crate::vendor::read_input::{Prompt, Test};

// Core function when running `.get()`.
pub(crate) fn read_input<T: FromStr>(
    prompt: &Prompt,
    err: &str,
    default: Option<T>,
    tests: &[Test<T>],
    err_pass: &dyn Fn(&T::Err) -> Option<String>,
    prompt_output: &mut dyn Write,
) -> io::Result<T> {
    fn try_flush(prompt_output: &mut dyn Write) -> () {
        prompt_output.flush().unwrap_or(())
    }

    fn input_as_string() -> io::Result<String> {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        Ok(input)
    }

    let _ = write!(prompt_output, "{}", prompt.msg);
    try_flush(prompt_output);

    loop {
        let input = input_as_string()?;

        if input.trim().is_empty() {
            if let Some(x) = default {
                return Ok(x);
            }
        };

        match parse_input(input, err, tests, err_pass) {
            Ok(v) => return Ok(v),
            Err(e) => {
                let _ = writeln!(prompt_output, "{}", e);
            }
        };

        if prompt.repeat {
            let _ = write!(prompt_output, "{}", prompt.msg);
            try_flush(prompt_output)
        };
    }
}

pub(crate) fn parse_input<T: FromStr>(
    input: String,
    err: &str,
    tests: &[Test<T>],
    err_pass: &dyn Fn(&T::Err) -> Option<String>,
) -> Result<T, String> {
    match T::from_str(&input.trim()) {
        Ok(value) => {
            for test in tests {
                if !(test.func)(&value) {
                    return Err(test.err.clone().unwrap_or_else(|| err.to_string()));
                }
            }
            Ok(value)
        }
        Err(error) => Err(err_pass(&error).unwrap_or_else(|| err.to_string())),
    }
}
