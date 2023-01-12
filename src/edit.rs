use crate::draw;
use crate::edit;
use crate::init::Settings;
use crate::res::Resources;

use crossterm::event::KeyModifiers;
use plist::{Integer, Value};

use std::{
    error::Error,
    i64,
    io::{Stdout, Write},
};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode},
    style,
};

#[derive(Debug)]
pub struct Found {
    pub level: usize,
    pub section: [usize; 5],
    pub keys: Vec<String>,
}

impl Found {
    pub fn new() -> Found {
        Found {
            level: 0,
            section: [0; 5],
            keys: vec![],
        }
    }
}

pub fn read_key() -> crossterm::Result<(KeyCode, KeyModifiers)> {
    loop {
        if let Event::Key(key) = event::read()? {
            return Ok((key.code, key.modifiers));
        }
    }
}

/// find the current highlighted item in the given 'plist_val' plist
/// and place it into the settings. held_item and held_key
/// if 'first' is true get the first key of a dict or item 0 of an array
/// if 'preserve' is true do not clear the held values to their defaults
pub fn extract_value(
    settings: &mut Settings,
    mut plist_val: &Value,
    first: bool,
    preserve: bool,
) -> bool {
    let mut extracted = true;
    for i in 0..settings.depth {
        match plist_val {
            Value::Dictionary(d) => {
                plist_val = match d.get(&settings.sec_key[i]) {
                    Some(k) => k,
                    None => return false,
                }
            }
            Value::Array(a) => {
                plist_val = a.get(0).expect("No 0 element in Sample.plist");
            }
            _ => (),
        }
    }
    match plist_val {
        Value::Dictionary(d) => {
            let key = if first {
                d.keys().next().expect("Didn't get first key").clone()
            } else {
                settings.sec_key[settings.depth].clone()
            };
            match d.get(&key) {
                Some(v) => {
                    settings.held_item = Some(v.to_owned());
                    settings.held_key = key.to_owned();
                }
                None => extracted = false,
            };
        }
        Value::Array(a) => {
            let num = if first {
                0
            } else {
                settings.sec_num[settings.depth]
            };
            if num < a.len() {
                let mut ex_item = a.get(num).unwrap().to_owned();
                match ex_item.as_dictionary_mut() {
                    Some(d) => {
                        if !preserve {
                            for val in d.values_mut() {
                                match val {
                                    Value::String(_) => *val = Value::String("".to_string()),
                                    Value::Boolean(_) => *val = Value::Boolean(false),
                                    Value::Integer(_) => {
                                        *val = Value::Integer(plist::Integer::from(0))
                                    }
                                    Value::Data(_) => *val = Value::Data(Default::default()),
                                    _ => (),
                                }
                            }
                        }
                        settings.held_item = Some(ex_item);
                        settings.held_key = settings.sec_key[settings.depth].clone();
                    }
                    None => settings.held_item = Some(ex_item), // not a dict in an array, return element "num"
                }
            } else {
                extracted = false;
            }
        }
        _ => extracted = false, // not a dict or array
    }
    extracted
}

/// if 'add' is true,
/// place the settings.held_item into the given 'plist_val' plist at the highlighted location
/// if 'add' is false
/// delete the highlighted value from the given 'plist_val' plist and place it in the settings.held_item
///
pub fn add_delete_value(settings: &mut Settings, mut plist_val: &mut Value, add: bool) -> bool {
    let mut changed = false;
    for i in 0..settings.depth {
        match plist_val {
            Value::Dictionary(d) => {
                plist_val = match d.get_mut(&settings.sec_key[i]) {
                    Some(k) => k,
                    None => return false,
                }
            }
            Value::Array(a) => {
                plist_val = a.get_mut(settings.sec_num[i]).unwrap();
            }
            _ => (),
        }
    }
    match plist_val {
        Value::Dictionary(d) => {
            if add {
                if d.contains_key(&settings.held_key) {
                    settings
                        .held_key
                        .insert_str(settings.held_key.len(), "-copy");
                }
                match settings.held_item.clone() {
                    Some(item) => {
                        if d.insert(settings.held_key.to_owned(), item) == None {
                            changed = true;
                        }
                    }
                    None => (),
                }
            } else {
                settings.held_item = Some(d.remove(&settings.sec_key[settings.depth]).unwrap());
                settings.held_key = settings.sec_key[settings.depth].to_owned();
                changed = true;
            }
            d.sort_keys();
            if changed {
                settings.sec_num[settings.depth] =
                    d.keys().position(|k| k == &settings.held_key).unwrap_or(0);
            };
        }
        Value::Array(a) => {
            if add {
                match settings.held_item.clone() {
                    Some(item) => {
                        a.insert(settings.sec_num[settings.depth], item);
                        changed = true;
                    }
                    None => (),
                }
            } else {
                settings.held_item = Some(a.remove(settings.sec_num[settings.depth]));
                settings.held_key = settings.sec_key[settings.depth].to_owned();
                changed = true;
            }
        }
        _ => (),
    }
    changed
}

