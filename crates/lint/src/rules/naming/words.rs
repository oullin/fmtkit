//! Words in kebab-case, snake_case, camelCase and PascalCase names, compared
//! without case.

/// One word of a name, by byte range, with its lowercase form.
pub struct Word {
    pub start: usize,
    pub lower: String,
}

/// The words of `name`. Any character that is not a letter or digit separates
/// words, and so does a case change: `HTTPClient` is `http client` and
/// `oauth2Client` is `oauth2 client`.
pub fn split(name: &str) -> Vec<Word> {
    let chars: Vec<(usize, char)> = name.char_indices().collect();
    let mut words = Vec::new();
    let mut start: Option<usize> = None;

    for (i, &(at, c)) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            if let Some(from) = start.take() {
                words.push(word(name, from, at));
            }
            continue;
        }

        if let Some(from) = start {
            let prev = chars[i - 1].1;
            let next = chars.get(i + 1).map(|&(_, c)| c);
            let lower_to_upper = (prev.is_lowercase() || prev.is_ascii_digit()) && c.is_uppercase();
            let acronym_end = prev.is_uppercase() && c.is_uppercase() && next.is_some_and(char::is_lowercase);

            if lower_to_upper || acronym_end {
                words.push(word(name, from, at));
                start = Some(at);
            }
        } else {
            start = Some(at);
        }
    }

    if let Some(from) = start {
        words.push(word(name, from, name.len()));
    }

    words
}

fn word(name: &str, start: usize, end: usize) -> Word {
    Word { start, lower: name[start..end].to_lowercase() }
}

/// The lowercase words of `name`.
pub fn phrase(name: &str) -> Vec<String> {
    split(name).into_iter().map(|word| word.lower).collect()
}

/// `name` as a phrase, and again with its last word made singular for each
/// candidate singular: `notifications` gives `notifications` and
/// `notification`, and `caches` gives `caches`, `cach` and `cache`.
pub fn forms(name: &str) -> Vec<Vec<String>> {
    let plural = phrase(name);
    let mut forms = vec![plural.clone()];

    if let Some((last, rest)) = plural.split_last() {
        for single in singulars(last) {
            let mut form = rest.to_vec();

            form.push(single);

            if !forms.contains(&form) {
                forms.push(form);
            }
        }
    }

    forms.retain(|form| !form.is_empty());
    forms
}

/// Every plausible singular of an English plural, or none when `word` does
/// not read as one: `policies` gives `policy` and `policie`, `cookies` gives
/// `cooky` and `cookie`, `addresses` gives `address`, and `access` gives none.
/// A wrong candidate costs nothing, because a prefix must match whole words.
fn singulars(word: &str) -> Vec<String> {
    let mut found = Vec::new();

    if word.len() > 3
        && let Some(stem) = word.strip_suffix("ies")
    {
        found.push(format!("{stem}y"));
        found.push(format!("{stem}ie"));
    }

    for suffix in ["sses", "xes", "ches", "shes", "zes"] {
        if word.len() > suffix.len() && word.ends_with(suffix) {
            found.push(word[..word.len() - 2].to_owned());

            // `caches` and `sizes` drop only the `s`; `addresses` does not.
            if suffix != "sses" {
                found.push(word[..word.len() - 1].to_owned());
            }
        }
    }

    if found.is_empty() && word.len() > 1 && word.ends_with('s') && !["ss", "us", "is"].iter().any(|suffix| word.ends_with(suffix)) {
        found.push(word[..word.len() - 1].to_owned());
    }

    found
}

/// Whether `words`, from `at`, start with `prefix`.
pub fn starts_with(words: &[Word], at: usize, prefix: &[String]) -> bool {
    words.len() >= at + prefix.len() && words[at..at + prefix.len()].iter().zip(prefix).all(|(word, part)| word.lower == *part)
}

/// `name` from its word `from` on, cased the way `name` is: `magicLinkThrottle`
/// from word 2 is `throttle`, `MagicLinkThrottle` is `Throttle`, and
/// `MAGIC_LINK_TTL` is `TTL`. A leading `_` or `$` is kept.
pub fn rest(name: &str, words: &[Word], from: usize) -> String {
    let lead = words.first().map_or("", |first| &name[..first.start]);
    let tail = &name[words[from].start..];
    let camel = name[lead.len()..].starts_with(char::is_lowercase);

    if !camel {
        return format!("{lead}{tail}");
    }

    let end = words.get(from + 1).map_or(tail.len(), |next| next.start - words[from].start);
    let (first, after) = tail.split_at(end);
    let acronym = first.chars().filter(|c| c.is_alphabetic()).all(char::is_uppercase);
    let first = if acronym {
        first.to_lowercase()
    } else {
        let mut chars = first.chars();

        chars.next().map_or_else(String::new, |c| c.to_lowercase().chain(chars).collect())
    };

    format!("{lead}{first}{after}")
}
