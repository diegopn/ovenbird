use super::*;

/// Converts common BibTeX/LaTeX text accents to readable Unicode for display.
/// The stored BibTeX values remain unchanged for editing and export.
pub fn display_bibtex_text(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut index = 0;
    decode_tex_display(&characters, &mut index, false)
}

pub fn display_bibtex_names(value: &str) -> Vec<String> {
    split_bibtex_names(value)
        .into_iter()
        .map(|name| display_bibtex_text(&name))
        .filter(|name| !name.trim().is_empty())
        .collect()
}

fn decode_tex_display(characters: &[char], index: &mut usize, in_group: bool) -> String {
    let mut output = String::new();
    while *index < characters.len() {
        match characters[*index] {
            '}' if in_group => break,
            '{' => {
                *index += 1;
                output.push_str(&decode_tex_display(characters, index, true));
                if characters.get(*index) == Some(&'}') {
                    *index += 1;
                }
            }
            '\\' => output.push_str(&decode_tex_command(characters, index)),
            character => {
                output.push(character);
                *index += 1;
            }
        }
    }
    output
}

fn decode_tex_command(characters: &[char], index: &mut usize) -> String {
    *index += 1;
    let Some(&first) = characters.get(*index) else {
        return "\\".to_owned();
    };

    let accent = match first {
        '\'' => Some('´'),
        '`' => Some('`'),
        '^' => Some('^'),
        '~' => Some('~'),
        '"' => Some('¨'),
        '=' => Some('¯'),
        '.' => Some('˙'),
        _ => None,
    };
    if let Some(accent) = accent {
        *index += 1;
        return read_tex_display_argument(characters, index)
            .map(|argument| apply_tex_accent(&argument, accent))
            .unwrap_or_else(|| format!("\\{first}"));
    }

    if first.is_ascii_alphabetic() {
        let start = *index;
        while characters
            .get(*index)
            .is_some_and(|character| character.is_ascii_alphabetic())
        {
            *index += 1;
        }
        let command = characters[start..*index].iter().collect::<String>();
        let accent = match command.as_str() {
            "c" => Some('¸'),
            "v" => Some('ˇ'),
            "u" => Some('˘'),
            "H" => Some('˝'),
            "r" => Some('˚'),
            _ => None,
        };
        if let Some(accent) = accent {
            return read_tex_display_argument(characters, index)
                .map(|argument| apply_tex_accent(&argument, accent))
                .unwrap_or_else(|| format!("\\{command}"));
        }
        return match command.as_str() {
            "i" => "ı".to_owned(),
            "j" => "ȷ".to_owned(),
            "ae" => "æ".to_owned(),
            "AE" => "Æ".to_owned(),
            "oe" => "œ".to_owned(),
            "OE" => "Œ".to_owned(),
            "aa" => "å".to_owned(),
            "AA" => "Å".to_owned(),
            "o" => "ø".to_owned(),
            "O" => "Ø".to_owned(),
            "ss" => "ß".to_owned(),
            "l" => "ł".to_owned(),
            "L" => "Ł".to_owned(),
            "dh" => "ð".to_owned(),
            "DH" => "Ð".to_owned(),
            "th" => "þ".to_owned(),
            "TH" => "Þ".to_owned(),
            "ng" => "ŋ".to_owned(),
            "NG" => "Ŋ".to_owned(),
            _ => format!("\\{command}"),
        };
    }

    *index += 1;
    match first {
        '&' | '%' | '$' | '#' | '_' | '{' | '}' => first.to_string(),
        '\\' => "\\".to_owned(),
        ' ' => " ".to_owned(),
        _ => format!("\\{first}"),
    }
}

fn read_tex_display_argument(characters: &[char], index: &mut usize) -> Option<String> {
    while characters
        .get(*index)
        .is_some_and(|character| character.is_whitespace())
    {
        *index += 1;
    }
    match characters.get(*index).copied()? {
        '{' => {
            *index += 1;
            let argument = decode_tex_display(characters, index, true);
            if characters.get(*index) == Some(&'}') {
                *index += 1;
            }
            Some(argument)
        }
        _ => Some(decode_tex_command_or_character(characters, index)),
    }
}

fn decode_tex_command_or_character(characters: &[char], index: &mut usize) -> String {
    if characters.get(*index) == Some(&'\\') {
        decode_tex_command(characters, index)
    } else {
        let Some(&character) = characters.get(*index) else {
            return String::new();
        };
        *index += 1;
        character.to_string()
    }
}

