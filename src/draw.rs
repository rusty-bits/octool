use plist::Value;

use std::error::Error;
use std::io::{Stdout, Write};

use crate::init::Settings;
use crate::res::{self, Resources};

use crossterm::terminal::size;

/// Redraws the plist on the screen
/// Draws the Footer first, in case it needs to be overwritten
/// Draws the plist next with current selection expanded
/// This allows the currently highlighted item info to be obtained
/// so any special comments can be included in the Header
/// which is drawn last
pub fn update_screen(
    settings: &mut Settings,
    resources: &Resources,
    stdout: &mut Stdout,
) -> Result<(), Box<dyn Error>> {
    let plist = &resources.config_plist;
    let screen_rows: i32 = size().unwrap().1.into();
    settings.can_expand = false;
    //    let bgc = &settings.bg_col.clone();
    let bgc = "\x1b[0m";

    // draw footer first, in case we need to write over it
    write!(stdout, "\x1B[{}H", screen_rows - 1)?;
    write!(
        stdout,
        " {inv}D{res}/{inv}^x{res}cut {inv}^c{res}op{inv}y{res} {inv}^v{res}/{inv}p{res}aste   {inv}f{res}ind {inv}n{res}ext   \
        {inv}a{res}dd {inv}d{res}el   {inv}M{res}erge {inv}P{res}urge {inv}r{res}eset   {inv}O{res}rder(Kernel>Add)\x1B[0K\r\n {inv}s{res}ave\
        +validate {inv}q{res}uit   {inv}G{res}o build EFI   {inv}K{res}ey     {inv}{red} {grn} {res}boolean {inv}{mag} {res}data {inv}{blu} \
        {res}integer {inv} {res}string\x1B[0K",
        inv = "\x1b[7m",
//        res = &settings.bg_col,
        res = "\x1b[0m",
        grn = "\x1b[32m",
        red = "\x1b[31m",
        mag = "\x1b[35m",
        blu = "\x1b[34m",
    )?;

    // jump under header, draw plist
    write!(stdout, "\x1B[3H")?;
    let mut row_being_drawn = 4;
    let list = plist.as_dictionary().unwrap();
    let keys: Vec<String> = list.keys().map(|s| s.to_string()).collect();
    for (i, k) in keys.iter().enumerate() {
        if row_being_drawn < screen_rows {
            row_being_drawn += display_value(
                k,
                None,
                settings,
                resources,
                list.get(k).unwrap(),
                stdout,
                i,
                0,
            )
            .unwrap();
        }
    }

    #[cfg(debug_assertions)]
    write!(
        stdout,
        "debug-> {:?} {} {:?} array? {} {:?}",
        settings.sec_length, settings.depth, settings.sec_num, settings.inside_an_array, settings.sec_key
    )?;

    let mut blanks = screen_rows - row_being_drawn - 1;
    if blanks < 0 {
        blanks = 0;
    }

    // clear rows up to footer
    write!(stdout, "{}", "\r\n\x1B[0K".repeat(blanks as usize))?;

    // lastly draw the header
    let mut info = String::new();
    settings.res_name(&mut info);
    if info.len() > 20 {
        info = info[0..17].to_string();
        info.push_str("...");
    }
    write!(
        stdout,
        "\x1b[1;{}H\x1b[2KOC \x1b[7mV{}ersion {}",
        (size().unwrap().0 - 16).to_string(),
        //        &settings.bg_col,
        "\x1b[0m",
        settings.oc_build_version,
    )
    .unwrap();
    write!(
        stdout,
        "\x1B[H{}{}   \x1B[0;7mi{}nfo for {}{}{}\r\n\x1B[0K",
        "\x1b[32m", &settings.config_file_name, bgc, "\x1b[4m", &info, bgc,
    )
    .unwrap();
    if settings.depth > 0 {
        write!(stdout, "  \x1B[7mleft{} collapse", bgc).unwrap();
    }
    write!(stdout, "{}", settings.item_instructions,).unwrap();
    if settings.depth == 2 {
        if settings.is_resource() {
            write!(stdout, "  \x1B[7mspace{} toggle", bgc).unwrap();
        }
    }
    if settings.find_string.len() > 0 {
        write!(
            stdout,
            "  \x1B[7mn{} jump to next {}{}{}",
            bgc, "\x1b[4m", settings.find_string, bgc,
        )
        .unwrap();
    }
    if settings.held_key.len() > 0 {
        write!(
            stdout,
            "  {}p{}aste {}{}{}",
            "\x1b[7m", bgc, "\x1b[4m", settings.held_key, bgc,
        )
        .unwrap();
    }
    write!(stdout, "\r\n\x1B[2K\x1B8",).unwrap();
    Ok(())
}

