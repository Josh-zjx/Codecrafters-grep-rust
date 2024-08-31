use std::char;
use std::collections::HashMap;
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
    Group(Vec<Pattern>),
    Capture(usize),
}

#[derive(Debug, PartialEq, Eq)]
enum OCCURENCE {
    Optional,
    Once,
    OnceOrMore,
}
#[derive(Debug, PartialEq, Eq)]
struct Pattern {
    allowable: ALLOWABLE,
    repeat: OCCURENCE,
    next: Option<Box<Pattern>>,
    capture_count: usize,
}

#[derive(Debug, PartialEq, Eq)]
struct CaptureState {
    captured: HashMap<usize, String>,
    counter: usize,
}

impl Pattern {
    fn try_match(&self, input_line: &str, index: usize, cs: &mut CaptureState) -> (bool, usize) {
        let mut index = index;
        if index >= input_line.len() {
            if index > input_line.len() || self.allowable != ALLOWABLE::EndOfString {
                return (false, index);
            }
        }
        if self.repeat == OCCURENCE::Optional {
            if let Some(next) = &self.next {
                let (success, end) = next.try_match(input_line, index, cs);
                if success {
                    return (success, end);
                }
            } else {
                return (true, index);
            }
        }
        match &self.allowable {
            ALLOWABLE::Digit => {
                if !input_line.chars().nth(index).unwrap().is_numeric() {
                    return (false, index);
                }
                index += 1;
            }
            ALLOWABLE::Alnum => {
                if !input_line.chars().nth(index).unwrap().is_alphanumeric() {
                    return (false, index);
                }
                index += 1;
            }
            ALLOWABLE::Wildcard => {
                index += 1;
            }
            ALLOWABLE::CharSet(charset) => {
                if !charset.contains(&input_line.chars().nth(index).unwrap()) {
                    return (false, index);
                }
                index += 1;
            }
            ALLOWABLE::NegCharSet(charset) => {
                if charset.contains(&input_line.chars().nth(index).unwrap()) {
                    return (false, index);
                }
                index += 1;
            }
            ALLOWABLE::StartOfString => {
                if index != 0 {
                    return (false, index);
                }
            }
            ALLOWABLE::EndOfString => {
                if index != input_line.len() {
                    return (false, index);
                }
            }
            ALLOWABLE::Group(patterns) => {
                for subpattern in patterns.iter() {
                    let (success, end) = subpattern.try_match(input_line, index, cs);
                    if success {
                        cs.captured
                            .insert(self.capture_count, input_line[index..end].to_string());
                        println!(
                            "captured {:} with string {:}",
                            self.capture_count,
                            input_line[index..end].to_string()
                        );
                        if let Some(next) = &self.next {
                            let (success, end) = next.try_match(input_line, end, cs);
                            if success {
                                return (success, end);
                            }
                        } else {
                            return (true, end);
                        }
                    }
                }
                return (false, index);
            }
            ALLOWABLE::Capture(num) => {
                if cs.captured.contains_key(num) {
                    let captured = cs.captured.get(num).unwrap();
                    for c in captured.chars() {
                        if c != input_line.chars().nth(index).unwrap() {
                            println!("illegal capture {:}", captured);

                            return (false, index);
                        }
                        index += 1;
                    }
                } else {
                    println!("No corresponding capture {:}", num);
                    return (false, index);
                }
            }
        }

        if self.repeat == OCCURENCE::OnceOrMore {
            let (success, end) = self.try_match(input_line, index, cs);
            if success {
                return (success, end);
            }
        }
        if let Some(next) = &self.next {
            return next.try_match(input_line, index, cs);
        } else {
            return (true, index);
        }
    }
}

fn parse_pattern(chars: &mut Peekable<Chars>, cs: &mut CaptureState) -> Pattern {
    let mut curr = Pattern {
        allowable: ALLOWABLE::Wildcard,
        repeat: OCCURENCE::Once,
        next: None,
        capture_count: 0,
    };
    if let Some(first) = chars.next() {
        match first {
            '\\' => {
                if let Some(escaped) = chars.next() {
                    if escaped == 'd' {
                        curr.allowable = ALLOWABLE::Digit;
                    } else if escaped == 'w' {
                        curr.allowable = ALLOWABLE::Alnum;
                    } else if escaped.is_numeric() {
                        let mut match_num = escaped.to_string();
                        while chars.peek().is_some_and(|c: &char| c.is_numeric()) {
                            match_num += &chars.next().unwrap().to_string();
                        }
                        curr.allowable = ALLOWABLE::Capture(match_num.parse::<usize>().unwrap());
                    }
                }
            }
            '(' => {
                // NOTE: ')' should be handled in this procedure

                curr.capture_count = cs.counter;
                cs.counter += 1;
                let mut patterns: Vec<Pattern> = vec![];
                let sub_pattern = parse_pattern(chars, cs);
                //println!("sub_pattern: {:?}", sub_pattern);
                patterns.push(sub_pattern);
                while let Some(next) = chars.next() {
                    if next == ')' {
                        break;
                    } else if next == '|' {
                        let sub_pattern = parse_pattern(chars, cs);
                        patterns.push(sub_pattern);
                    } else {
                        panic!("Illegal Input");
                    }
                }
                curr.allowable = ALLOWABLE::Group(patterns);
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

    if chars.peek().is_none() || *chars.peek().unwrap() == '|' || *chars.peek().unwrap() == ')' {
        curr.next = None;
    } else {
        curr.next = Some(Box::new(parse_pattern(chars, cs)));
    }
    return curr;
}

fn make_pattern(pattern: &str, cs: &mut CaptureState) -> Pattern {
    let chars = pattern.chars();
    return parse_pattern(&mut chars.peekable(), cs);
}

fn match_pattern(input_line: &str, pattern: Pattern, cs: &mut CaptureState) -> bool {
    for i in 0..input_line.len() {
        let (success, _end) = pattern.try_match(input_line, i, cs);
        if success {
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
    let mut capture = CaptureState {
        captured: HashMap::default(),
        counter: 1,
    };
    let _parsed_pattern = make_pattern(pattern, &mut capture);
    println!("{:?}", _parsed_pattern);
    let _result = match_pattern(input_line, _parsed_pattern, &mut capture);
    return _result;
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
