use std::char;
use std::collections::HashSet;
use std::env;
use std::io;
use std::iter::Peekable;
use std::process;
use std::str::Chars;

#[derive(Debug, PartialEq, Eq)]
enum ALLOWABLE {
    Digit,
    Alnum,
    Wildcard,
    StartOfString,
    EndOfString,
    CharSet(HashSet<char>),
    NegCharSet(HashSet<char>),
}

#[derive(Debug, PartialEq, Eq)]
enum OCCURENCE {
    Optional,
    Once,
    OnceOrMore,
}
#[derive(Debug)]
struct Pattern {
    allowable: ALLOWABLE,
    repeat: OCCURENCE,
    next: Option<Box<Pattern>>,
}

impl Pattern {
    fn try_match(&self, input_line: &str, index: usize) -> bool {
        let mut index = index;
        if index >= input_line.len() {
            if index > input_line.len() || self.allowable != ALLOWABLE::EndOfString {
                return false;
            }
        }
        if self.repeat == OCCURENCE::Optional {
            if let Some(next) = &self.next {
                if next.try_match(input_line, index) {
                    return true;
                }
            } else {
                return true;
            }
        }
        match &self.allowable {
            ALLOWABLE::Digit => {
                if !input_line.chars().nth(index).unwrap().is_numeric() {
                    return false;
                }
                index += 1;
            }
            ALLOWABLE::Alnum => {
                if !input_line.chars().nth(index).unwrap().is_alphanumeric() {
                    return false;
                }
                index += 1;
            }
            ALLOWABLE::Wildcard => {
                index += 1;
            }
            ALLOWABLE::CharSet(charset) => {
                if !charset.contains(&input_line.chars().nth(index).unwrap()) {
                    return false;
                }
                index += 1;
            }
            ALLOWABLE::NegCharSet(charset) => {
                if charset.contains(&input_line.chars().nth(index).unwrap()) {
                    return false;
                }
                index += 1;
            }
            ALLOWABLE::StartOfString => {
                if index != 0 {
                    return false;
                }
            }
            ALLOWABLE::EndOfString => {
                if index != input_line.len() {
                    return false;
                }
            }
        }
        if let Some(next) = &self.next {
            if self.repeat == OCCURENCE::OnceOrMore {
                return next.try_match(input_line, index) || self.try_match(input_line, index);
            }
            return next.try_match(input_line, index);
        } else {
            return true;
        }
    }
}

fn parse_pattern(mut chars: Peekable<Chars>) -> Pattern {
    let mut curr = Pattern {
        allowable: ALLOWABLE::Wildcard,
        repeat: OCCURENCE::Once,
        next: None,
    };
    if let Some(first) = chars.next() {
        match first {
            '\\' => {
                if let Some(escaped) = chars.next() {
                    if escaped == 'd' {
                        curr.allowable = ALLOWABLE::Digit;
                    } else if escaped == 'w' {
                        curr.allowable = ALLOWABLE::Alnum;
                    }
                }
            }
            '[' => {
                let mut neg = false;
                let mut charset: HashSet<char> = HashSet::default();
                while let Some(c) = chars.next() {
                    if c == ']' {
                        if neg {
                            curr.allowable = ALLOWABLE::NegCharSet(charset);
                        } else {
                            curr.allowable = ALLOWABLE::CharSet(charset);
                        }
                        break;
                    } else if c == '^' {
                        neg = true;
                    } else {
                        charset.insert(c);
                    }
                }
            }
            '.' => {
                curr.allowable = ALLOWABLE::Wildcard;
            }
            '^' => {
                curr.allowable = ALLOWABLE::StartOfString;
            }
            '$' => {
                curr.allowable = ALLOWABLE::EndOfString;
            }
            c => {
                let mut charset: HashSet<char> = HashSet::default();
                charset.insert(c);
                curr.allowable = ALLOWABLE::CharSet(charset);
            }
        }
    }
    if let Some(peek) = chars.peek() {
        if *peek == '+' {
            curr.repeat = OCCURENCE::OnceOrMore;
            chars.next();
        } else if *peek == '?' {
            curr.repeat = OCCURENCE::Optional;
            chars.next();
        }
    }

    if chars.peek().is_none() {
        curr.next = None;
    } else {
        curr.next = Some(Box::new(parse_pattern(chars)));
    }
    return curr;
}

fn make_pattern(pattern: &str) -> Pattern {
    let chars = pattern.chars();
    return parse_pattern(chars.peekable());
}

fn match_pattern(input_line: &str, pattern: Pattern) -> bool {
    for i in 0..input_line.len() {
        if pattern.try_match(input_line, i) {
            return true;
        }
    }
    return false;
}

fn match_string(input_line: &str, pattern: &str) -> bool {
    /*
    for i in 0..input_line.len() {
        if match_pattern(input_line, i, parse_pattern(pattern)) {
            return true;
        }
    }
    return false;
    */
    let _parsed_pattern = make_pattern(pattern);
    let _result = match_pattern(input_line, _parsed_pattern);
    return _result;

    if pattern.chars().count() == 1 {
        return input_line.contains(pattern);
    } else if pattern == r"\w" {
        for i in input_line.chars() {
            if i.is_alphanumeric() || i == '_' {
                return true;
            }
        }
        return false;
    } else if pattern.starts_with('[') {
        let mut allowable: HashSet<char> = HashSet::default();

        for i in pattern.chars() {
            if i != '[' && i != ']' && i != '^' {
                allowable.insert(i);
            }
        }

        if (pattern.bytes().nth(1).unwrap()) == b'^' {
            for i in input_line.chars() {
                if !allowable.contains(&i) {
                    return true;
                }
            }
            return false;
        }
        for i in input_line.chars() {
            if allowable.contains(&i) {
                return true;
            }
        }
        return false;
    } else if pattern == r"\d" {
        for i in input_line.chars() {
            if i.is_numeric() {
                return true;
            }
        }
        return false;
    } else {
        panic!("Unhandled pattern: {}", pattern)
    }
}

// Usage: echo <input_text> | your_program.sh -E <pattern>
fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    if env::args().nth(1).unwrap() != "-E" {
        println!("Expected first argument to be '-E'");
        process::exit(1);
    }

    let pattern = env::args().nth(2).unwrap();
    let mut input_line = String::new();

    io::stdin().read_line(&mut input_line).unwrap();

    // Uncomment this block to pass the first stage
    if match_string(&input_line, &pattern) {
        process::exit(0)
    } else {
        process::exit(1)
    }
}