/// ask for a search string and give a scrollable list of locations to jump to in 'found'
/// if only 1 result is found, jump immediately
pub fn find(find_string: &str, resource: &plist::Value, found: &mut Vec<Found>) {
    if find_string.len() > 0 {
        let search = find_string.to_lowercase();
        let resource = resource.as_dictionary().unwrap();
        for (i, key) in resource.keys().enumerate() {
            let low_key = key.to_lowercase();
            if low_key.contains(&search) {
                found.push(Found {
                    level: 0,
                    section: [i, 0, 0, 0, 0],
                    keys: vec![key.to_owned()],
                });
            }
            match resource.get(key).unwrap().as_dictionary() {
                Some(sub) => {
                    for (j, s_key) in sub.keys().enumerate() {
                        let low_key = s_key.to_lowercase();
                        if low_key.contains(&search) {
                            found.push(Found {
                                level: 1,
                                section: [i, j, 0, 0, 0],
                                keys: vec![key.to_owned(), s_key.to_owned()],
                            });
                        }
                        let sub_sub = sub.get(s_key).unwrap();
                        match sub_sub {
                            plist::Value::Dictionary(d) => {
                                for (k, s_s_key) in d.keys().enumerate() {
                                    let low_key = s_s_key.to_lowercase();
                                    if low_key.contains(&search) {
                                        found.push(Found {
                                            level: 2,
                                            section: [i, j, k, 0, 0],
                                            keys: vec![
                                                key.to_owned(),
                                                s_key.to_owned(),
                                                s_s_key.to_owned(),
                                            ],
                                        });
                                    }
                                    match d.get(s_s_key).unwrap() {
                                        plist::Value::Dictionary(sub_d) => {
                                            for (l, sub_d_key) in sub_d.keys().enumerate() {
                                                let low_key = sub_d_key.to_lowercase();
                                                if low_key.contains(&search) {
                                                    found.push(Found {
                                                        level: 3,
                                                        section: [i, j, k, l, 0],
                                                        keys: vec![
                                                            key.to_owned(),
                                                            s_key.to_owned(),
                                                            s_s_key.to_owned(),
                                                            sub_d_key.to_owned(),
                                                        ],
                                                    });
                                                }
                                            }
                                        }
                                        _ => (),
                                    } // end match d
                                }
                            }
                            plist::Value::Array(a) => {
                                for (k, v) in a.iter().enumerate() {
                                    match v {
                                        plist::Value::Dictionary(d) => {
                                            for (l, s_s_key) in d.keys().enumerate() {
                                                let low_key = s_s_key.to_lowercase();
                                                if low_key.contains(&search) {
                                                    found.push(Found {
                                                        level: 3,
                                                        section: [i, j, k, l, 0],
                                                        keys: vec![
                                                            key.to_owned(),
                                                            s_key.to_owned(),
                                                            k.to_string(),
                                                            s_s_key.to_owned(),
                                                        ],
                                                    });
                                                }
                                            }
                                        }
                                        _ => (),
                                    } // end match v
                                }
                            }

                            _ => (),
                        } // end match sub_sub
                    }
                }
                _ => (),
            } // end match resource
        }
    }
}