fn apply_tex_accent(value: &str, accent: char) -> String {
    let mut characters = value.chars();
    let Some(base) = characters.next() else {
        return value.to_owned();
    };
    let rest = characters.collect::<String>();
    let composed = match (base, accent) {
        ('A', '´') => 'Á',
        ('E', '´') => 'É',
        ('I', '´') => 'Í',
        ('O', '´') => 'Ó',
        ('U', '´') => 'Ú',
        ('Y', '´') => 'Ý',
        ('C', '´') => 'Ć',
        ('N', '´') => 'Ń',
        ('a', '´') => 'á',
        ('e', '´') => 'é',
        ('i', '´') => 'í',
        ('o', '´') => 'ó',
        ('u', '´') => 'ú',
        ('y', '´') => 'ý',
        ('c', '´') => 'ć',
        ('n', '´') => 'ń',
        ('A', '`') => 'À',
        ('E', '`') => 'È',
        ('I', '`') => 'Ì',
        ('O', '`') => 'Ò',
        ('U', '`') => 'Ù',
        ('a', '`') => 'à',
        ('e', '`') => 'è',
        ('i', '`') => 'ì',
        ('o', '`') => 'ò',
        ('u', '`') => 'ù',
        ('A', '^') => 'Â',
        ('E', '^') => 'Ê',
        ('I', '^') => 'Î',
        ('O', '^') => 'Ô',
        ('U', '^') => 'Û',
        ('a', '^') => 'â',
        ('e', '^') => 'ê',
        ('i', '^') => 'î',
        ('o', '^') => 'ô',
        ('u', '^') => 'û',
        ('A', '~') => 'Ã',
        ('N', '~') => 'Ñ',
        ('O', '~') => 'Õ',
        ('a', '~') => 'ã',
        ('n', '~') => 'ñ',
        ('o', '~') => 'õ',
        ('A', '¨') => 'Ä',
        ('E', '¨') => 'Ë',
        ('I', '¨') => 'Ï',
        ('O', '¨') => 'Ö',
        ('U', '¨') => 'Ü',
        ('Y', '¨') => 'Ÿ',
        ('a', '¨') => 'ä',
        ('e', '¨') => 'ë',
        ('i', '¨') => 'ï',
        ('o', '¨') => 'ö',
        ('u', '¨') => 'ü',
        ('y', '¨') => 'ÿ',
        ('C', '¸') => 'Ç',
        ('S', '¸') => 'Ş',
        ('T', '¸') => 'Ţ',
        ('c', '¸') => 'ç',
        ('s', '¸') => 'ş',
        ('t', '¸') => 'ţ',
        ('A', 'ˇ') => 'Ǎ',
        ('C', 'ˇ') => 'Č',
        ('D', 'ˇ') => 'Ď',
        ('E', 'ˇ') => 'Ě',
        ('L', 'ˇ') => 'Ľ',
        ('N', 'ˇ') => 'Ň',
        ('R', 'ˇ') => 'Ř',
        ('S', 'ˇ') => 'Š',
        ('T', 'ˇ') => 'Ť',
        ('Z', 'ˇ') => 'Ž',
        ('a', 'ˇ') => 'ǎ',
        ('c', 'ˇ') => 'č',
        ('d', 'ˇ') => 'ď',
        ('e', 'ˇ') => 'ě',
        ('l', 'ˇ') => 'ľ',
        ('n', 'ˇ') => 'ň',
        ('r', 'ˇ') => 'ř',
        ('s', 'ˇ') => 'š',
        ('t', 'ˇ') => 'ť',
        ('z', 'ˇ') => 'ž',
        ('A', '˘') => 'Ă',
        ('G', '˘') => 'Ğ',
        ('U', '˘') => 'Ŭ',
        ('a', '˘') => 'ă',
        ('g', '˘') => 'ğ',
        ('u', '˘') => 'ŭ',
        ('O', '˝') => 'Ő',
        ('U', '˝') => 'Ű',
        ('o', '˝') => 'ő',
        ('u', '˝') => 'ű',
        ('A', '˚') => 'Å',
        ('U', '˚') => 'Ů',
        ('a', '˚') => 'å',
        ('u', '˚') => 'ů',
        _ => '\0',
    };
    let mut output = String::new();
    if composed != '\0' {
        output.push(composed);
    } else {
        output.push(base);
        output.push(match accent {
            '´' => '\u{0301}',
            '`' => '\u{0300}',
            '^' => '\u{0302}',
            '~' => '\u{0303}',
            '¨' => '\u{0308}',
            '¯' => '\u{0304}',
            '˙' => '\u{0307}',
            '¸' => '\u{0327}',
            'ˇ' => '\u{030c}',
            '˘' => '\u{0306}',
            '˝' => '\u{030b}',
            '˚' => '\u{030a}',
            _ => accent,
        });
    }
    output.push_str(&rest);
    output
}
