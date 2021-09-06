use std::io::{Read, Stdout, Write};
use std::{thread, time};

use termion::raw::RawTerminal;
use termion::{async_stdin, terminal_size};

// blame mahasvan for this "secret" snake option

pub fn snake(stdout: &mut RawTerminal<Stdout>) {
    let mut direction = 8;
    let mut rest = 100;
    let ma = "BLAME_MAHASVAN_FOR_THIS_";
    let mut masc = ma.chars();

    write!(stdout, "{}", termion::clear::All).unwrap();

    let (col, row) = terminal_size().unwrap();

    let mut scr = vec![false; (row * col).into()];
    let mut sx = col / 2;
    let mut sy = row / 2;
    let mut stdin = async_stdin();

    let mut key_bytes = [0];
    loop {
        stdin.read(&mut key_bytes).unwrap();

        direction = match key_bytes[0] {
            b'w' | b'k' => 8,
            b's' | b'j' => 2,
            b'a' | b'h' => 4,
            b'd' | b'l' => 6,
            _ => direction,
        };
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
        if rest < 20 {
            rest = 20
        };
        if scr[pos] {
            write!(stdout, " you died! ").unwrap();
            stdout.flush().unwrap();
            break;
        } else {
            scr[pos] = true;
            write!(stdout, "\x1B[{};{}H{}", sy, sx, c).unwrap();
            stdout.flush().unwrap();
        }
    }
    stdin.read(&mut key_bytes).unwrap();
    stdin.read(&mut key_bytes).unwrap();
}