/// add an item of a user selected type to the loaded config.plist as the highlighted
/// location.  If the highlighted location is inside a section that holds resources
/// e.g. Kexts, Drivers, etc. then give an option to insert a blank template made from
/// the format in the corresponding Sample.plist
/// setting auto_add to a resource name will automatically add that resource then return
pub fn add_item(
    mut settings: &mut Settings,
    resources: &mut Resources,
    auto_add: &str,
    stdout: &mut Stdout,
) {
    settings.modified = true;
    let mut selection = 1;
    let mut selection_adjust = 0;
    let mut item_types = Vec::<String>::new();
    let mut res_list = vec![];
    let mut res_type = "";
    let mut res_ext = "";
    if settings.is_resource() {
        match settings.sec_key[0].as_str() {
            "ACPI" => {
                res_ext = ".aml";
                res_type = "acpi";
            }
            "Kernel" => {
                res_ext = ".kext";
                res_type = "kext";
            }
            "Misc" => {
                res_ext = ".efi";
                res_type = "tool";
            }
            "UEFI" => {
                res_ext = ".efi";
                res_type = "driver";
            }
            _ => (),
        };
        for res in resources.resource_list.as_object().unwrap() {
            if res.0.contains(res_ext) {
                if res_ext == ".efi" {
                    if res.1["res_type"] == res_type {
                        res_list.push(res.0.clone());
                    }
                } else {
                    res_list.push(res.0.clone());
                }
            }
        }
        res_list.sort();
        let msg = format!("Select new {} from a list", res_type,);
        item_types.push(msg);
        selection_adjust += 1;
    }
    if settings.inside_an_array && settings.depth == 2 {
        let msg = format!(
            "New {} > {} template from Sample.plist",
            settings.sec_key[0], settings.sec_key[1]
        );
        selection_adjust += 1;
        item_types.push(msg);
    }
    for s in [
        "plist array",
        "plist boolean",
        "plist data",
        "plist dict",
        "plist integer",
        "plist string",
    ] {
        item_types.push(s.to_owned());
    }
    if auto_add.len() == 0 {
        write!(
            stdout,
            "\r\n\x1b[32mSelect type of item to add to plist:\x1b[0m\x1B[0K\r\n\x1b[0K\r\n{}",
            cursor::SavePosition,
        )
        .unwrap();
        loop {
            write!(stdout, "\x1B8").unwrap();
            for (i, item_type) in item_types.iter().enumerate() {
                if i == selection - 1 {
                    write!(stdout, "\x1B[7m").unwrap();
                }
                write!(stdout, "{}\x1B[0m\x1B[0K\r\n", item_type).unwrap();
            }
            write!(stdout, "\x1B[2K").unwrap();
            stdout.flush().unwrap();
            match read_key().unwrap().0 {
                KeyCode::Up => {
                    if selection > 1 {
                        selection -= 1;
                    }
                }
                KeyCode::Down => {
                    if selection < item_types.len() {
                        selection += 1;
                    }
                }
                KeyCode::Enter => break,
                KeyCode::Esc => {
                    selection = 0;
                    break;
                }
                _ => (),
            }
        }
        if selection == 0 {
            return;
        };
    } else {
        selection = 1;
        selection_adjust = 2;
    }
    //    if selection_adjust > 1 && selection < 3 {
    if selection <= selection_adjust {
        if selection == 1 && selection_adjust == 2 {
            let new_val_set;
            let mut selected_res = res_list[0].clone();
            if auto_add.len() == 0 {
                write!(
                    stdout,
                    "{}{}\r\x1B[2KSelect or edit {} to insert: {}\r\n\x1B[2K\x1B8",
                    cursor::RestorePosition,
                    cursor::Show,
                    res_type,
                    cursor::SavePosition,
                )
                .unwrap();
                new_val_set =
                    edit::edit_string(&mut selected_res, Some(&res_list), stdout).unwrap();
                write!(stdout, "{}", cursor::Hide).unwrap();
            } else {
                new_val_set = true;
                selected_res = auto_add.to_owned();
            }
            if !new_val_set {
                return;
            }
            if !extract_value(&mut settings, &resources.sample_plist, true, false) {
                return;
            }
            let item = settings.held_item.as_mut().unwrap();
            match res_type {
                "acpi" => {
                    item.as_dictionary_mut()
                        .unwrap()
                        .insert("Path".to_string(), plist::Value::String(selected_res));
                    item.as_dictionary_mut()
                        .unwrap()
                        .insert("Enabled".to_string(), plist::Value::Boolean(true));
                }
                "driver" => {
                    if settings.oc_build_version > "0.7.2".to_string() {
                        item.as_dictionary_mut()
                            .unwrap()
                            .insert("Path".to_string(), plist::Value::String(selected_res));
                        item.as_dictionary_mut()
                            .unwrap()
                            .insert("Enabled".to_string(), plist::Value::Boolean(true));
                    } else {
                        settings.held_item = Some(plist::Value::String(selected_res));
                    }
                }
                "kext" => {
                    let item = item.as_dictionary_mut().unwrap();
                    item.insert("Arch".to_string(), plist::Value::String("Any".to_string()));
                    item.insert(
                        "BundlePath".to_string(),
                        plist::Value::String(selected_res.clone()),
                    );
                    let mut ex_path = "Contents/MacOS/".to_string();
                    ex_path.push_str(selected_res.split('.').next().unwrap());
                    item.insert("ExecutablePath".to_string(), plist::Value::String(ex_path));
                    item.insert(
                        "PlistPath".to_string(),
                        plist::Value::String("Contents/Info.plist".to_string()),
                    );
                    item.insert("Enabled".to_string(), plist::Value::Boolean(true));
                }
                "tool" => {
                    let item = item.as_dictionary_mut().unwrap();
                    item.insert("Path".to_string(), plist::Value::String(selected_res));
                    item.insert(
                        "Flavour".to_string(),
                        plist::Value::String("Auto".to_string()),
                    );
                    item.insert("Enabled".to_string(), plist::Value::Boolean(true));
                }
                _ => (),
            };
        } else {
            if !extract_value(&mut settings, &resources.sample_plist, true, false) {
                return;
            }
        }
    } else {
        write!(
            stdout,
            "Enter key for new {} item: {}{}\x1B[0K\r\n\x1B[2K",
            item_types[selection - 1],
            cursor::SavePosition,
            cursor::Show
        )
        .unwrap();
        stdout.flush().unwrap();
        let mut key = String::new();
        edit_string(&mut key, None, stdout).unwrap();
        settings.held_key = String::from(key.trim());
        settings.held_item = Some(match selection - selection_adjust {
            1 => plist::Value::Array(vec![]),
            2 => false.into(),
            3 => plist::Value::Data(vec![]),
            4 => plist::Value::Dictionary(plist::Dictionary::default()),
            5 => 0.into(),
            6 => plist::Value::String("".to_string()),
            _ => panic!("How did you select this?"),
        });
        write!(stdout, "{}", cursor::Hide).unwrap();
    }
    if add_delete_value(settings, &mut resources.config_plist, true) {
        settings.add();
    }
}

