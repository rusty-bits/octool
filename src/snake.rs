use console::{style, Key, Term};
use std::io::Write;
use std::sync::mpsc::{self, TryRecvError};
use std::{thread, time};

// blame mahasvan for this "secret" snake option

pub fn snake(term: &Term) {
    let (tx, rx) = mpsc::channel();

    let t2 = term.clone();

    thread::spawn(move || loop {
        let key = t2.read_key().unwrap();
        match tx.send(key) {
            Ok(_) => (),
            Err(_) => break,
        }
    });

    let mut direction = 8;
    let mut rest = 100;
    let ma = "BLAME_MAHASVAN_FOR_THIS_";
    let mut masc = ma.chars();

    write!(&*term, "\x1B[2J").unwrap();

    let (row, col) = term.size();
    let mut scr = vec![false; (row * col).into()];
    let mut sx = col / 2;
    let mut sy = row / 2;

    loop {
        match rx.try_recv() {
            Ok(key) => {
                direction = match key {
                    Key::ArrowUp | Key::Char('k') => 8,
                    Key::ArrowDown | Key::Char('j') => 2,
                    Key::ArrowLeft | Key::Char('h') => 4,
                    Key::ArrowRight | Key::Char('l') => 6,
                    _ => 4,
                };
            }
            _ => (),
        }
        match direction {
            8 => {
                sy -= 1;
                if sy < 1 {
                    sy = row
                };
            }
            2 => {
                sy += 1;
                if sy > row {
                    sy = 1
                };
            }
            4 => {
                sx -= 1;
                if sx < 1 {
                    sx = col
                };
            }
            6 => {
                sx += 1;
                if sx > col {
                    sx = 1
                };
            }
            _ => (),
        }
        let pos = ((sy - 1) * col + (sx - 1)) as usize;
        let c = match masc.next() {
            Some(c) => c,
            None => {
                masc = ma.chars();
                masc.next().unwrap()
            }
        };
        thread::sleep(time::Duration::from_millis(rest));
        rest -= 1;
        if rest < 20 { rest = 20 };
        if scr[pos] {
            write!(&*term, " you died! ").unwrap();
            while rx.try_recv() == Err(TryRecvError::Empty) {}
            break;
        } else {
            scr[pos] = true;
            write!(&*term, "\x1B[{};{}H{}", sy, sx, style(c).reverse()).unwrap();
        }
    }
}
