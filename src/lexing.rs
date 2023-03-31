#![allow(dead_code, unused_variables)]
//
// Simple lexer to break up a stream of characters into a small set of tokens for further
// processing:
//
// - String: Any single- or double quoted string
// - Date: NNNN-NN-NN
// - Number: Any "integer-value" or "decimal-value" from the ABNF grammar
// - Comment: Any single-line comment or block comment
// - OpenCurlyBrace
// - ClosingCurlyBrace
// - SemiColon
// - Other: Any other token, including keywords, numbers, booleans and unquoted strings
//

use regex::Regex;

lazy_static! {
    static ref IS_NUMBER: Regex = Regex::new(r"^\-?(0|([1-9]\d*(\.\d+)?))$").unwrap();
}

#[derive(Debug, PartialEq)]
pub enum TokenType {
    String,
    Date,
    Number,
    Comment,
    OpenCurlyBrace,
    ClosingCurlyBrace,
    SemiColon,
    Other,
}

#[derive(Debug, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub span: (usize, usize),
    pub text: String,
}

pub trait HumanReadableTokensExt {
    fn human_readable_string(&self) -> String;
}

impl HumanReadableTokensExt for Vec<Token> {
    /// Format the tokens into a nice, human readable string for troubleshooting purposes
    fn human_readable_string(&self) -> String {
        let mut output = String::new();

        for token in self {
            output.push_str(&format!(
                "{:<20} {:<15} {:?}\n",
                format!("{:?}", token.token_type),
                format!("{} -> {}", token.span.0, token.span.1),
                token.text,
            ))
        }

        return output;
    }
}

/// 1-based cursor position in a text file
pub struct TextPosition {
    line: usize,
    col: usize,
}

impl TextPosition {
    fn from_buffer_index(buffer: &Vec<char>, index: usize) -> Self {
        let mut line = 1;
        let mut col = 1;

        for (i, c) in buffer.iter().enumerate() {
            if i == index {
                break;
            }

            if *c == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        Self { line, col }
    }
}

impl core::fmt::Display for TextPosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {} col {}", self.line, self.col)
    }
}

pub fn scan(text: &str) -> Result<Vec<Token>, String> {
    let buffer = Vec::from_iter(text.chars());
    let buffer_size = buffer.len();
    let mut cursor = 0;
    let mut tokens: Vec<Token> = vec![];

    loop {
        if cursor == buffer_size {
            break;
        }

        // Quick boundary check, since I'm not perfect
        if cursor > buffer_size {
            panic!("Oops, we've read too far!");
        }

        let char = buffer[cursor];

        macro_rules! push_char_token {
            ($type:expr, $char:expr) => {
                tokens.push(Token {
                    token_type: $type,
                    span: (cursor, cursor),
                    text: $char.to_string(),
                });
                cursor += 1;
            };
        }

        macro_rules! push_token {
            ($type:expr, $text:expr) => {
                let text_length = $text.len();

                tokens.push(Token {
                    token_type: $type,
                    span: (cursor, cursor + text_length - 1),
                    text: $text,
                });

                cursor += text_length;
            };
        }

        if let Some(length) = read_whitespace(&buffer, cursor) {
            cursor += length; // Ignore whitespace
        } else if let Some(length) = read_line_break(&buffer, cursor) {
            cursor += length; // Ignore line breaks
        } else if char == '{' {
            push_char_token!(TokenType::OpenCurlyBrace, '{');
        } else if char == '}' {
            push_char_token!(TokenType::ClosingCurlyBrace, '}');
        } else if char == ';' {
            push_char_token!(TokenType::SemiColon, ';');
        } else if let Some(string) = read_string(&buffer, cursor)? {
            push_token!(TokenType::String, string);
        } else if let Some(comment) = read_single_line_comment(&buffer, cursor) {
            push_token!(TokenType::Comment, comment);
        } else if let Some(comment) = read_block_comment(&buffer, cursor)? {
            push_token!(TokenType::Comment, comment);
        } else if let Some(date) = read_date(&buffer, cursor) {
            push_token!(TokenType::Date, date);
        } else if let Some(text) = read_other(&buffer, cursor) {
            if IS_NUMBER.is_match(&text) {
                push_token!(TokenType::Number, text);
            } else {
                push_token!(TokenType::Other, text);
            }
        } else {
            return Err(format!("Failed to parse input at position: {}", cursor));
        }
    }

    return Ok(tokens);
}

/// Returns true if this character should delimit a token
fn is_delimiter(c: &char) -> bool {
    [' ', '\t', '\r', '\n', ';', '{', '}'].contains(c)
}