/// edit the highlighted value in the loaded config.plist
///
/// ```
/// space_pressed: bool // was space pressed to get here, if so, toggle value
/// edit_key: bool // edit the key name of the field, not the value in the field
///
/// ```
pub fn edit_value(
    settings: &mut Settings,
    mut val: &mut Value,
    valid_values: Option<&Vec<String>>,
    stdout: &mut Stdout,
    space_pressed: bool,
    edit_key: bool,
) -> Result<(), Box<dyn Error>> {
    write!(
        stdout,
        "{}\x1B[H\x1B[0K\r\n\x1B[0K {inv}enter{res} save changes   {inv}esc{res} cancel changes\x1b[H",
        cursor::Show,
        inv = style::Attribute::Reverse,
        res = style::Attribute::Reset,
    )?;
    let mut search_depth = settings.depth + 1;
    if edit_key {
        search_depth -= 1;
    }
    for i in 0..search_depth {
        match val {
            Value::Dictionary(d) => {
                let key = d.keys().map(|s| s.to_string()).collect::<Vec<String>>()
                    [settings.sec_num[i]]
                    .clone();
                val = match d.get_mut(&key) {
                    Some(k) => k,
                    None => panic!("failure to get Value from Dict"),
                };
            }
            Value::Array(a) => {
                val = a.get_mut(settings.sec_num[i]).unwrap();
            }
            _ => (),
        }
    }

    if space_pressed {
        match val {
            Value::Boolean(b) => *b = !*b,
            Value::Dictionary(d) => match d.get_mut("Enabled") {
                Some(Value::Boolean(b)) => *b = !*b,
                _ => (),
            },
            Value::String(s) => {
                if settings.is_resource() {
                    if s.chars().next() == Some('#') {
                        s.remove(0);
                    } else {
                        s.insert(0, '#');
                    }
                }
            }
            _ => (),
        }
    } else if edit_key {
        match val {
            Value::Dictionary(d) => {
                let mut key = settings.sec_key[search_depth].to_owned();
                let hold = d.remove(&key);
                match hold {
                    Some(v) => {
                        write!(stdout, "\x1B8\r{}| \x1B7", "    ".repeat(settings.depth))?;
                        edit_string(&mut key, valid_values, stdout)?;
                        d.insert(key.clone(), v);
                        d.sort_keys();
                        settings.sec_num[settings.depth] =
                            d.keys().position(|k| k == &key).unwrap_or(0);
                    }
                    None => (),
                }
            }
            _ => (),
        }
    } else {
        match val {
            Value::Integer(i) => {
                edit_int(i, valid_values, stdout);
            }
            Value::String(s) => {
                edit_string(s, valid_values, stdout)?;
            }
            Value::Data(d) => {
                edit_data(d, stdout)?;
            }
            _ => (),
        }
    }

    write!(stdout, "{}", cursor::Hide)?;
    settings.modified = true;
    Ok(())
}

