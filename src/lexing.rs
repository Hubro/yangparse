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

use std::str;

use regex::Regex;

const TAB: u8 = 9;
const NEWLINE: u8 = 10;
const CARRIAGE_RETURN: u8 = 10;
const SPACE: u8 = 32;
const DOUBLE_QUOTE: u8 = 34;
const SINGLE_QUOTE: u8 = 39;
const ASTERISK: u8 = 42;
const DASH: u8 = 45;
const SLASH: u8 = 47;
const SEMICOLON: u8 = 59;
const BACKSLASH: u8 = 92;
const LEFT_CURLY_BRACKET: u8 = 123;
const RIGHT_CURLY_BRACKET: u8 = 125;

lazy_static! {
    static ref NUMBER_PATTERN: Regex = Regex::new(r"^\-?(0|([1-9]\d*(\.\d+)?))$").unwrap();
    static ref DATE_PATTERN: Regex = Regex::new(r"^\d{4}\-\d{2}\-\d{2}$").unwrap();
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
pub struct Token<'a> {
    pub token_type: TokenType,
    pub span: (usize, usize),
    pub text: &'a str,
}

pub trait HumanReadableTokensExt {
    fn human_readable_string(&self) -> String;
}

impl HumanReadableTokensExt for Token<'_> {
    /// Format the tokens into a nice, human readable string for troubleshooting purposes
    fn human_readable_string(&self) -> String {
        format!(
            "{:<20} {:<15} {:?}\n",
            format!("{:?}", self.token_type),
            format!("{} -> {}", self.span.0, self.span.1),
            self.text,
        )
    }
}

