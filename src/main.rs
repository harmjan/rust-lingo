use ncurses;
use rand::Rng;
use std::fs;

const FILENAME: &str = "word-list-nl.txt";
const WORD_LENGTH: usize = 5;
const GUESSES: u32 = 5;

// Ids used by ncurses to identify colors
const COLOR_PAIR_CORRECT: i16 = 1;
const COLOR_PAIR_WRONG_PLACE: i16 = 2;

enum GuessedLetter {
    // No letter has been entered on this spot yet
    NoLetter,
    // A letter has been entered but it hasn't been verified yet
    Letter(char),
    // The letter has been verified and isn't in the target word
    Wrong(char),
    // The letter has been verified and is in the target word at a different place
    WrongPlace(char),
    // The letter has been verified and is in this place in the target word
    Correct(char),
}

impl Default for GuessedLetter {
    fn default() -> Self {
        GuessedLetter::NoLetter
    }
}

#[derive(Default)]
struct BoardState {
    board: [[GuessedLetter; WORD_LENGTH]; GUESSES as usize],
    message: Option<String>,
}

fn main() {
    // This should be the only object that actually has bytes in it instead of references to bytes
    let word_string = fs::read_to_string(FILENAME).expect("Failed to read file");

    // Collect the possible words into a vector of references
    let mut words: Vec<&str> = word_string
        .lines()
        .filter(|x| x.len() == WORD_LENGTH)
        .collect();

    // Sort the word list and make the list non-mutable afterwards
    words.sort_unstable();
    let words = words;

    // Since the words vector should be sorted now should duplicate words be after each other.
    // unique from itertools could also be used but this is faster since the word list should be
    // sorted.
    if words.windows(2).any(|x| x[0].eq(x[1])) {
        println!("Word list contains duplicates");
        panic!();
    }

    play_game(words);
}

fn play_game(words: Vec<&str>) {
    // Pick a random word
    let word;
    {
        let mut rng = rand::thread_rng();
        let index = rng.gen_range(0..words.len());
        word = words[index];
    }

    // Do some ncurses initialization
    ncurses::initscr();
    ncurses::start_color();
    ncurses::use_default_colors();
    ncurses::init_pair(
        COLOR_PAIR_CORRECT,
        ncurses::COLOR_GREEN,
        ncurses::COLOR_BLACK,
    );
    ncurses::init_pair(
        COLOR_PAIR_WRONG_PLACE,
        ncurses::COLOR_YELLOW,
        ncurses::COLOR_BLACK,
    );
    ncurses::raw();
    ncurses::noecho();
    ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);

    let mut board_state: BoardState = Default::default();
    let mut guess_num = 0;

    // Loop over all the guesses
    loop {
        // Get the guess this round
        let mut guess = String::new();
        // Loop over the characters
        loop {
            // Copy the guess string into the board
            for i in 0..WORD_LENGTH {
                board_state.board[guess_num][i] = match guess.chars().nth(i) {
                    None => GuessedLetter::NoLetter,
                    Some(x) => GuessedLetter::Letter(x),
                };
            }

            // Render the current guess on the screen
            render_game(&board_state);

            let input = ncurses::getch();

            if input == ncurses::KEY_ENTER || input as u8 as char == '\n' {
                if guess.len() == WORD_LENGTH {
                    break;
                }
            } else if input == ncurses::KEY_BACKSPACE || input == ncurses::KEY_DC || input == 127 {
                if !guess.is_empty() {
                    guess.pop();
                }
            } else if 'a' as i32 <= input && input <= 'z' as i32 {
                if guess.len() < WORD_LENGTH {
                    guess.push(input as u8 as char);
                }
            }

            // Reset the board message
            board_state.message = None;
        }

        assert!(guess.len() == WORD_LENGTH);

        // Process the guessed word
        if !words.contains(&guess.as_str()) {
            board_state.message = Some(format!("The word {} is not in the dictionary", guess));
            continue;
        } else {
            for (index, value) in guess.chars().enumerate().map(|(index, chr)| {
                if word.chars().nth(index).unwrap() == chr {
                    (index, GuessedLetter::Correct(chr))
                } else if word.contains(chr) {
                    (index, GuessedLetter::WrongPlace(chr))
                } else {
                    (index, GuessedLetter::Wrong(chr))
                }
            }) {
                board_state.board[guess_num][index] = value;
            }
            guess_num += 1;
        }

        // The game end conditions
        if word.eq_ignore_ascii_case(guess.as_str()) {
            board_state.message = Some("You win! Press any key to quit".to_string());
            break;
        } else if guess_num as u32 == GUESSES {
            board_state.message = Some(format!("The word was {}! Press any key to quit.", word));
            break;
        }
    }

    // Render the last message and quit
    render_game(&board_state);
    ncurses::getch();
    ncurses::endwin();
}