fn edit_data(val: &mut Vec<u8>, stdout: &mut Stdout) -> Result<(), Box<dyn Error>> {
    let mut edit_hex = hex::encode(val.clone());
    let mut pos = edit_hex.len();
    let mut hexedit = true;
    write!(
        stdout,
        "edit {und}plist data{res}   {inv}tab{res} switch between editing by hex or string",
        und = style::Attribute::Underlined,
        inv = style::Attribute::Reverse,
        res = style::Attribute::Reset,
    )
    .unwrap();
    //    let mut keys = std::io::stdin().keys();
    loop {
        let mut tmp_val = edit_hex.clone();
        if tmp_val.len() % 2 == 1 {
            tmp_val.insert(0, '0');
        }
        let tmp_val = hex::decode(tmp_val).unwrap();
        write!(
            stdout,
            "\x1B8\x1B[G{mag}as hex{res}\x1B8{}\x1B[0K\x1B[E{mag}as string\x1B[0K\x1B8\x1B[B\x1B[4m{}\x1B8",
            draw::hex_str_with_style(edit_hex.clone()),
            draw::get_lossy_string(&tmp_val),
            mag = "\x1b[35m",
            res = style::Attribute::Reset,
        )?;
        if hexedit {
            write!(
                stdout,
                "\x1B[G{}{}as hex{}\x1B8{}",
                "\x1b[7m",
                "\x1b[35m",
                "\x1b[0m",
                "\x1B[C".repeat(pos)
            )?;
        } else {
            write!(
                stdout,
                "\x1B[E{}{}as string{}\x1B8\x1B[B{}",
                "\x1b[7m",
                "\x1b[35m",
                "\x1b[0m",
                "\x1B[C".repeat(pos / 2)
            )?;
        }
        stdout.flush()?;
        match read_key().unwrap().0 {
            KeyCode::Enter => {
                *val = tmp_val;
                break;
            }
            KeyCode::Backspace => {
                if edit_hex.len() > 0 {
                    if pos > 0 {
                        let _ = edit_hex.remove(pos - 1);
                        pos -= 1;
                        if !hexedit {
                            let _ = edit_hex.remove(pos - 1);
                            pos -= 1;
                        }
                    }
                }
            }
            KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                if hexedit {
                    if edit_hex.len() % 2 == 1 {
                        edit_hex.insert(0, '0');
                    }
                    if pos % 2 == 1 {
                        pos += 1;
                    }
                }
                hexedit = !hexedit;
            }
            KeyCode::Delete => {
                if edit_hex.len() > 0 {
                    if pos < edit_hex.len() {
                        let _ = edit_hex.remove(pos);
                        if !hexedit {
                            let _ = edit_hex.remove(pos);
                        }
                    }
                }
            }
            KeyCode::Left => {
                if pos > 0 {
                    pos -= 1;
                    if !hexedit {
                        pos -= 1;
                    }
                }
            }
            KeyCode::Right => {
                if pos < edit_hex.len() {
                    pos += 1;
                    if !hexedit {
                        pos += 1;
                    }
                }
            }
            KeyCode::Char(c) => {
                if hexedit {
                    if c.is_ascii_hexdigit() {
                        edit_hex.insert(pos, c);
                        pos += 1;
                    }
                } else {
                    if c.is_ascii() {
                        for ic in hex::encode(vec![c as u8]).chars() {
                            edit_hex.insert(pos, ic);
                            pos += 1;
                        }
                    }
                }
            }
            KeyCode::Home => pos = 0,
            KeyCode::End => pos = edit_hex.len(),
            KeyCode::Esc => break,
            _ => (),
        }
    }
    Ok(())
}