impl HumanReadableTokensExt for Vec<Token<'_>> {
    /// Format the tokens into a nice, human readable string for troubleshooting purposes
    fn human_readable_string(&self) -> String {
        let mut output = String::new();

        for token in self {
            output.push_str(&token.human_readable_string());
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
    fn from_buffer_index(buffer: &Vec<u8>, index: usize) -> Self {
        let mut line = 1;
        let mut col = 1;

        for (i, c) in buffer.iter().enumerate() {
            if i == index {
                break;
            }

            if *c == NEWLINE {
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

pub struct ScanIterator<'a> {
    buffer: &'a Vec<u8>,
    cursor: usize,
}

impl<'a> Iterator for ScanIterator<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match next_token(self.buffer, self.cursor).expect("Parse error") {
            Some((next_cursor, token)) => {
                self.cursor = next_cursor;
                Some(token)
            }
            None => None,
        }
    }
}

pub fn scan(buffer: &Vec<u8>) -> ScanIterator {
    ScanIterator { buffer, cursor: 0 }
}

/// Reads the next token from the buffer and the next cursor position, returns None on EOF
///
/// Returns an error on lexer errors such as unterminated strings or comments.
///
fn next_token(buffer: &Vec<u8>, cursor: usize) -> Result<Option<(usize, Token)>, String> {
    let cursor = skip_whitespace(buffer, cursor);

    let char = match buffer.get(cursor) {
        Some(char) => char,
        None => return Ok(None),
    };

    macro_rules! get_str {
        ($length:expr) => {
            str::from_utf8(buffer.get(cursor..cursor + $length).unwrap())
                .map_err(|err| format!("{}", err))?
        };
    }

    macro_rules! read_token {
        ($token_type:expr, $length:expr) => {{
            let token = Token {
                token_type: $token_type,
                span: (cursor, cursor + $length - 1),
                text: get_str!($length),
            };

            Ok(Some((cursor + $length, token)))
        }};
    }

    if *char == SEMICOLON {
        return read_token!(TokenType::SemiColon, 1);
    } else if *char == LEFT_CURLY_BRACKET {
        return read_token!(TokenType::OpenCurlyBrace, 1);
    } else if *char == RIGHT_CURLY_BRACKET {
        return read_token!(TokenType::ClosingCurlyBrace, 1);
    } else if let Some(string_length) = scan_string(buffer, cursor)? {
        return read_token!(TokenType::String, string_length);
    } else if let Some(comment_length) = scan_comment(buffer, cursor) {
        return read_token!(TokenType::Comment, comment_length);
    } else if let Some(comment_length) = scan_block_comment(buffer, cursor)? {
        return read_token!(TokenType::Comment, comment_length);
    } else if let Some(token_length) = scan_other(buffer, cursor) {
        let str = get_str!(token_length);

        if NUMBER_PATTERN.is_match(str) {
            return read_token!(TokenType::Number, token_length);
        } else if DATE_PATTERN.is_match(str) {
            return read_token!(TokenType::Date, token_length);
        } else {
            return read_token!(TokenType::Other, token_length);
        }
    } else {
        return Err(format!(
            "Unexpected character at position {}: {:?}",
            cursor, char
        ));
    }
}

fn scan_line_break(buffer: &Vec<u8>, cursor: usize) -> Option<usize> {
    if let Some(first_char) = buffer.get(cursor) {
        if *first_char == NEWLINE {
            return Some(1);
        } else if *first_char == CARRIAGE_RETURN
            && buffer
                .get(cursor + 1)
                .map_or(false, |next_char| *next_char == NEWLINE)
        {
            return Some(2);
        }
    }

    None
}

/// Checks if there is a string at the current position
///
/// Returns Ok(Some(string_length)) if there is a string at the current position, Ok(None) if
/// there isn't. Returns an error if the string is never terminated.
///
fn scan_string(buffer: &Vec<u8>, cursor: usize) -> Result<Option<usize>, String> {
    let quote_char = match buffer[cursor] {
        DOUBLE_QUOTE => DOUBLE_QUOTE,
        SINGLE_QUOTE => SINGLE_QUOTE,
        _ => return Ok(None), // This position doesn't start a string, exit early
    };

    let mut prev_char: Option<&u8> = None;

    let mut i = cursor + 1;

    loop {
        if let Some(char) = buffer.get(i) {
            let prev_char_is_backslash = match prev_char {
                Some(x) => *x == BACKSLASH,
                None => false,
            };

            // If the string is closed, we're done!
            if *char == quote_char && !prev_char_is_backslash {
                return Ok(Some(i + 1 - cursor));
            }

            prev_char = Some(char);
        } else {
            return Err(format!(
                "Unexpected end of input, string started at {} was never terminated",
                TextPosition::from_buffer_index(buffer, cursor),
            ));
        }

        i += 1;
    }
}

/// Checks if there is a single-line comment at the current position
fn scan_comment(buffer: &Vec<u8>, cursor: usize) -> Option<usize> {
    let is_forward_slash = |c: &u8| *c == SLASH;

    if !(buffer.get(cursor).map_or(false, is_forward_slash)
        && buffer.get(cursor + 1).map_or(false, is_forward_slash))
    {
        return None;
    }

    let mut length = 2;

    for i in cursor + 2.. {
        // Single-line comments last until the next line break or the end of the buffer
        if scan_line_break(buffer, i).is_some() || i == buffer.len() {
            break;
        }

        length += 1;
    }

    Some(length)
}

/// Checks if there is a block comment at the current position
fn scan_block_comment(buffer: &Vec<u8>, cursor: usize) -> Result<Option<usize>, String> {
    if !(buffer.get(cursor).map_or(false, |c| *c == SLASH)
        && buffer.get(cursor + 1).map_or(false, |c| *c == ASTERISK))
    {
        return Ok(None);
    }

    let mut length = 4;

    for i in cursor + 2.. {
        if i == buffer.len() {
            return Err(format!(
                "Unexpected end of input, block comment started at {} was never terminated",
                TextPosition::from_buffer_index(buffer, cursor)
            ));
        }

        if buffer.get(i).map_or(false, |c| *c == ASTERISK)
            && buffer.get(i + 1).map_or(false, |c| *c == SLASH)
        {
            break;
        }

        length += 1;
    }

    Ok(Some(length))
}

fn scan_other(buffer: &Vec<u8>, cursor: usize) -> Option<usize> {
    let mut i = cursor;

    loop {
        if let Some(char) = buffer.get(i) {
            if is_delimiter(char) {
                break;
            }
        } else {
            break; // End of input
        }

        i += 1;
    }

    return if i > cursor { Some(i - cursor) } else { None };
}

/// Reads until a non-whitespace character is found, returns the new cursor position
fn skip_whitespace(buffer: &Vec<u8>, cursor: usize) -> usize {
    let mut cursor = cursor;

    while let Some(char) = buffer.get(cursor) {
        if [SPACE, TAB, CARRIAGE_RETURN, NEWLINE].contains(char) {
            cursor += 1;
        } else {
            break;
        }
    }

    return cursor;
}

/// Returns true if this character should delimit a token
fn is_delimiter(c: &u8) -> bool {
    [
        SPACE,
        TAB,
        CARRIAGE_RETURN,
        NEWLINE,
        SEMICOLON,
        LEFT_CURLY_BRACKET,
        RIGHT_CURLY_BRACKET,
    ]
    .contains(c)
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::util::dedent;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_scan() {
        let buffer: Vec<u8> = dedent(
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
        )
        .bytes()
        .collect();

        let tokens: Vec<_> = scan(&buffer).collect();

        assert_eq!(
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
            tokens.human_readable_string(),
        );
    }
}
