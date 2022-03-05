use defer::defer;
use itertools::Itertools;
use ncurses;
use rand::Rng;

const WORD_LENGTH: usize = 5;
const GUESSES: u32 = 5;

// Ids used by ncurses to identify colors
const COLOR_PAIR_CORRECT: i16 = 1;
const COLOR_PAIR_WRONG_PLACE: i16 = 2;

enum GuessedLetter {
    /// No letter has been entered on this spot yet
    NoLetter,
    /// A letter has been entered but it hasn't been verified yet
    Letter(char),
    /// The letter has been verified and isn't in the target word
    Wrong(char),
    /// The letter has been verified and is in the target word at a different place
    WrongPlace(char),
    /// The letter has been verified and is in this place in the target word
    Correct(char),
}

impl Default for GuessedLetter {
    fn default() -> Self {
        GuessedLetter::NoLetter
    }
}

type GuessedWord = [GuessedLetter; WORD_LENGTH];

#[derive(Default)]
struct BoardState {
    board: [GuessedWord; GUESSES as usize],
    message: Option<String>,
    possible_words: Vec<&'static str>,
}

fn main() {
    // This should be the only object that actually has bytes in it instead of references to bytes
    let word_string = include_str!("../word-list-nl.txt");

    // Collect the possible words into a vector of references
    let mut words: Vec<&str> = word_string
        // The dictionary should have a valid word on each line
        .lines()
        // Only take words of the correct length
        .filter(|word| word.len() == WORD_LENGTH)
        // Remove words that cannot be entered on the keyboard, the lists that are currently used
        // also contain city names
        .filter(|word| word.chars().all(|chr| ('a'..='z').contains(&chr)))
        .collect();

    // Sort the word list and make the list non-mutable afterwards
    words.sort_unstable();
    let words = words;

    // Since the words vector should be sorted now should duplicate words be after each other.
    // unique from itertools could also be used but this is faster since the word list should be
    // sorted.
    if words.iter().tuple_windows::<(_, _)>().any(|(a, b)| a == b) {
        panic!("Word list contains duplicates");
    }

    // Extract the alphabet from the dictionary
    let alphabet;
    {
        alphabet = words
            .iter()
            .flat_map(|word| word.chars())
            .unique()
            .collect_vec();
        // TODO Sort alphabet?
    }

    println!("Alphabet: {:?}", alphabet);

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
    // endwin always needs to get called
    let _window_ender = defer(|| {
        ncurses::endwin();
    });

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

            board_state.possible_words = words
                .iter()
                // Only consider words the fit the currently typed guess
                .filter(|word| word.chars().take(guess.len()).eq(guess.chars()))
                .take(3 + 2 * GUESSES as usize)
                .map(|word| *word)
                .collect();

            // Render the current guess on the screen
            render_game(&board_state);

            // Get input from the user
            let input = ncurses::getch();

            // Act on the input
            if [27].contains(&input) {
                // On escape close down the application
                return;
            } else if [ncurses::KEY_ENTER, '\n' as i32].contains(&input) {
                // On a enter or newline if the current guess is the correct amount of characters
                // process the guess
                if guess.len() == WORD_LENGTH {
                    break;
                }
            } else if [ncurses::KEY_BACKSPACE, ncurses::KEY_DC, 127].contains(&input) {
                // On a backspace remove the last entered letter, if there is one
                if !guess.is_empty() {
                    guess.pop();
                }
            } else if ('a' as i32..='z' as i32).contains(&input) {
                // If the input is a letter add it to the guess, if more letters are allowed in the
                // guess
                if guess.len() < WORD_LENGTH {
                    guess.push(char::from_u32(input as u32).unwrap());
                }
            }

            // Reset the board message
            board_state.message = None;
        }

        assert!(guess.len() == WORD_LENGTH);

        // Process the guessed word
        if !words.contains(&guess.as_str()) {
            // If the word is not in the dictionary disallow the guess
            board_state.message = Some(format!("The word {} is not in the dictionary", guess));
            continue;
        } else {
            // If the word is in the dictionary process each character
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
            // If the guess is equal to the selected word the player wins and the game ends
            board_state.message = Some("You win! Press any key to quit".to_string());
            break;
        } else if guess_num as u32 == GUESSES {
            // If the maximum amount of guesses has been reached the player loses and the game ends
            board_state.message = Some(format!("The word was {}! Press any key to quit.", word));
            break;
        }
    }

    // Render the last message and quit
    render_game(&board_state);
    ncurses::getch();
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
    let print_guess = |y: i32, guess: &GuessedWord| {
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

    // Print the possible words
    for (index, word) in board_state.possible_words.iter().enumerate() {
        ncurses::mvaddstr(win_y + index as i32, win_x + win_width + 1, word);
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