fn edit_int(val: &mut Integer, valid_values: Option<&Vec<String>>, stdout: &mut Stdout) {
    let mut new_int = val.as_signed().unwrap();
    let mut selected = 0;
    let mut hit_space = false;
    let mut new = new_int.to_string();
    write!(
        stdout,
        "edit {und}plist integer{res}",
        und = style::Attribute::Underlined,
        res = style::Attribute::Reset,
    )
    .unwrap();
    loop {
        if let Some(valid_values) = valid_values {
            write!(
                stdout,
                " directly or  {inv}up{res}/{inv}down{res} select   {inv}space{res} toggle bit",
                inv = style::Attribute::Reverse,
                res = style::Attribute::Reset,
            )
            .unwrap();
            let mut hex_val;
            write!(stdout, "\x1b8\r\n\x1B[2K\r\n").unwrap();
            for (i, vals) in valid_values.iter().enumerate() {
                if i == selected {
                    write!(stdout, "\x1b[7m").unwrap();
                }
                hex_val = vals.split("---").next().unwrap().trim().to_owned();
                if hex_val.contains(' ') {
                    hex_val = hex_val.split(" ").next().unwrap().trim().to_owned();
                }
                if hex_val.len() > 2 && &hex_val[..2] == "0x" {
                    let dec_val = i64::from_str_radix(&hex_val[2..], 16).unwrap();
                    if dec_val & new_int == dec_val {
                        write!(stdout, "\x1b[32m").unwrap();
                        if hit_space && i == selected {
                            new_int -= dec_val;
                            new = new_int.to_string();
                            hit_space = false;
                            write!(stdout, "\x1b[31m").unwrap();
                        }
                    } else {
                        write!(stdout, "\x1b[31m").unwrap();
                        if hit_space && i == selected {
                            new_int += dec_val;
                            new = new_int.to_string();
                            hit_space = false;
                            write!(stdout, "\x1b[32m").unwrap();
                        }
                    }
                }
                write!(stdout, "{}\x1b[0m\x1B[0K\r\n", vals).unwrap();
            }
            write!(stdout, "\x1B[2K\r\n").unwrap();
        }
        write!(stdout, "\x1B8{}\x1B[0K", new).unwrap();
        stdout.flush().unwrap();

        match read_key().unwrap().0 {
            KeyCode::Enter => {
                *val = match new.parse::<i64>() {
                    Ok(i) => Integer::from(i),
                    _ => Integer::from(0),
                };
                break;
            }
            KeyCode::Backspace => {
                if new.len() > 0 {
                    let _ = new.pop().unwrap();
                }
                if new.len() == 0 {
                    new_int = 0;
                } else if &new != "-" {
                    new_int = new.parse::<i64>().unwrap();
                }
            }
            KeyCode::Char(' ') => hit_space = true,
            KeyCode::Char(c @ '0'..='9') => {
                new.push(c);
                new_int = match new.parse::<i64>() {
                    Ok(int) => int,
                    Err(_) => {
                        new.pop();
                        write!(stdout, "   \x1b[33mERROR:\x1b[0m int value exceeded").unwrap();
                        stdout.flush().unwrap();
                        read_key().unwrap();
                        new.parse::<i64>().unwrap()
                    }
                };
            }
            KeyCode::Char('-') => {
                if new.len() == 0 {
                    new.push('-');
                }
            }
            KeyCode::Up => {
                if selected > 0 {
                    selected -= 1;
                }
            }
            KeyCode::Down => {
                if let Some(valid_values) = valid_values {
                    if selected < valid_values.len() - 1 {
                        selected += 1;
                    }
                }
            }
            KeyCode::Esc => break,
            _ => (),
        };
    }
}

