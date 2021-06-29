extern crate hex;
extern crate plist;
extern crate termion;

use plist::{Dictionary, Value};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::*;

use std::env;
use std::io::{stdin, stdout, Write};

struct Position {
    section: [usize; 5],
    sec_length: [usize; 5],
    depth: usize,
    can_expand: bool,
}

impl Position {
    fn up(&mut self) {
        if self.section[self.depth] > 0 {
            self.section[self.depth] -= 1;
        }
    }

    fn down(&mut self) {
        if self.section[self.depth] < self.sec_length[self.depth] - 1 {
            self.section[self.depth] += 1;
        }
    }

    fn left(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
        }
    }

    fn right(&mut self) {
        if self.can_expand {
            self.depth += 1;
            self.section[self.depth] = 0;
        }
    }
}

fn main() {
    let file = env::args()
        .nth(1)
        .unwrap_or("INPUT/Sample.plist".to_string());

    let mut list =
        Value::from_file(&file).expect(format!("Didn't find plist at {}", file).as_str());
    let oc_plist = list
        .as_dictionary_mut()
        .expect(format!("Didn't find Dictionary in {}", file).as_str());

    let mut position = Position {
        section: [0; 5],
        sec_length: [oc_plist.keys().len(), 0, 0, 0, 0],
        depth: 0,
        can_expand: true,
    };

    let stdin = stdin();
    let mut screen = AlternateScreen::from(stdout().into_raw_mode().unwrap());
    write!(screen, "{}", termion::cursor::Hide).unwrap();

    draw_screen(&mut position, oc_plist, &mut screen);

    for c in stdin.keys() {
        match c.unwrap() {
            Key::Char('q') => break,
            Key::Down => {
                position.down();
                draw_screen(&mut position, oc_plist, &mut screen);
            }
            Key::Up => {
                position.up();
                draw_screen(&mut position, oc_plist, &mut screen);
            }
            Key::Right => {
                position.right();
                draw_screen(&mut position, oc_plist, &mut screen);
            }
            Key::Left => {
                position.left();
                draw_screen(&mut position, oc_plist, &mut screen);
            }
            _ => {}
        }
        screen.flush().unwrap();
    }
    write!(screen, "{}", termion::cursor::Show).unwrap();
    let _e = list.to_file_xml("test1");
}

fn draw_screen<W: Write>(position: &mut Position, list: &mut Dictionary, screen: &mut W) {
    write!(
        screen,
        "{}{}{}",
        termion::clear::All,
        termion::cursor::Goto(1, 1),
        "Hello!\r\n"
    )
    .unwrap();
    let keys: Vec<String> = list.keys().map(|s| s.to_string()).collect();
    for (i, k) in keys.iter().enumerate() {
        display_section(k, position, list.get_mut(k).unwrap(), screen, i, 0);
    }
    screen.flush().unwrap();
}

fn display_section<W: Write>(
    key: &String,
    position: &mut Position,
    oc_plist: &mut Value,
    screen: &mut W,
    item: usize,
    d: usize,
) {
    let mut live_item = false;
    for _ in 0..d {
        write!(screen, "    ").unwrap();
    }
    if position.section[d] == item {
        write!(screen, "\x1B[7m").unwrap();
        if d == position.depth {
            // current live item
            live_item = true;
            position.can_expand = false;
        }
    }
    match oc_plist {
        Value::Array(v) => {
            if live_item {
                position.can_expand = true;
                position.sec_length[d + 1] = v.len();
                if v.len() == 0 {
                    position.can_expand = false;
                }
            }
            write!(screen, "{}\x1B[0m >\n\r", key).unwrap();
            if position.depth > d && position.section[d] == item {
                for i in 0..v.len() {
                    display_section(&i.to_string(), position, &mut v[i], screen, i, d + 1);
                }
            }
        }
        Value::Boolean(v) => match v {
            true => write!(screen, "\x1B[32m{}\x1B[0m: {}\n\r", key, v).unwrap(),
            false => write!(screen, "\x1B[31m{}\x1B[0m: {}\n\r", key, v).unwrap(),
        },
        Value::Data(v) => {
            write!(
                screen,
                "\x1B[33m{}\x1B[0m: 0x{} | {}\n\r",
                key,
                hex::encode_upper(&*v),
                String::from_utf8_lossy(v)
            )
            .unwrap();
        }
        Value::Dictionary(v) => {
            if live_item {
                position.can_expand = true;
                position.sec_length[d + 1] = v.keys().len();
                if v.keys().len() == 0 {
                    position.can_expand = false;
                }
            }
            write!(screen, "{}\x1B[0m > \r\n", key).unwrap();
            let keys: Vec<String> = v.keys().map(|s| s.to_string()).collect();
            if position.depth > d && position.section[d] == item {
                for (i, k) in keys.iter().enumerate() {
                    display_section(&k, position, v.get_mut(&k).unwrap(), screen, i, d + 1);
                }
            }
        }
        Value::Integer(v) => write!(screen, "\x1B[34m{}\x1B[0m: {}\n\r", key, v).unwrap(),
        Value::String(v) => {
            write!(screen, "{:>2}\x1B[0m: {}\n\r", key, v).unwrap();
        }
        _ => panic!("Can't handle this type"),
    }
}
