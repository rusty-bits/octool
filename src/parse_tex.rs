use std::{
    error::Error,
    fs,
    io::{Stdout, Write},
};

use crossterm::event::KeyCode;
use crossterm::terminal::size;

use crate::{draw::Settings, edit::read_key, res::Resources};

/// Read through the Configuration.tex and display the info for the highlighted plist item
///
/// TODO: keep highlighted item on screen so it can be edited while looking at definition
/// TODO: display info of NVRAM variables
pub fn show_info(
    resources: &Resources,
    settings: &Settings,
    gather_valid: bool,
    valid_values: &mut Vec<String>,
    stdout: &mut Stdout,
) -> Result<bool, Box<dyn Error>> {
    let mut showing_info = true;
    let mut align = false;
    let rows = size()?.1;
    let mut row = 0;
    let mut bg_col = settings.bg_col_info.clone();
    if settings.depth == 0 {
        bg_col = "\x1b[0m".to_string();
    }

    let tex_path = &resources
        .open_core_source_path
        .join("Docs/Configuration.tex");
    let contents = fs::read_to_string(tex_path)?;
    let mut result = vec![];

    let mut sub_search = "\\subsection{".to_string();

    match settings.depth {
        //        0 => sub_search.push_str("Introduction}\\"),
        0 => (),
        1 => sub_search.push_str("Properties}\\"),
        2 | 3 => match settings.sec_key[0].as_str() {
            "NVRAM" => sub_search.push_str("Introduction}"),
            "DeviceProperties" => sub_search.push_str("Common"),
            "Misc" => {
                if settings.depth < 3 {
                    sub_search.push_str(&settings.sec_key[1]);
                    sub_search.push_str(" Properties}\\");
                } else {
                    sub_search.push_str("Entry Properties}\\");
                }
            }
            _ => {
                sub_search.push_str(&settings.sec_key[1]);
                sub_search.push_str(" Properties}\\");
            }
        },
        _ => return Ok(false),
    }
    if !gather_valid {
        //        write!(stdout, "{}\r-\r\n", bg_col)?;
        write!(
            stdout,
            "{}\x1b[4m{}\x1b8\r\x1b[4m{}\r\n{}",
            &settings.live_value,
            " ".repeat(size()?.0.into()),
            "    ".repeat(settings.depth),
            bg_col
        )?;
    }
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
        if &settings.sec_key[0] == "NVRAM" && settings.depth > 1 {
            text_search.push_str(&settings.sec_key[2]);
            if settings.depth == 3 {
                text_search.push(':');
                text_search.push_str(&settings.sec_key[settings.depth]);
            }
        } else if &settings.sec_key[0] == "DeviceProperties" && settings.depth == 3 {
            text_search.push_str(&settings.sec_key[settings.depth]);
        } else {
            text_search.push_str(&settings.sec_key[settings.depth]);
            text_search.push_str(&"}\\");
        }
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
    let mut enumerate = 0;
    let mut hit_bottom = false;
    let mut columns = 0;
    let mut lines_between_valid = 0;

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
        if line.contains("\\begin{align*}") {
            align = true;
            continue;
        }
        if line.contains("\\end{align*}}") {
            align = false;
            continue;
        }
        // cheap hack to keep track of being in a list
        if line.contains("\\begin{itemize}") {
            itemize += 1;
            continue;
        }
        if line.contains("\\begin{enumerate}") {
            enumerate += 1;
            continue;
        }
        if line.contains("\\mbox") {
            continue;
        }
        //        if line.contains("\\begin{") {
        //            continue;
        //        }
        if line.contains("\\end{tabular}") {
            columns = 0;
            continue;
        }
        if line.contains("\\end{itemize}") {
            itemize -= 1;
            continue;
        }
        if line.contains("\\end{enumerate}") {
            enumerate -= 1;
            continue;
        }
        if line.contains("\\end{") {
            continue;
        }
        if line.contains("\\item") && (itemize == 0 && enumerate == 0) {
            break;
        }
        if line.contains("\\subsection{") || line.contains("\\section{") {
            break;
        }
        let parsed_line = parse_line(line, columns, align, gather_valid, &bg_col);
        if gather_valid {
            // gather list items to display when editing a string or integer
            if itemize > 0 {
                // we are inside an itemize bracket
                if line.contains("---") {
                    if lines_between_valid < 10 {
                        valid_values.push(parsed_line);
                    }
                }
            } else {
                // stop gathering if there has been a big break
                if valid_values.len() > 0 {
                    lines_between_valid += 1;
                }
            }
        } else {
            if parsed_line.len() != 0 {
                result.push(format!("\x1B[0K{}", parsed_line));
            }
        }
    }
    if !gather_valid {
        // show config tex info if not already in edit mode on a string or integer
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
                        write!(stdout, "{}END{} ... 'q' to quit\x1B[G", "\x1b[7m", bg_col,)?;
                    } else {
                        write!(stdout, "\x1b[7mmore{} ...\x1B[G", bg_col)?;
                    }
                    stdout.flush()?;
                    match read_key().unwrap().0 {
                        KeyCode::Char('q') | KeyCode::Char('i') | KeyCode::Esc => {
                            hit_bottom = false;
                            showing_info = false;
                            valid_values.push("ok".to_string()); // don't display no info found
                            break;
                        }
                        KeyCode::Down => {
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
                        KeyCode::Up => {
                            row = 0;
                            if start > 0 {
                                start -= 1;
                            }
                            write!(stdout, "\x1B[1H")?;
                            break;
                        }
                        KeyCode::Char('b') => {
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
    }
    stdout.flush()?;
    Ok(showing_info)
}

/// Go through line 1 character at a time to apply .tex formatting
///
/// TODO: pass back attributes so formatting/mode can exist for more than 1 line
///
fn parse_line(line: &str, columns: i32, align: bool, gather_valid: bool, bg_col: &str) -> String {
    let mut ret = String::new();
    let mut build_key = false;
    let mut key = String::new();
    let width = size().unwrap().0 as i32;
    let mut col_width = 0;
    if columns > 0 {
        col_width = width / (columns + 1);
    }
    let mut ignore = false;
    let mut col_contents_len = 0;
    for c in line.chars() {
        if build_key {
            match c {
                // end of key
                '{' | '[' => {
                    build_key = false;
                    //                    build_name = true;
                    if !gather_valid {
                        match key.as_str() {
                            "text" => ret.push_str(bg_col),
                            "textbf" => ret.push_str("\x1B[1m"),
                            "emph" => ret.push_str("\x1B[7m"),
                            "texttt" => ret.push_str("\x1B[4m"),
                            //                            "href" => ret.push_str("\x1B[34m"),
                            //                            "hyperref" => ret.push_str("\x1B[4m"),
                            //                            "hyperlink" => build_key = true, // ignore link text
                            _ => ignore = true,
                        };
                    }
                    if &key != "href" {
                        // hold href key to insert space after it
                        key.clear();
                    }
                }
                // end of key - may be special character or formatting
                ' ' | ',' | '(' | ')' | '\\' | '0'..='9' | '$' | '&' => {
                    build_key = false;
                    match key.as_str() {
                        "kappa" => ret.push('\u{03f0}'),
                        "lambda" => ret.push('\u{03bb}'),
                        "mu" => ret.push('\u{03bc}'),
                        "alpha" => ret.push('\u{03b1}'),
                        "beta" => ret.push('\u{03b2}'),
                        "gamma" => ret.push('\u{03b3}'),
                        "leq" => ret.push('\u{2264}'),
                        "cdot" => ret.push('\u{00b7}'),
                        "in" => ret.push('\u{220a}'),
                        "infty" => ret.push('\u{221e}'),
                        "textbackslash" => ret.push('\\'),
                        "item" => {
                            if !gather_valid {
                                ret.push_str("• ");
                            }
                        }
                        "" => ret.push(' '),
                        _ => (),
                    }
                    col_contents_len += 1;
                    if c == ',' || c == '(' || c == ')' || (c >= '0' && c <= '9') || c == '$' {
                        ret.push(c);
                    }
                    if c == '\\' {
                        if key.len() > 0 {
                            // check for double \
                            build_key = true; // or start of new key
                        }
                    }
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
                '}' | ']' => {
                    if !ignore {
                        if !gather_valid {
                            ret.push_str(bg_col);
                            if &key == "href" {
                                ret.push(' ');
                                key.clear();
                            } else if c == ']' {
                                ret.push(']');
                            }
                        }
                    }
                    ignore = false;
                }
                '{' => {
                    if !gather_valid {
                        ret.push_str("\x1B[4m");
                    }
                }
                '&' => {
                    if columns > 0 {
                        let fill = col_width - col_contents_len - 1;
                        if fill > 0 {
                            ret.push_str(&" ".repeat(fill as usize));
                        }
                        ret.push_str("|");
                        col_contents_len = 0;
                    } else {
                        if !align {
                            ret.push('&');
                        }
                    }
                }
                _ => {
                    if !ignore {
                        ret.push(c);
                        col_contents_len += 1;
                    }
                }
            }
        }
    }
    if key.len() > 0 {
        match key.as_str() {
            // repetitive - TODO: move to a function
            "kappa" => ret.push('\u{03f0}'),
            "lambda" => ret.push('\u{03bb}'),
            "mu" => ret.push('\u{03bc}'),
            "alpha" => ret.push('\u{03b1}'),
            "beta" => ret.push('\u{03b2}'),
            "gamma" => ret.push('\u{03b3}'),
            "leq" => ret.push('\u{2264}'),
            "cdot" => ret.push('\u{00b7}'),
            "in" => ret.push('\u{220a}'),
            "infty" => ret.push('\u{221e}'),
            "textbackslash" => ret.push('\\'),
            _ => (),
        }
    }
    if !gather_valid {
        if key == "tightlist" {
            // ignore
            ret.clear();
        } else {
            if key == "hline" {
                ret.push_str(&"-".repeat(width as usize - 4));
            }
            ret.push_str("\r\n");
        }
    }

    ret
}