fn display_value(
    key: &String,
    key_color: Option<bool>,
    settings: &mut Settings,
    resources: &Resources,
    plist_value: &Value,
    stdout: &mut Stdout,
    item_num: usize,
    display_depth: usize,
) -> Result<i32, Box<dyn Error>> {
    let mut live_item = false;
    let mut selected_item = false;
    let mut save_curs_pos = String::new();
    let mut key_style = String::new();
    let mut pre_key = '>';
    let mut row = 1;
    //    let bgc = &settings.bg_col.clone();
    let bgc = "\x1b[0m";
    write!(stdout, "\r\n{}\x1B[0K", "    ".repeat(display_depth))?; // indent to section and clear rest of line
    if settings.sec_num[display_depth] == item_num {
        selected_item = true;
        settings.sec_key[display_depth] = key.to_string();
        key_style.push_str("\x1B[7m");
        // is current live item
        if display_depth == settings.depth {
            live_item = true;
            settings.item_instructions = match plist_value {
            Value::Array(_) | Value::Dictionary(_) => "  \x1B[7mright\x1B[0m expand",
            Value::Integer(_) | Value::String(_) => "  \x1B[7menter\x1B[0m/\x1B[7mtab\x1B[0m edit",
            Value::Boolean(_) => "  \x1B[7mspace\x1B[0m toggle",
            Value::Data(_) =>
                "  \x1B[7menter\x1B[0m/\x1B[7mtab\x1B[0m edit  \x1B[7mtab\x1B[0m switch hex/string",
            _ => "  XXXunknownXXX",
        }.to_string();
            save_curs_pos = "\x1B7".to_string(); // save cursor position for editing and info display
        }
    }
    match plist_value {
        Value::Array(v) => {
            if selected_item {
                settings.sec_length[display_depth + 1] = v.len();
            }
            if live_item {
                settings.can_expand = true;
                settings.live_value.clear();
            }
            if settings.depth > display_depth && settings.sec_num[display_depth] == item_num {
                // we have an open array
                pre_key = 'v';
                if settings.depth == display_depth + 1 {
                    // we are in the first level of an array
                    settings.inside_an_array = true;
                }
            }
            write!(
                stdout,
                "{} {}{}\x1B[0m  [{}]{} ",
                pre_key,
                key_style,
                key,
                v.len(),
                save_curs_pos
            )?;
            if settings.depth > display_depth && settings.sec_num[display_depth] == item_num {
                if v.len() == 0 {
                    write!(
                        stdout,
                        "\r\n\x1B[0K{}\x1B[7mempty{}{}",
                        "    ".repeat(display_depth + 1),
                        bgc,
                        save_curs_pos
                    )
                    .unwrap();
                    row += 1;
                } else {
                    let mut key = String::new();
                    for i in 0..v.len() {
                        let color = get_array_key(&mut key, &v[i], i);
                        row += display_value(
                            &key,
                            color,
                            settings,
                            resources,
                            &v[i],
                            stdout,
                            i,
                            display_depth + 1,
                        )?;
                    }
                }
            }
        }
        Value::Boolean(v) => {
            match v {
                true => write!(
                    stdout,
                    "{}\x1b[32m{}{}: {}{}",
                    key_style, key, bgc, save_curs_pos, v
                )
                .unwrap(),
                false => write!(
                    stdout,
                    "{}\x1b[31m{}{}: {}{}",
                    key_style, key, bgc, save_curs_pos, v
                )
                .unwrap(),
            };
            if live_item {
                settings.live_value = v.to_string();
            }
        }
        Value::Data(v) => {
            write!(
                stdout,
                "{}\x1b[35m{}{}: <{}{}> | \"{}\"\x1B[0K",
                key_style,
                key,
                bgc,
                save_curs_pos,
                hex_str_with_style(hex::encode(&*v)),
                get_lossy_string(v),
            )?;
            if live_item {
                settings.live_value = hex::encode(&*v).to_string();
            }
        }
        Value::Dictionary(v) => {
            if selected_item {
                settings.sec_length[display_depth + 1] = v.keys().len();
            }
            if live_item {
                settings.can_expand = true;
                settings.live_value.clear();
            }
            if settings.depth > display_depth && settings.sec_num[display_depth] == item_num {
                pre_key = 'v';
            }
            write!(
                stdout,
                "{} {}{}{}{} {} [{}]{} ",
                pre_key,
                key_style,
                match key_color {
                    Some(true) => "\x1b[32m",
                    Some(false) => "\x1b[31m",
                    None => "",
                },
                key,
                bgc,
                if display_depth == 2 {
                    let mut sec_sub = settings.sec_key[0].clone();
                    sec_sub.push_str(&settings.sec_key[1]);
                    if settings.resource_sections.contains(&sec_sub) {
                        match res::res_version(settings, &resources, &key) {
                            Some(s) => s,
                            None => " \x1b[33m∆\x1b[0m ".to_string(),
                        }
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                },
                v.len(),
                save_curs_pos
            )
            .unwrap();
            if settings.depth > display_depth && settings.sec_num[display_depth] == item_num {
                if v.keys().len() == 0 {
                    write!(
                        stdout,
                        "\r\n\x1B[0K{}\x1B[7mempty{}{}",
                        "    ".repeat(display_depth + 1),
                        bgc,
                        save_curs_pos
                    )
                    .unwrap();
                    row += 1;
                } else {
                    let keys: Vec<String> = v.keys().map(|s| s.to_string()).collect();
                    for (i, k) in keys.iter().enumerate() {
                        row += display_value(
                            &k,
                            None,
                            settings,
                            resources,
                            v.get(&k).unwrap(),
                            stdout,
                            i,
                            display_depth + 1,
                        )?;
                    }
                }
            }
        }
        Value::Integer(v) => {
            write!(
                stdout,
                "{}\x1b[34m{}{}: {}{}",
                key_style, key, bgc, save_curs_pos, v
            )?;
            if live_item {
                settings.live_value = v.to_string();
            }
        }
        Value::String(v) => {
            write!(
                stdout,
                "{}{:>2}{}: {}{}",
                key_style, key, bgc, save_curs_pos, v
            )?;
            if live_item {
                settings.live_value = v.to_string();
            }
        }
        _ => panic!("Can't handle this type"),
    }
    Ok(row)
}

pub fn get_lossy_string(v: &Vec<u8>) -> String {
    let mut tmp = String::new();
    for c in v {
        if c < &32 || c > &126 {
            tmp.push('\u{fffd}');
        } else {
            tmp.push(*c as char);
        }
    }
    tmp
}

fn get_array_key(key: &mut String, v: &plist::Value, i: usize) -> Option<bool> {
    match v {
        Value::Dictionary(d) => {
            for k in ["Path", "BundlePath", "Name", "Comment"] {
                if d.contains_key(k) {
                    *key = d.get(k).unwrap().clone().into_string().unwrap();
                    break; // stop after first match
                }
            }

            if key.len() == 0 {
                *key = i.to_string();
            }
            match d.get("Enabled") {
                Some(Value::Boolean(b)) => {
                    if *b {
                        return Some(true);
                    } else {
                        return Some(false);
                    }
                }
                _ => (),
            }
        }
        _ => *key = i.to_string(),
    }
    None
}

pub fn hex_str_with_style(v: String) -> String {
    let mut hex_u = String::new();
    let mut col = v.len() % 2;
    for c in v.chars() {
        if col > 1 {
            hex_u.push_str("\x1b[35m");
            hex_u.push(c);
            hex_u.push_str("\x1b[0m");
        } else {
            hex_u.push(c);
        }
        col += 1;
        if col > 3 {
            col = 0;
        };
    }
    hex_u
}