pub fn edit_string(
    val: &mut String,
    valid_values: Option<&Vec<String>>,
    stdout: &mut Stdout,
) -> Result<bool, Box<dyn Error>> {
    let mut new_val_set = false;
    let mut new = String::from(&*val);
    let mut pos = new.len();
    let mut selected = 0;
    write!(
        stdout,
        "edit {und}plist string{res}",
        und = style::Attribute::Underlined,
        res = style::Attribute::Reset,
    )
    .unwrap();
    if let Some(valid_values) = valid_values {
        write!(
            stdout,
            " directly or  {inv}up{res}/{inv}down{res} select",
            inv = style::Attribute::Reverse,
            res = style::Attribute::Reset,
        )
        .unwrap();
        selected = valid_values.len();
        if valid_values.len() > 0 {
            for (i, vals) in valid_values.iter().enumerate() {
                if vals.split("---").next().unwrap().trim() == &new {
                    selected = i;
                }
            }
        }
    }
    loop {
        if let Some(valid_values) = valid_values {
            if valid_values.len() > 0 {
                write!(stdout, "\x1b8\r\n\x1B[2K\r\n").unwrap();
                for (i, vals) in valid_values.iter().enumerate() {
                    if (selected < 5 && i < 10)
                        || (selected >= 5 && i + 5 >= selected && i < selected + 5)
                    {
                        if i == selected {
                            write!(stdout, "\x1b[7m").unwrap();
                        }
                        write!(stdout, "{}\x1b[0m\x1B[0K\r\n\x1b[2K", vals).unwrap();
                    } else if i > 10 {
                        write!(stdout, "<more>\r").unwrap();
                    }
                }
                write!(stdout, "\n\x1B[2K\r\n").unwrap();
            }
        }
        write!(stdout, "\x1B8{}\x1B[0K", draw::highlight_non_print("\x1b[4m", &new, true))?;
        write!(stdout, "\x1B8{}", "\x1B[C".repeat(pos))?;
        stdout.flush()?;
        match read_key().unwrap().0 {
            KeyCode::Enter => {
                *val = new;
                new_val_set = true;
                break;
            }
            KeyCode::Backspace => {
                if new.len() > 0 {
                    if pos > 0 {
                        let _ = new.remove(pos - 1);
                        pos -= 1;
                    }
                }
            }
            KeyCode::Delete => {
                if new.len() > 0 {
                    if pos < new.len() {
                        let _ = new.remove(pos);
                    }
                }
            }
            KeyCode::Up => {
                if selected > 0 {
                    selected -= 1;
                    if let Some(valid_values) = valid_values {
                        new = valid_values[selected]
                            .split("---")
                            .next()
                            .unwrap()
                            .trim()
                            .to_owned();
                        pos = new.len();
                    }
                }
            }
            KeyCode::Down => {
                if let Some(valid_values) = valid_values {
                    if selected < valid_values.len() - 1 {
                        selected += 1;
                    }
                    if selected == valid_values.len() {
                        selected = 0;
                    }
                    new = valid_values[selected]
                        .split("---")
                        .next()
                        .unwrap()
                        .trim()
                        .to_owned();
                    pos = new.len();
                }
            }
            KeyCode::Left => {
                if pos > 0 {
                    pos -= 1;
                }
            }
            KeyCode::Right => {
                if pos < new.len() {
                    pos += 1;
                }
            }
            KeyCode::Char(c) => {
                if c.is_ascii() {
                    new.insert(pos, c);
                    pos += 1;
                }
            }
            KeyCode::Home => pos = 0,
            KeyCode::End => pos = new.len(),
            KeyCode::Esc => break,
            _ => (),
        };
    }
    Ok(new_val_set)
}
