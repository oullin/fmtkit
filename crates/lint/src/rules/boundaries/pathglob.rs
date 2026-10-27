//! Go path.Match compatibility for patterns globset cannot represent exactly.

use regress::Regex;

/// Keep globset's fast path for ordinary repository globs. Its classes and `?`
/// operate on bytes, accept shell class negation, and treat a final `/**` as
/// requiring a slash. Cloud instead matches Unicode characters per segment.
pub fn needs_compatibility(pattern: &str) -> bool {
    pattern.contains(['?', '[', '\\']) || pattern.ends_with("/**")
}

pub fn compile(pattern: &str) -> Result<Regex, String> {
    let mut segments = Vec::new();

    for segment in pattern.split('/') {
        if segment != "**" || segments.last() != Some(&"**") {
            segments.push(segment);
        }
    }

    let mut expression = String::from("^");

    for (index, segment) in segments.iter().enumerate() {
        if *segment == "**" {
            expression.push_str(match (index == 0, index + 1 == segments.len()) {
                (true, true) => "[\\s\\S]*",
                (true, false) => "(?:[^/]*/)*",
                (false, true) => "(?:/[^/]*)*",
                (false, false) => "(?:/[^/]*)*/",
            });
        } else {
            if index > 0 && segments[index - 1] != "**" {
                expression.push('/');
            }
            expression.push_str(&segment_expression(segment)?);
        }
    }

    // ECMAScript's $ also accepts a final newline, which is part of a filename.
    expression.push_str("(?![\\s\\S])");

    Regex::with_flags(&expression, "u").map_err(|error| error.to_string())
}

fn literal(character: char) -> String {
    format!("\\u{{{:x}}}", u32::from(character))
}

fn segment_expression(segment: &str) -> Result<String, String> {
    let mut expression = String::new();
    let mut characters = segment.chars().peekable();

    while let Some(character) = characters.next() {
        match character {
            '*' => expression.push_str("[^/]*"),
            '?' => expression.push_str("[^/]"),
            '\\' => expression.push_str(&literal(characters.next().ok_or("dangling escape")?)),
            '[' => {
                let negated = characters.next_if_eq(&'^').is_some();
                let mut ranges = String::new();
                let mut count = 0;

                while characters.peek() != Some(&']') || count == 0 {
                    let start = class_character(&mut characters)?;
                    let end = if characters.next_if_eq(&'-').is_some() { class_character(&mut characters)? } else { start };

                    // Go accepts reversed ranges, which match no characters.
                    if start <= end {
                        ranges.push_str(&literal(start));
                        if start != end {
                            ranges.push('-');
                            ranges.push_str(&literal(end));
                        }
                    }
                    count += 1;
                }
                characters.next();

                if negated {
                    expression.push_str("[^/");
                    expression.push_str(&ranges);
                    expression.push(']');
                } else if ranges.is_empty() {
                    expression.push_str("(?!)");
                } else {
                    expression.push('[');
                    expression.push_str(&ranges);
                    expression.push(']');
                }
            }
            _ => expression.push_str(&literal(character)),
        }
    }

    Ok(expression)
}

fn class_character(characters: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Result<char, String> {
    match characters.next() {
        Some('\\') => characters.next().ok_or_else(|| "dangling class escape".into()),
        None | Some('-' | ']') => Err("malformed character class".into()),
        Some(character) => Ok(character),
    }
}