fn render_game(board_state: &BoardState) {
    // First clear whatever was there before
    ncurses::clear();

    // Use ncurses examples from https://lib.rs/crates/ncurses
    let mut max_x = 0;
    let mut max_y = 0;
    ncurses::getmaxyx(ncurses::stdscr(), &mut max_y, &mut max_x);

    let win_width: i32 = 1 + 4 * WORD_LENGTH as i32;
    let win_height: i32 = 3 + 2 * GUESSES as i32;

    let win_x = (max_x - win_width) / 2;
    let win_y = (max_y - win_height) / 2;

    let print_horizontal_line = |y: i32| {
        for i in 0..(WORD_LENGTH) {
            ncurses::mvaddstr(win_y + y, win_x + 4 * i as i32, &"+---".to_string());
        }
        ncurses::mvaddch(win_y + y, win_x + win_width - 1, '+' as ncurses::chtype);
    };
    let print_guess = |y: i32, guess: &[GuessedLetter; WORD_LENGTH]| {
        for i in 0..WORD_LENGTH {
            ncurses::mvaddstr(win_y + y, win_x + 4 * i as i32, &"|   ".to_string());

            // Resolve the guess into a (char, attribute) tuple
            let (character, attribute) = match guess[i as usize] {
                GuessedLetter::NoLetter => (' ', 0),
                GuessedLetter::Letter(x) => (x, 0),
                GuessedLetter::Wrong(x) => (x, ncurses::A_BOLD()),
                GuessedLetter::WrongPlace(x) => (
                    x,
                    ncurses::A_BOLD() | ncurses::COLOR_PAIR(COLOR_PAIR_WRONG_PLACE),
                ),
                GuessedLetter::Correct(x) => (
                    x,
                    ncurses::A_BOLD() | ncurses::COLOR_PAIR(COLOR_PAIR_CORRECT),
                ),
            };

            ncurses::attron(attribute);
            ncurses::mvaddch(
                win_y + y,
                win_x + 2 + 4 * i as i32,
                character.to_ascii_uppercase() as ncurses::chtype,
            );
            ncurses::attroff(attribute);
        }
        ncurses::mvaddch(win_y + y, win_x + win_width - 1, '|' as ncurses::chtype);
    };

    // Print the header
    {
        // Print the top line
        for i in 0..(WORD_LENGTH) {
            ncurses::mvaddstr(win_y, win_x + 4 * i as i32, &"----".to_string());
        }
        ncurses::mvaddch(win_y, win_x, '+' as ncurses::chtype);
        ncurses::mvaddch(win_y, win_x + win_width - 1, '+' as ncurses::chtype);
    }
    {
        // Print the line with LINGO in it
        ncurses::mvaddstr(win_y + 1, win_x + (win_width - 5) / 2, &"LINGO".to_string());
        ncurses::mvaddch(win_y + 1, win_x, '|' as ncurses::chtype);
        ncurses::mvaddch(win_y + 1, win_x + win_width - 1, '|' as ncurses::chtype);
    }
    // The line below LINGO
    print_horizontal_line(2);

    // Print the guesses
    for i in 0..GUESSES {
        print_guess(3 + (i as i32 * 2), &board_state.board[i as usize]);
        print_horizontal_line(4 + 2 * i as i32);
    }

    // Print the message below the window if there is one
    match &board_state.message {
        None => (),
        Some(msg) => {
            ncurses::mvaddstr(
                win_y + win_height + 1,
                (max_x - msg.len() as i32) / 2,
                msg.as_str(),
            );
        }
    }

    ncurses::refresh();
}

/*

For reference:

+-------------------+
|       LINGO       |
+---+---+---+---+---+
|   |   |   |   |   |
+---+---+---+---+---+
|   |   |   |   |   |
+---+---+---+---+---+
|   |   |   |   |   |
+---+---+---+---+---+
|   |   |   |   |   |
+---+---+---+---+---+
|   |   |   |   |   |
+---+---+---+---+---+

The message goes here

*/
