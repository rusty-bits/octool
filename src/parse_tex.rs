use std::{
    error::Error,
    fs,
    io::{Stdout, Write},
};

use termion::event::Key;
use termion::input::TermRead;
use termion::raw::RawTerminal;
use termion::{style, terminal_size};

use crate::{draw::Settings, res::Resources};

/// Read through the Configuration.tex and display the info for the highlighted plist item
///
/// TODO: parse tables instead of just printing &*&&&*&*&
/// TODO: keep highlighted item on screen so it can be edited while looking at definition
/// TODO: display info of NVRAM variables
pub fn show_info(
    resources: &Resources,
    settings: &Settings,
    stdout: &mut RawTerminal<Stdout>,
) -> Result<bool, Box<dyn Error>> {
    let mut showing_info = true;
    let rows = terminal_size()?.1;
    let mut row = 0;

    let tex_path = &resources.open_core_source_path.join("Docs/Configuration.tex");
    let contents = fs::read_to_string(tex_path)?;
    let mut result = vec![];

    let mut sub_search = "\\subsection{".to_string();

    match settings.depth {
        //        0 => sub_search.push_str("Introduction}\\"),
        0 => (),
        1 => sub_search.push_str("Properties}\\"),
        2 | 3 => {
            sub_search.push_str(&settings.sec_key[1]);
            sub_search.push_str(" Properties}\\");
        }
        _ => return Ok(false),
    }
    write!(stdout, "\r-\r\n")?;
    row += 1;

    let mut sec_search = "\\section{".to_string();
    sec_search.push_str(&settings.sec_key[0]);

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

    if settings.depth != 0 {
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
        text_search.push_str(&settings.sec_key[settings.depth]);
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
    let mut columns = 0;

    for line in lines {
        if line.contains("\\subsection{Introduction}") {
            continue;
        }
        if line.contains("\\begin{tabular") {
            for c in line.chars() {
                if c == 'c' {
                    columns += 1;
                };
            }
            continue;
        }
        if line.contains("\\begin{item") || line.contains("\\begin{enum") {
            itemize += 1;
            continue;
        }
        if line.contains("\\begin{") {
            continue;
        }
        if line.contains("\\end{tabular}") {
            columns = 0;
            continue;
        }
        if line.contains("\\end{item") || line.contains("\\end{enum") {
            itemize -= 1;
            continue;
        }
        if line.contains("\\end{") {
            continue;
        }
        if line.contains("\\item") && itemize == 0 {
            break;
        }
        if line.contains("\\subsection{") || line.contains("\\section{") {
            break;
        }
        let parsed_line = parse_line(line, columns);
        if parsed_line.len() != 0 {
            result.push(format!("\x1B[0K{}", parsed_line));
        }
    }
    let mut start = 0;
    loop {
        for i in start..result.len() {
            write!(stdout, "{}", result[i])?;
            row += 1;
            if row == rows {
                if row == result.len() as u16 + 1 {
                    break;
                } else {
                    hit_bottom = true;
                }
                if i == result.len() - 1 {
                    write!(
                        stdout,
                        "{}END{} ... 'q' to quit\x1B[G",
                        style::Invert,
                        style::Reset,
                    )?;
                } else {
                    write!(stdout, "{}more{} ...\x1B[G", style::Invert, style::Reset,)?;
                }
                stdout.flush()?;
                match std::io::stdin().keys().next().unwrap().unwrap() {
                    Key::Char('q') | Key::Char('i') | Key::Esc => {
                        hit_bottom = false;
                        showing_info = false;
                        break;
                    }
                    Key::Down => {
                        if i < result.len() - 1 {
                            row -= 1;
                            start += 1;
                            if start > result.len() - rows as usize {
                                start = result.len() - rows as usize;
                            }
                        } else {
                            row = 0;
                        }
                    }
                    Key::Up => {
                        row = 0;
                        if start > 0 {
                            start -= 1;
                        }
                        write!(stdout, "\x1B[1H")?;
                        break;
                    }
                    Key::Char('b') => {
                        if start > rows as usize {
                            start -= rows as usize;
                        } else {
                            start = 0;
                        }
                        row = 0;
                        write!(stdout, "\x1B[1H")?;
                        break;
                    }
                    _ => {
                        row = 0;
                        if i < result.len() - 1 {
                            start += rows as usize;
                            if start > result.len() - rows as usize {
                                start = result.len() - rows as usize;
                            }
                        }
                        break;
                    }
                }
            }
        }
        if !hit_bottom {
            break;
        }
    }
    stdout.flush()?;
    Ok(showing_info)
}

/// Go through line 1 character at a time to apply .tex formatting
/// TODO: pass back attributes so formatting/mode can exist for more than 1 line
///
fn parse_line(line: &str, columns: i32) -> String {
    let mut ret = String::new();
    let mut build_key = false;
    let mut key = String::new();
    let width = terminal_size().unwrap().0 as i32;
    let mut col_width = 0;
    if columns > 0 {
        col_width = width / (columns + 1);
    }
    let mut col_contents_len = 0;
    for c in line.chars() {
        if build_key {
            match c {
                // end of key
                '{' => {
                    build_key = false;
                    //                    build_name = true;
                    match key.as_str() {
                        "textbf" => ret.push_str("\x1B[1m"),
                        "emph" => ret.push_str("\x1B[7m"),
                        "texttt" => ret.push_str("\x1B[4m"),
                        "href" => ret.push_str("\x1B[34m"),
                        "hyperlink" => build_key = true, // ignore link text
                        _ => (),
                    };
                    key.clear();
                }
                // end of key - may be special character or formatting
                ' ' => {
                    build_key = false;
                    match key.as_str() {
                        "textbackslash" => ret.push('\\'),
                        "item" => ret.push_str("• "),
                        "" => ret.push(' '),
                        _ => (),
                    }
                    col_contents_len += 1;
                    key.clear();
                }
                // found escaped character
                '_' | '^' | '#' => {
                    build_key = false;
                    ret.push(c);
                    col_contents_len += 1;
                    key.clear();
                }
                _ => key.push(c),
            }
        } else {
            match c {
                '\\' => build_key = true,
                '}' => ret.push_str("\x1B[0m"),
                '{' => ret.push_str("\x1B[4m"),
                '&' => {
                    if columns > 0 {
                        let fill = col_width - col_contents_len - 1;
                        if fill > 0 {
                            ret.push_str(&" ".repeat(fill as usize));
                        }
                        ret.push_str("|");
                        col_contents_len = 0;
                    } else {
                        ret.push('&');
                    }
                }
                _ => {
                    ret.push(c);
                    col_contents_len += 1;
                }
            }
        }
    }
    if key == "tightlist" {
        // ignore
        ret.clear();
    } else {
        if key == "hline" {
            ret.push_str(&"-".repeat(width as usize - 4));
        }
        ret.push_str("\r\n");
    }

    ret
}