/// Tries to read whitespace from the given start position
///
/// Returns the number of whitespace characters that was found.
///
fn read_whitespace(buffer: &Vec<char>, start: usize) -> Option<usize> {
    let mut count: usize = 0;

    for i in start.. {
        if buffer
            .get(i)
            .map_or(false, |char| [' ', '\t'].contains(char))
        {
            count += 1;
        } else {
            break;
        }
    }

    return if count > 0 { Some(count) } else { None };
}

/// Tries to read a line break at the given start position
///
/// Returns the number of characters (1 or 2) or None if no line break was found.
///
fn read_line_break(buffer: &Vec<char>, start: usize) -> Option<usize> {
    if let Some(first_char) = buffer.get(start) {
        if *first_char == '\n' {
            return Some(1);
        } else if *first_char == '\r'
            && buffer
                .get(start + 1)
                .map_or(false, |next_char| *next_char == '\n')
        {
            return Some(2);
        }
    }

    None
}

// /// Returns true if this is a valid YANG character
// ///
// /// See the definition of "yang-char" in the YANG ABNF grammar for more information.
// ///
// fn is_yang_char(c: &char) -> bool {
//     let ord = (*c) as u32;
//
//     return [0x09, 0x0A, 0x0D].contains(&ord)
//         || (0x20..=0xD7FF).contains(&ord)
//         || (0xE000..=0xFDCF).contains(&ord)
//         || (0xFDF0..=0xFFFD).contains(&ord)
//         || (0x10000..=0x1FFFD).contains(&ord)
//         || (0x20000..=0x2FFFD).contains(&ord)
//         || (0x30000..=0x3FFFD).contains(&ord)
//         || (0x40000..=0x4FFFD).contains(&ord)
//         || (0x50000..=0x5FFFD).contains(&ord)
//         || (0x60000..=0x6FFFD).contains(&ord)
//         || (0x70000..=0x7FFFD).contains(&ord)
//         || (0x80000..=0x8FFFD).contains(&ord)
//         || (0x90000..=0x9FFFD).contains(&ord)
//         || (0xA0000..=0xAFFFD).contains(&ord)
//         || (0xB0000..=0xBFFFD).contains(&ord)
//         || (0xC0000..=0xCFFFD).contains(&ord)
//         || (0xD0000..=0xDFFFD).contains(&ord)
//         || (0xE0000..=0xEFFFD).contains(&ord)
//         || (0xF0000..=0xFFFFD).contains(&ord)
//         || (0x100000..=0x10FFFD).contains(&ord);
// }

/// Tries to read a string from the cursor position in the buffer
///
/// Returns an error if the position contains a string, but it's never closed.
/// Returns None if there is no string at the given position.
/// Otherwise returns the string as a String, with the string's quotes included.
///
/// The initial cursor position is assumed to be a valid buffer index.
///
fn read_string(buffer: &Vec<char>, start: usize) -> Result<Option<String>, String> {
    let quote_char = match buffer[start] {
        '"' => '"',
        '\'' => '\'',
        _ => return Ok(None), // This position doesn't start a string, exit early
    };

    let mut string = String::from(quote_char);
    let mut prev_char: Option<&char> = None;
    let mut cursor = start + 1;

    loop {
        if let Some(curr_char) = buffer.get(cursor) {
            string.push(*curr_char);

            let prev_char_is_backslash = match prev_char {
                Some(x) => *x == '\\',
                None => false,
            };

            // If the string is closed, we're done!
            if *curr_char == quote_char && !prev_char_is_backslash {
                return Ok(Some(string));
            }

            prev_char = Some(curr_char);
        } else {
            return Err(format!(
                "Unexpected end of input, string started at {} was never terminated",
                TextPosition::from_buffer_index(buffer, start),
            ));
        }

        cursor += 1;
    }
}

fn read_single_line_comment(buffer: &Vec<char>, start: usize) -> Option<String> {
    let is_forward_slash = |c: &char| *c == '/';

    if !(buffer.get(start).map_or(false, is_forward_slash)
        && buffer.get(start + 1).map_or(false, is_forward_slash))
    {
        return None;
    }

    let mut length = 2;

    for i in start + 2.. {
        // Single-line comments last until the next line break or the end of the buffer
        if read_line_break(buffer, i).is_some() || i == buffer.len() {
            break;
        }

        length += 1;
    }

    Some(String::from_iter(
        buffer
            .get(start..start + length)
            .expect("The bounds have already been checked"),
    ))
}

