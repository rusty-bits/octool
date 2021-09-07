use std::{
    error::Error,
    fs,
    io::{Stdout, Write},
};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::RawTerminal;
use termion::{style, terminal_size};

use crate::draw::Position;

pub fn show_info(
    position: &Position,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<bool, Box<dyn Error>> {
    let mut showing_info = true;
    let rows = terminal_size()?.1;
    let mut row = 0;

    let contents = fs::read_to_string("tool_config_files/OpenCorePkg/Docs/Configuration.tex")?;

    let mut sub_search = "\\subsection{".to_string();

    match position.depth {
        //        0 => sub_search.push_str("Introduction}\\"),
        0 => (),
        1 => sub_search.push_str("Properties}\\"),
        2 | 3 => {
            sub_search.push_str(&position.sec_key[1]);
            sub_search.push_str(" Properties}\\");
        }
        _ => return Ok(false),
    }
    write!(stdout, "\r-\r\n")?;
    row += 1;

    let mut sec_search = "\\section{".to_string();
    sec_search.push_str(&position.sec_key[0]);

    let mut lines = contents.lines();

    loop {
        match lines.next() {
            Some(line) => {
                if line.contains(&sec_search) {
                    break;
                }
            }
            None => return Ok(false),
        }
    }

    if position.depth != 0 {
        loop {
            match lines.next() {
                Some(line) => {
                    if line.contains(&sub_search) {
                        break;
                    }
                }
                None => return Ok(false),
            }
        }

        let mut text_search = "texttt{".to_string();
        text_search.push_str(&position.sec_key[position.depth]);
        text_search.push_str(&"}\\");
        loop {
            match lines.next() {
                Some(line) => {
                    if line.contains(&text_search) {
                        break;
                    }
                }
                None => return Ok(false),
            }
        }
    }

    let mut itemize = 0;
    let mut hit_bottom = false;

    for line in lines {
        if row == rows {
            hit_bottom = true;
            write!(stdout, "{}more{} ...\x1B[G", style::Invert, style::Reset)?;
            stdout.flush()?;
            match std::io::stdin().keys().next().unwrap().unwrap() {
                Key::Char('q') | Key::Esc => {
                    hit_bottom = false;
                    showing_info = false;
                    break;
                }
                Key::Down => row -= 1,
                _ => row = 0,
            }
        }
        if line.contains("\\item") {
            if itemize == 0 {
                break;
            }
        }
        if line.contains("\\begin{") {
            itemize += 1;
            continue;
        }
        if line.contains("\\end{") {
            itemize -= 1;
            continue;
        }
        if line.contains("\\section{") {
            break;
        }
        if line.contains("\\subsection{Introduction}") {
            continue;
        }
        if line.contains("\\subsection{") {
            break;
        }
        write!(stdout, "\x1B[2K{}", parse_line(line))?;
        stdout.flush()?;
        row += 1;
    }
    if hit_bottom {
        write!(stdout, "{}{}  q to close\x1B[0J", style::Invert, "(END)")?;
        while showing_info {
            stdout.flush()?;
            match std::io::stdin().keys().next().unwrap().unwrap() {
                Key::Char('q') => {
                    showing_info = false;
                }
                _ => write!(stdout, "\x07")?,
            }
        }
    }
    Ok(showing_info)
}

fn parse_line(line: &str) -> String {
    let mut ret = String::new();
    let mut build_key = false;
    let mut build_name = false;
    let mut key = String::new();
    let mut name = String::new();
    for c in line.chars() {
        if build_key {
            match c {
                '{' => {
                    build_key = false;
                    build_name = true;
                    match key.as_str() {
                        "textbf" => ret.push_str("\x1B[1m"),
                        "emph" => ret.push_str("\x1B[7m"),
                        "texttt" => ret.push_str("\x1B[4m"),
                        _ => (),
                    };
                    key.clear();
                }
                ' ' => {
                    build_key = false;
                    match key.as_str() {
                        "textbackslash" => ret.push('\\'),
                        "item" => ret.push_str("• "),
                        "" => ret.push(' '),
                        _ => (),
                    }
                    key.clear();
                }
                '_' | '^' => {
                    build_key = false;
                    ret.push(c);
                    key.clear();
                }
                _ => key.push(c),
            }
        } else if build_name {
            match c {
                '}' => {
                    build_name = false;
                    ret.push_str(&name);
                    ret.push_str("\x1B[0m");
                    name.clear();
                }
                '\\' => {
                    ret.push_str(&name);
                    name.clear();
                    build_key = true;
                }
                _ => name.push(c),
            }
        } else {
            match c {
                '\\' => build_key = true,
                _ => ret.push(c),
            }
        }
    }
    if key == "tightlist" {
        ret.clear();
    } else {
        ret.push_str("\r\n");
    }

    ret
}
