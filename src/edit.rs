extern crate hex;
extern crate plist;

use crate::draw;

//use plist::Dictionary;
use plist::{Integer, Value};

use console::{Key, Term};

use std::io::{self, Write};

pub fn edit_value(position: &draw::Position, mut val: &mut Value, term: &Term) -> io::Result<()> {
    term.show_cursor()?;
    for i in 0..position.depth + 1 {
        match val {
            Value::Dictionary(d) => {
                let k = d.keys().map(|s| s.to_string()).collect::<Vec<String>>()
                    [position.section[i]]
                    .clone();
                val = d.get_mut(&k).unwrap();
            }
            Value::Array(a) => {
                val = a.get_mut(position.section[i]).unwrap();
            }
            _ => (),
        }
    }
    match val {
        Value::Boolean(b) => *b = !*b,
        Value::Integer(i) => edit_int(i, term),
        Value::String(s) => edit_string(s, term),
        Value::Data(d) => edit_data(d, term),
        _ => (),
    }
    term.hide_cursor()?;

    Ok(())
}

fn edit_data(val: &mut Vec<u8>, term: &Term) {
    let mut new = String::from_utf8(val.clone()).unwrap();
    loop {
        write!(
            &*term,
            "\x1B[u{} | 0x{} | {}\x1B[0K",
            base64::encode(&new),
            hex::encode_upper(&new),
            new
        )
        .unwrap();
        let key = term.read_key().unwrap();
        match key {
            Key::Enter => {
                *val = Vec::<u8>::from(new);
                break;
            }
            Key::Backspace => {
                if new.len() > 0 {
                    let _ = new.pop().unwrap();
                }
            }
            Key::Char(c) => new.push(c),
            //            Key::Char(c @ '0' ..= '9') => new.push(c),
            //            Key::Char(c @ 'A' ..= 'F') => new.push(c),
            //            Key::Char(c @ 'a' ..= 'f') => new.push(c),
            //            Key::Char('-') => {
            //                if new.len() == 0 {
            //                    new.push('-');
            //                }
            //            }
            Key::Escape => break,
            _ => (),
        }
    }
}

fn edit_int(val: &mut Integer, term: &Term) {
    let mut new = val.to_string();
    write!(&*term, "\x1B[u{}", new).unwrap();
    loop {
        let key = term.read_key().unwrap();
        match key {
            Key::Enter => {
                *val = Integer::from(new.parse::<i32>().unwrap());
                break;
            }
            Key::Backspace => {
                if new.len() > 0 {
                    let _ = new.pop().unwrap();
                }
            }
            Key::Char(c @ '0'..='9') => new.push(c),
            Key::Char('-') => {
                if new.len() == 0 {
                    new.push('-');
                }
            }
            Key::Escape => break,
            _ => (),
        }
        write!(&*term, "\x1B[u{}\x1B[0K", new).unwrap();
    }
}

fn edit_string(val: &mut String, term: &Term) {
    let mut new = String::from(&*val);
    write!(&*term, "\x1B[u{}", new).unwrap();
    loop {
        let key = term.read_key().unwrap();
        match key {
            Key::Enter => {
                *val = new;
                break;
            }
            Key::Backspace => {
                if new.len() > 0 {
                    let _ = new.pop().unwrap();
                }
            }
            Key::Char(c) => new.push(c),
            Key::Escape => break,
            _ => (),
        }
        write!(&*term, "\x1B[u{}\x1B[0K", new).unwrap();
    }
}
