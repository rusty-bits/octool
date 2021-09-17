use std::error::Error;
use std::io::{Read, Stdout, Write};
use std::{thread, time};

use termion::input::TermRead;
use termion::raw::RawTerminal;
use termion::{async_stdin, terminal_size};

// blame mahasvan for this "secret" snake option

struct Snake<'a> {
    stdout: &'a mut RawTerminal<Stdout>,
    col: u16,
    row: u16,
    pos_x: u16,
    pos_y: u16,
}

impl<'a> Snake<'a> {
    fn diaplay(&mut self, c: char) {
            write!(
                self.stdout,
                "\x1B[{};{}H\x1B[7m{}\x1B[0m",
                self.pos_y, self.pos_x, c
            ).unwrap();
            self.stdout.flush().unwrap();
    }
    fn moveit(&mut self, direction: i32) {
        match direction {
            4 => self.up(),
            2 => self.down(),
            3 => self.left(),
            1 => self.right(),
            _ => (),
        }
    }
    fn left(&mut self) {
                self.pos_x -= 1;
                if self.pos_x < 1 {
                    self.pos_x = self.col
                };
            }
    fn right(&mut self) {
                self.pos_x += 1;
                if self.pos_x > self.col {
                    self.pos_x = 1
                };
    }
    fn up(&mut self) {
                self.pos_y -= 1;
                if self.pos_y < 1 {
                    self.pos_y = self.row
                };
    }
    fn down(&mut self) {
                self.pos_y += 1;
                if self.pos_y > self.row {
                    self.pos_y = 1
                };
    }
}

pub fn snake(stdout: &mut RawTerminal<Stdout>) -> Result<(), Box<dyn Error>> {
    let mut direction = 1;
    let mut rest = 100;
    let mut masc = "BLAME_MAHASVAN_FOR_THIS_".chars().cycle();

    write!(stdout, "{}", termion::clear::All)?;

    let (col, row) = terminal_size()?;

    let mut scr = vec![false; (row * col).into()];
    let mut stdin = async_stdin();

    let mut snake = Snake {
        stdout,
        col,
        row,
        pos_x: col / 2,
        pos_y: row / 2,
    };

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
            _ => direction,
        };
        key_bytes[0] = b'x'; // set byte 0 to a non direction value
        snake.moveit(direction);
        let pos = ((snake.pos_y - 1) * col + (snake.pos_x - 1)) as usize;
        thread::sleep(time::Duration::from_millis(rest));
        rest -= 1;
        if rest < 20 {
            rest = 20
        };
        if scr[pos] {
                break;
        } else {
            scr[pos] = true;
            snake.diaplay(masc.next().unwrap());
        }
    }
    write!(stdout, " you died! ")?;
    stdout.flush()?;
    let _ = std::io::stdin().keys();
    Ok(())
}
