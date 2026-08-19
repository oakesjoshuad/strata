use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub(crate) fn slugify(title: &str) -> String {
    let mut slug = String::new();
    let mut pending_hyphen = false;

    for character in title.nfkd() {
        if is_combining_mark(character) {
            continue;
        }
        if character.is_ascii_alphanumeric() {
            if pending_hyphen && !slug.is_empty() {
                if slug.len() == 64 {
                    break;
                }
                slug.push('-');
            }
            if slug.len() == 64 {
                break;
            }
            slug.push(character.to_ascii_lowercase());
            pending_hyphen = false;
        } else if !slug.is_empty() {
            pending_hyphen = true;
        }
    }

    if slug.is_empty() {
        "untitled".into()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_applies_the_persisted_slug_rules() {
        assert_eq!(slugify("Café: SQLite / Store!"), "cafe-sqlite-store");
        assert_eq!(slugify("中文"), "untitled");
        assert_eq!(slugify("A".repeat(80).as_str()).len(), 64);
        assert_eq!(slugify("hello---world"), "hello-world");
    }
}
