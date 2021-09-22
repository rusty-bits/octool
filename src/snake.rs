use core::iter::Cycle;
use core::str::Chars;
use rand::prelude::*;
use std::collections::VecDeque;
use std::error::Error;
use std::io::{Read, Stdout, Write};
use std::ops::Add;
use std::thread::sleep;
use std::time::Duration;

//use termion::input::TermRead;
use termion::raw::RawTerminal;
use termion::{async_stdin, terminal_size};
// blame mahasvan for this "secret" snake option

type Snake = VecDeque<CoordinateVector>;

#[derive(Debug, PartialEq, Copy, Clone)]
struct CoordinateVector(pub i32, pub i32);
impl Add for CoordinateVector {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        CoordinateVector(self.0 + rhs.0, self.1 + rhs.1)
    }
}

fn travel(snake: &mut Snake, direction: CoordinateVector, grow: bool) -> CoordinateVector {
    let &old_head = snake.back().unwrap();
    if grow {
        let &old_tail = snake.front().unwrap();
        for _ in 0..5 {
            snake.push_front(old_tail);
        }
    } else {
        snake.pop_front().unwrap();
    }
    let new_head = old_head + direction;
    snake.push_back(old_head + direction);
    new_head
}

fn head_touching_object(snake: &Snake, object: CoordinateVector) -> bool {
    *snake.back().unwrap() == object
}

fn head_touching_self(snake: &Snake) -> bool {
    let &head = snake.back().unwrap();
    // Find the position of first snake segment which is equal to the head
    let position = snake.iter().position(|&coord| coord == head).unwrap();
    // Return true if the found position is not the head.
    position < snake.len() - 1
}

fn head_out_of_bounds(snake: &Snake, bounds: CoordinateVector) -> bool {
    let &head = snake.back().unwrap();
    head.0 > bounds.0 || head.1 > bounds.1 || head.0 < 1 || head.1 < 1
}

pub fn snake(stdout: &mut RawTerminal<Stdout>) -> Result<(), Box<dyn Error>> {
    let mut masc = "BLAME_MAHASVAN_FOR_THIS_".chars().cycle();
    let mut apple = "113322446655".chars().cycle();
    let mut score = 0;

    write!(stdout, "{}", termion::clear::All)?;

    let (y1, x1) = terminal_size()?;
    let x = i32::from(x1);
    let y = i32::from(y1);

    let mut stdin = async_stdin();

    let mut rng = thread_rng();
    let mut direction = CoordinateVector(1, 0);
    let board_bounds = CoordinateVector(y + 1, x + 1);
    let mut snake = VecDeque::from(vec![CoordinateVector(y / 2, x / 2)]);
    let mut food = get_new_food_position(&snake, board_bounds, &mut rng);

    let mut slp = 100;
    travel(&mut snake, direction, true);
    let mut key_bytes = [0, 0, 0];
    loop {
        if stdin.read(&mut key_bytes)? == 3 {
            key_bytes[0] = key_bytes[2];
        }
        direction = match key_bytes[0] {
            b'D' if direction.1 != 0 => CoordinateVector(-1, 0),
            b'C' if direction.1 != 0 => CoordinateVector(1, 0),
            b'A' if direction.0 != 0 => CoordinateVector(0, -1),
            b'B' if direction.0 != 0 => CoordinateVector(0, 1),
            _ => direction,
        };

        let eating_food = head_touching_object(&snake, food);
        if eating_food {
            score += 1;
            food = get_new_food_position(&snake, board_bounds, &mut rng);
            slp -= 2;
            if slp < 20 {
                slp = 20;
            };
        }
        travel(&mut snake, direction, eating_food);
        display(stdout, &snake, food, &mut masc, &mut apple, score);
        if head_touching_self(&snake) || head_out_of_bounds(&snake, board_bounds) {
            break;
        }
        stdout.flush()?;
        sleep(Duration::from_millis(slp));
    }
    Ok(())
}

fn get_new_food_position(
    snake: &Snake,
    bounds: CoordinateVector,
    rng: &mut ThreadRng,
) -> CoordinateVector {
    let new_position = CoordinateVector(rng.gen_range(1..bounds.0), rng.gen_range(1..bounds.1));
    match snake.contains(&new_position) {
        true => get_new_food_position(snake, bounds, rng),
        false => new_position,
    }
}

fn display(
    stdout: &mut RawTerminal<Stdout>,
    snake: &Snake,
    food: CoordinateVector,
    snk: &mut Cycle<Chars>,
    apple: &mut Cycle<Chars>,
    score: i32,
) {
    write!(
        stdout,
        "\x1B[{};{}H\x1B[3{}m{}\x1B[0m",
        food.1,
        food.0,
        apple.next().unwrap(),
        ''
    )
    .unwrap();
    let segment = snake.back().unwrap();
    write!(
        stdout,
        "\x1B[{};{}H\x1B[7m{}\x1B[0m",
        segment.1,
        segment.0,
        snk.next().unwrap()
    )
    .unwrap();
    let segment = snake.front().unwrap();
    write!(stdout, "\x1B[{};{}H{}", segment.1, segment.0, ' ').unwrap();
    write!(stdout, "\x1B[1;1H{}", score).unwrap();

    //    }
}
