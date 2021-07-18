use console::{style, Term};
use std::{fs, io::Write};

use crate::draw::Position;

pub fn show_info(position: &Position, term: &Term) {
    let contents = fs::read_to_string("resources/OpenCorePkg/Docs/Configuration.tex").unwrap();

    let mut sec_search = "\\section{".to_string();
    sec_search.push_str(&position.sec_key[0]);
    let mut sub_search = "\\subsection{".to_string();

    match position.depth {
        //        0 => sub_search.push_str("Introduction}\\"),
        0 => (),
        1 => sub_search.push_str("Properties}\\"),
        2 | 3 => {
            sub_search.push_str(&position.sec_key[1]);
            sub_search.push_str(" Properties}\\");
        }
        _ => return,
    }
    //    write!(&*term, "\x1B[1A\r{}", style("    ".repeat(position.depth)).underlined()).unwrap();
    write!(&*term, "\r\n").unwrap();

    let mut lines = contents.lines();

    loop {
        match lines.next() {
            Some(line) => {
                if line.contains(&sec_search) {
                    break;
                }
            }
            None => return,
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
                None => return,
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
                None => return,
            }
        }
    }

    let mut itemize = 0;

    for line in lines {
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
        if line.contains("\\subsection{") && !line.contains("\\subsection{Introduction}") {
            break;
        }
        write!(&*term, "\x1B[2K{}", parse_line(line)).unwrap();
    }
    write!(&*term, "{}\x1B[0K", style(" ".repeat(70)).underlined()).unwrap();
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
                    if key == "textbf" {
                        ret.push_str("\x1B[1m");
                    }
                    if key == "emph" {
                        ret.push_str("\x1B[7m");
                    }
                    if key == "texttt" {
                        ret.push_str("\x1B[4m");
                    }
                    key.clear();
                }
                ' ' => {
                    build_key = false;
                    if key == "textbackslash" {
                        ret.push('\\');
                    }
                    if key == "item" {
                        ret.push_str("+ ");
                    }
                    if key == "" {
                        ret.push(' ');
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
