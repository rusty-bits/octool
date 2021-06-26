extern crate hex;
extern crate plist;
extern crate termion;

use plist::{Dictionary, Value};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::*;

use std::env;
use std::io::{Write, stdout, stdin};

struct Position {
    section: i32,
    sec_length: usize,
    depth: i32,
}

impl Position {
    fn up(&mut self) {
        self.section -= 1;
        if self.section < 0 {
            self.section = 0;
        }
    }

    fn down(&mut self) {
        self.section += 1;
        if self.section == self.sec_length as i32 {
            self.section -= 1;
        }
    }

    fn right(&mut self) {
        self.depth += 1;
    }

    fn left(&mut self) {
        self.depth -= 1;
        if self.depth < 0 {
            self.depth = 0;
        }
    }
}

fn main() {
    let file = env::args()
        .nth(1)
        .unwrap_or("INPUT/Sample.plist".to_string());

    let mut list =
        Value::from_file(&file).expect(format!("Didn't find plist at {}", file).as_str());

    let oc_plist = list.as_dictionary_mut().unwrap();

    let oc_keys: Vec<String> = oc_plist.keys().map(|s| s.to_string()).collect();

    let mut position = Position {
        section: 0,
        sec_length: oc_keys.len(),
        depth: 0,
    };

    let stdin = stdin();
    let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
    write!(screen, "{}", termion::cursor::Hide).unwrap();


    update_screen(&mut position, &oc_keys, oc_plist, &mut screen);
    screen.flush().unwrap();

    for c in stdin.keys() {
        match c.unwrap() {
            Key::Char('q') => break,
            Key::Down => {
                position.down();
                update_screen(&mut position, &oc_keys, oc_plist, &mut screen);
            }
            Key::Up => {
                position.up();
                update_screen(&mut position, &oc_keys, oc_plist, &mut screen);
            }
            Key::Right => {
                position.right();
                update_screen(&mut position, &oc_keys, oc_plist, &mut screen);
            }
            Key::Left => {
                position.left();
                update_screen(&mut position, &oc_keys, oc_plist, &mut screen);
            }
            _ => {}
        }
        screen.flush().unwrap();
    }
    write!(screen, "{}", termion::cursor::Show).unwrap();
    let _e = list.to_file_xml("test1");
}

fn update_screen<W: Write>(position: &mut Position, keys: &Vec<String>, oc_plist: &mut Dictionary, screen: &mut W) {
    write!(screen, "{}{}", termion::clear::All, termion::cursor::Goto(1,1)).unwrap();
    for i in 0..keys.len() {
        if i == position.section as usize {
            write!(screen, "\x1B[7m").unwrap();
            display_value(position, &keys[i], oc_plist.get_mut(&keys[i]), screen);
        } else {
            write!(screen, "{} >\n\r", &keys[i as usize]).unwrap();
        }
    }
}

fn display_value<W: Write>(position: &mut Position, key: &String, val: Option<&mut Value>, screen: &mut W) {
    for _ in 0..position.depth {
        write!(screen, "    ").unwrap();
    }
    match val.expect("Failed to unwrap Value") {
        Value::Array(v) => {
            write!(screen, "{}\x1B[0m >\n\r", key.as_str()).unwrap();
            for i in 0..v.len() {
                position.right();
                display_value(position, &i.to_string(), Some(&mut v[i]), screen);
                position.left();
            }
        }
        Value::Boolean(v) => match v {
            true => write!(screen, "\x1B[0;32m{}\x1B[0m: {}\n\r", key, v).unwrap(),
            false => write!(screen, "\x1B[0;31m{}\x1B[0m: {}\n\r", key, v).unwrap(),
        },
        Value::Data(v) => {
            write!(
                screen, "\x1B[0;33m{}\x1B[0m: {} | {}\n\r",
                key,
                hex::encode_upper(&*v),
                String::from_utf8_lossy(v)
            ).unwrap();
        }
        Value::Dictionary(v) => {
            write!(screen, "{}\x1B[0m >\n\r", key).unwrap();
            for key in v.keys().map(|s| s.to_string()).collect::<Vec<String>>() {
                position.right();
                display_value(position, &key, v.get_mut(&key), screen);
                position.left();
            }
        }
        Value::Integer(v) => write!(screen, "\x1B[0;34m{}\x1B[0m: {}\n\r", key, v).unwrap(),
        Value::String(v) => write!(screen, "{:>2}\x1B[0m: {}\n\r", key, v).unwrap(),
        _ => panic!("Can't handle this type"),
    }
}
