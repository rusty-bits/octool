use std::error::Error;
use std::io::{Read, Stdout, Write};
use std::{thread, time};

use termion::input::TermRead;
use termion::raw::RawTerminal;
use termion::{async_stdin, terminal_size};

// blame mahasvan for this "secret" snake option

pub fn snake(stdout: &mut RawTerminal<Stdout>) -> Result<(), Box<dyn Error>> {
    let mut direction = 1;
    let mut score = 0;
    let mut rest = 100;
    let mut masc = "BLAME_MAHASVAN_FOR_THIS_".chars().cycle();

    write!(stdout, "{}", termion::clear::All)?;

    let (col, row) = terminal_size()?;

    let mut scr = vec![false; (row * col).into()];
    let mut sx = col / 2;
    let mut sy = row / 2;
//    let mut old_x = sx;
//    let mut old_y = sy;
    let mut stdin = async_stdin();
//    let mut turns = 0;

    let mut key_bytes = [0, 0, 0];
    loop {
        if stdin.read(&mut key_bytes)? == 3 {
            key_bytes[0] = key_bytes[2];
        }
        direction = match key_bytes[0] {
            b'A' | b'k' => 4,
            b'B' | b'j' => 2,
            b'D' | b'h' => 3,
            b'C' | b'l' => 1,
            b'q' => 0,
            _ => direction,
        };
        key_bytes[0] = b'x';
        //        if direction == 0 {
        //            break;
        //        };
        match direction {
            4 => {
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
            3 => {
                sx -= 1;
                if sx < 1 {
                    sx = col
                };
            }
            1 => {
                sx += 1;
                if sx > col {
                    sx = 1
                };
            }
            0 => break,
            _ => (),
        }
        let pos = ((sy - 1) * col + (sx - 1)) as usize;
        thread::sleep(time::Duration::from_millis(rest));
        rest -= 1;
        if rest < 20 {
            rest = 20
        };
        if scr[pos] {
/*            direction += 1;
            if direction == 5 {
                direction = 1
            };
            turns += 1;
            if turns == 5 {
            */
                break;
       /*     };
            sx = old_x;
            sy = old_y;
            */
        } else {
            scr[pos] = true;
//            old_x = sx;
//            old_y = sy;
            score += 1;
//            turns = 0;
            write!(
                stdout,
                "\x1B[1;1H{}\x1B[{};{}H\x1B[7m{}\x1B[0m",
                score, sy, sx, masc.next().unwrap()
            )?;
            stdout.flush()?;
        }
    }
    write!(stdout, " you died! ")?;
    stdout.flush()?;
    let _ = std::io::stdin().keys();
    Ok(())
}