/// Tries to read a block comment at the given position
///
/// Returns the comment as a string if it was successfully read. Returns None if there was no
/// multi-line string at the position. Returns an error if the multi-line comment was opened but
/// never closed.
///
fn read_block_comment(buffer: &Vec<char>, start: usize) -> Result<Option<String>, String> {
    if !(buffer.get(start).map_or(false, |c| *c == '/')
        && buffer.get(start + 1).map_or(false, |c| *c == '*'))
    {
        return Ok(None);
    }

    let mut length = 4;

    for i in start + 2.. {
        if i == buffer.len() {
            return Err(format!(
                "Unexpected end of input, block comment started at {} was never terminated",
                TextPosition::from_buffer_index(buffer, start)
            ));
        }

        if buffer.get(i).map_or(false, |c| *c == '*')
            && buffer.get(i + 1).map_or(false, |c| *c == '/')
        {
            break;
        }

        length += 1;
    }

    Ok(Some(String::from_iter(
        buffer
            .get(start..start + length)
            .expect("The bounds have already been checked"),
    )))
}

/// Tries to read any token from the given position
///
/// This means any chunk of text up until a delimiter.
///
/// The initial cursor position is assumed to be a valid buffer index.
///
fn read_other(buffer: &Vec<char>, cursor: usize) -> Option<String> {
    let mut string = String::new();
    let mut cursor = cursor;

    loop {
        if let Some(char) = buffer.get(cursor) {
            if is_delimiter(char) {
                break;
            }

            string.push(*char);
        } else {
            break; // End of input
        }

        cursor += 1;
    }

    return if string.len() > 0 { Some(string) } else { None };
}

/// Tries to read a date at the given position in the buffer
fn read_date(buffer: &Vec<char>, cursor: usize) -> Option<String> {
    if let Some(chunk) = buffer.get(cursor..cursor + 10) {
        if chunk[0].is_numeric()
            && chunk[1].is_numeric()
            && chunk[2].is_numeric()
            && chunk[3].is_numeric()
            && chunk[4] == '-'
            && chunk[5].is_numeric()
            && chunk[6].is_numeric()
            && chunk[7] == '-'
            && chunk[8].is_numeric()
            && chunk[9].is_numeric()
        {
            return Some(String::from_iter(chunk));
        }
    }

    return None;
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util::dedent;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_scan() {
        let result = scan(&dedent(
            r#"
            module test {
                namespace "A double quoted string";
                description 'A single quoted string';

                revision 2018-12-03 {
                    // I'm a comment!
                }

                number 0;
                number 123;
                number -123;
                number 123.12345;
                number -123.12345;

                not-number 0123;
                not-number abc123;

                /* I'm a multi-line comment */

                /*
                 * I'm a weird multi-line comment thingy
                 */
            }
            "#,
        ))
        .unwrap();

        assert_eq!(
            result.human_readable_string(),
            dedent(
                r#"
                Other                0 -> 5          "module"
                Other                7 -> 10         "test"
                OpenCurlyBrace       12 -> 12        "{"
                Other                18 -> 26        "namespace"
                String               28 -> 51        "\"A double quoted string\""
                SemiColon            52 -> 52        ";"
                Other                58 -> 68        "description"
                String               70 -> 93        "'A single quoted string'"
                SemiColon            94 -> 94        ";"
                Other                101 -> 108      "revision"
                Date                 110 -> 119      "2018-12-03"
                OpenCurlyBrace       121 -> 121      "{"
                Comment              131 -> 147      "// I'm a comment!"
                ClosingCurlyBrace    153 -> 153      "}"
                Other                160 -> 165      "number"
                Number               167 -> 167      "0"
                SemiColon            168 -> 168      ";"
                Other                174 -> 179      "number"
                Number               181 -> 183      "123"
                SemiColon            184 -> 184      ";"
                Other                190 -> 195      "number"
                Number               197 -> 200      "-123"
                SemiColon            201 -> 201      ";"
                Other                207 -> 212      "number"
                Number               214 -> 222      "123.12345"
                SemiColon            223 -> 223      ";"
                Other                229 -> 234      "number"
                Number               236 -> 245      "-123.12345"
                SemiColon            246 -> 246      ";"
                Other                253 -> 262      "not-number"
                Other                264 -> 267      "0123"
                SemiColon            268 -> 268      ";"
                Other                274 -> 283      "not-number"
                Other                285 -> 290      "abc123"
                SemiColon            291 -> 291      ";"
                Comment              298 -> 327      "/* I'm a multi-line comment */"
                Comment              334 -> 388      "/*\n     * I'm a weird multi-line comment thingy\n     */"
                ClosingCurlyBrace    390 -> 390      "}"
                "#
            ),
        );
    }
}
