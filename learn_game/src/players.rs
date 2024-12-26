use crate::board::{Board, IsGameOver, GameState};
use crate::config::{EXPLORATION_RATE, K};
use crate::q_table::{QTable, Moves};
use rand::prelude::SliceRandom;
use rand::{thread_rng, Rng};
use std::ops::DerefMut;
use std::io;
use chrono::Local;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::Read;
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Marks {
    CROSS = 88isize,
    NOUGHT = 48isize,
    None,
}

#[derive(Debug)]
pub struct HumanPlayer {
    pub name: String,
    pub mark: Marks,
}

#[derive(Debug)]
pub struct ComputerPlayerRLmax {
    pub name: String,
    pub mark: Marks,
}

#[derive(Debug)]
pub struct ComputerPlayerRLmin {
    pub name: String,
    pub mark: Marks,
}
#[derive(Debug)]
pub struct MinimaxPlayer {
    pub name: String,
    pub mark: Marks,
    pub q_max: RefCell<QTable>,
}

pub trait Player {
    fn set_mark(&mut self, mark: Marks);
    fn get_mark(&self) -> &Marks;
    fn get_name(&self) -> &str;
    fn choose_move(&self, board: &Board, q: &mut QTable) -> (usize, usize);
    fn make_move(&self, board: &mut Board, mv: &(usize, usize));
    fn choose_move_k(&self, board: &Board, q: &mut QTable) -> (usize, usize);
    fn q_to_disk(&mut self) -> Result<(), anyhow::Error>;
}

impl Marks {
    pub fn other(self) -> Self {
        match self {
            Self::CROSS => Marks::NOUGHT,
            Self::NOUGHT => Marks::CROSS,
            Self::None => Marks::None,
        }
    }
    pub fn as_char(self) -> char {
        match self {
            Self::CROSS => 'X',
            Self::NOUGHT => '0',
            Self::None => '0',
        }
    }
}
impl Player for HumanPlayer {
    fn set_mark(&mut self, mark: Marks) {
        self.mark = mark;
    }
    fn get_mark(&self) -> &Marks {
        &self.mark
    }
    fn get_name(&self) -> &str {
        &self.name
    }
    fn choose_move(&self, board: &Board, q: &mut QTable) -> (usize, usize) {
        fn move_from_human(board: &Board, _q: &QTable, name: &str) -> Result<(usize, usize), io::Error> {
            let mut row_buf = [0u8; 2];
            let mut col_buf = [0u8; 2];
            let mut garbage = String::new();
            println!("Drawing current state from human.choose_move()");
            board.current_state.draw();
            println!("{:?}, please, choose your move", name);
            println!("please, enter the row number (1-3):");
            let mut counter = 0_i32;
            loop {
                if counter > 2 {println!("You tried 3 times"); return Err(io::Error::other("Wrong symbol 3 times"));}
                io::stdin().read_exact(&mut row_buf)?;
                if (49..=51).contains(&row_buf[0]) {
                    break;
                } else {
                    println!("Unknown symbol, please, try again (a number 1, 2 or 3):");
                    io::stdin().read_line(&mut garbage);
                    row_buf = [0u8; 2];
                }
                counter += 1;
            }
            println!("please, enter the column number (1-3):");
            let mut counter = 0_i32;
            loop {
                if counter > 2 {println!("You tried 3 times"); return Err(io::Error::other("Wrong symbol 3 times"));}
                io::stdin().read_exact(&mut col_buf)?;
                if (49..=51).contains(&col_buf[0]) {
                    break;
                } else {
                    println!("Unknown symbol, please, try again (a number 1, 2 or 3):");
                    io::stdin().read_line(&mut garbage);
                    col_buf = [0u8; 2];
                }
                counter += 1;
            }
            let r = char::from(row_buf[0]).to_digit(10).unwrap();
            let c = char::from(col_buf[0]).to_digit(10).unwrap();
            let mv = (r as usize - 1, c as usize - 1);
            if board.current_state.available_moves().contains(&mv) {return Ok(mv)
            } else {return Err(io::Error::other("The square is taken, please, choose another one."))}
        }
        loop{
            if let Ok((x, y)) = move_from_human(board, q, self.get_name()) {return (x, y);
            } else {println!("Please, try choosing your move again.")}
        }
    }

    fn choose_move_k(&self, board: &Board, q: &mut QTable) -> (usize, usize){
        unimplemented!()
    }
    fn q_to_disk(&mut self) -> Result<(), anyhow::Error> {
        unimplemented!()
    }
    fn make_move(&self, board: &mut Board, mv: &(usize, usize)) {
        let (a, b) = mv.clone();
        println!("Inside make move a {:?} b {:?}", a, b);
        *board.next_state.get_mut([a, b]).unwrap() = self.mark.as_char();
    }
}

impl HumanPlayer {
    pub fn new(name: String) -> Self {
        let mark: Marks;
        loop {
            match Self::choose_mark() {
                Ok('X') => {
                    mark = Marks::CROSS;
                    break;
                }
                Ok('0') => {
                    mark = Marks::NOUGHT;
                    break;
                }
                _ => {
                    println!("You typed a wrong simbol, please try again.")
                }
            }
        }
        HumanPlayer { name, mark }
    }
    pub fn choose_mark() -> io::Result<char> {
        let mut mark_byte = [0u8; 2];
        let mut garbage_mark = String::new();
        println!("Human player, do you want to play X (makes the first move) or 0?");
        io::stdin().read_exact(&mut mark_byte[..])?;
        if mark_byte[1] != 10 {
            io::stdin().read_line(&mut garbage_mark);
        }
        Ok(mark_byte[0] as char)
    }
}

impl Player for ComputerPlayerRLmax {
    fn set_mark(&mut self, mark: Marks) {
        self.mark = mark;
    }
    fn get_mark(&self) -> &Marks {
        &self.mark
    }
    fn get_name(&self) -> &str {
        &self.name
    }
    fn choose_move(&self, board: &Board, q: &mut QTable) -> (usize, usize) {
        let current_state_key = board.current_state.to_state_key() + self.get_name();
        let mut rng = thread_rng();
        let n = rng.gen_range(0_f32..=1_f32);
        EXPLORATION_RATE.with_borrow(|erate| {
            if !q.contains_key(&current_state_key) || n < *erate {
                let available_moves = board.current_state.available_moves();
                return *available_moves.choose(&mut rng).unwrap();
            } else {
                let state_moves = q.get(&current_state_key).unwrap();
                return *state_moves.select_max_move();
            }
        })
    }
    fn make_move(&self, board: &mut Board, mv: &(usize, usize)) {
        let (a, b) = *mv;
        *board.next_state.get_mut([a, b]).unwrap() = self.mark.as_char();
    }
    fn choose_move_k(&self, board: &Board, q: &mut QTable) -> (usize, usize) {
        let current_state_key = board.current_state.to_state_key() + self.get_name();
        let mut moves_with_probabilities: Moves = q.get(&current_state_key).unwrap().clone();
        let min_val = moves_with_probabilities.values().min_by(|a, b| a.total_cmp(b)).unwrap();
        if *min_val < 0_f32 {
            let constant = min_val.abs() + 0.1;
            for value in moves_with_probabilities.values_mut() {
                *value += constant;
            }
        }
        K.with_borrow(|k| {
            let sum = moves_with_probabilities.values().fold(0_f32, |acc, x| acc + x.powf(*k));
            for value in moves_with_probabilities.values_mut() {
                *value = value.powf(*k) / sum;
            }
        });
        let max_move = moves_with_probabilities.iter().max_by(|&x, &y| x.1.total_cmp(y.1)).map(|(key, _value)| key);
        max_move.unwrap().clone()
    }
    fn q_to_disk(&mut self) -> Result<(), anyhow::Error> {
        unimplemented!()
    }
}

impl Player for ComputerPlayerRLmin {
    fn set_mark(&mut self, mark: Marks) {
        self.mark = mark;
    }
    fn get_mark(&self) -> &Marks {
        &self.mark
    }
    fn get_name(&self) -> &str {
        &self.name
    }
    fn choose_move(&self, board: &Board, q: &mut QTable) -> (usize, usize) {
        let current_state_key = board.current_state.to_state_key() + self.get_name();
        let mut rng = thread_rng();
        let n = rng.gen_range(0f32..=1f32);
        EXPLORATION_RATE.with_borrow(|erate| {
            if !q.contains_key(&current_state_key) || n < *erate {
                let available_moves = board.current_state.available_moves();
                return *available_moves.choose(&mut rng).unwrap();
            } else {
                return *q.get(&current_state_key).unwrap().select_min_move();
            }
        })
    }
    fn make_move(&self, board: &mut Board, mv: &(usize, usize)) {
        let (a, b) = *mv;
        *board.next_state.get_mut([a, b]).unwrap() = self.mark.as_char();
    }
    fn choose_move_k(&self, board: &Board, q: &mut QTable) -> (usize, usize) {
        let current_state_key = board.current_state.to_state_key() + self.get_name();
        let mut moves_with_probabilities: Moves = q.get(&current_state_key).unwrap().clone();
        let min_val = moves_with_probabilities.values().min_by(|a, b| a.total_cmp(b)).unwrap();
        if *min_val < 0_f32 {
            let constant = min_val.abs() + 0.1;
            for value in moves_with_probabilities.values_mut() {
                *value += constant;
            }
        }
        K.with_borrow(|k| {
            let sum = moves_with_probabilities.values().fold(0_f32, |acc, x| acc + x.powf(*k));
            for value in moves_with_probabilities.values_mut() {
                *value = value.powf(*k) / sum;
            }
        });
        let min_move = moves_with_probabilities.iter().min_by(|&x, &y| x.1.total_cmp(y.1)).map(|(key, _value)| key);
        min_move.unwrap().clone()
    }
    fn q_to_disk(&mut self) -> Result<(), anyhow::Error> {
        unimplemented!()
    }
}

impl Player for MinimaxPlayer {
    fn set_mark(&mut self, mark: Marks) {
        self.mark = mark;
    }
    fn get_mark(&self) -> &Marks {
        &self.mark
    }
    fn get_name(&self) -> &str {
        &self.name
    }
    fn choose_move(&self, board: &Board, q: &mut QTable) -> (usize, usize) {
        let current_state_key = board.current_state.to_state_key() + "max";
        if let Some(mvs) = self.q_max.borrow_mut().get(&current_state_key) {
            return *mvs.select_max_move()
        }
        let available_moves = board.current_state.available_moves();
        let mut moves_map: Moves = Moves::from(available_moves);
        moves_map.iter_mut().for_each(|(mv, val)| {
            let mut state: GameState = board.current_state.clone();
            let (a, b) = *mv;
            *state.get_mut([a, b]).unwrap() = self.mark.as_char();
            *val = Self::minimax(state, &self.get_mark().other(), &0, false) as f32;
        });
        let best = *moves_map.select_max_move();
        self.q_max.borrow_mut().insert(current_state_key, moves_map);
        best
    }
    fn choose_move_k(&self, board: &Board, q: &mut QTable) -> (usize, usize){
        self.choose_move(board, q)
    }
    fn make_move(&self, board: &mut Board, mv: &(usize, usize)) {
        let (a, b) = *mv;
        *board.next_state.get_mut([a, b]).unwrap() = self.mark.as_char();
    }
    fn q_to_disk(&mut self) -> Result<(), anyhow::Error> {
        let dt = Local::now();
        let today = dt.date_naive();
        let filename = "qtable-max-".to_owned() + (&today.to_string()) + r#".pickle"#;
        let path = Path::new("./q_table_archive/");
        let q_pickle: PathBuf = [path, &Path::new(&filename)].iter().collect();
        let mut file = File::create(&q_pickle)?;
        serde_pickle::to_writer(&mut file, self.q_max.get_mut(), serde_pickle::SerOptions::new())?;
        Ok(())
    }
}
impl MinimaxPlayer {
    pub fn minimax (mut state: GameState, mark: &Marks, depth: &i32, isMax: bool) -> i32 {
        match state.is_game_over(&mark.other()) {
            IsGameOver::Win if isMax => {
                return -10 + depth},
            IsGameOver::Win if !isMax => {
                return 10 - depth},
            IsGameOver::Drawn => {
                return 0},
            IsGameOver::InPlay => {
                if isMax {
                    let mut available_moves: Moves = Moves::from(state.available_moves());
                    available_moves.iter_mut().for_each(|(mv, val)| {
                        let (a, b) = *mv;
                        *state.get_mut([a, b]).unwrap() = mark.as_char();
                        *val = Self::minimax(state.clone(), &mark.other(), &(depth + 1), !isMax) as f32;
                        *state.get_mut([a, b]).unwrap() = '-';
                    });
                    available_moves.values().max_by(|x, y| x.total_cmp(y)).unwrap().clone() as i32
                } else {
                    let mut available_moves: Moves = Moves::from(state.available_moves());
                    available_moves.iter_mut().for_each(|(mv, val)| {
                        let (a, b) = *mv;
                        *state.get_mut([a, b]).unwrap() = mark.as_char();
                        *val = Self::minimax(state.clone(), &mark.other(), &(depth + 1), !isMax) as f32;
                        *state.get_mut([a, b]).unwrap() = '-';
                    });
                    available_moves.values().min_by(|x, y| x.total_cmp(y)).unwrap().clone() as i32
                }
            },
            IsGameOver => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn is_minimax_working() {
        let mut state: GameState = GameState::new();
        *state.get_mut([0, 0]).unwrap() = '0';
        //*state.get_mut([0, 1]).unwrap() = 'X';
        *state.get_mut([0, 2]).unwrap() = 'X';
        *state.get_mut([1, 0]).unwrap() = 'X';
        *state.get_mut([1, 1]).unwrap() = 'X';
        //*state.get_mut([1, 2]).unwrap() = '0';
        *state.get_mut([2, 0]).unwrap() = '0';
        state.draw();
        let mark: Marks = Marks::CROSS;
        let value = MinimaxPlayer::minimax(state, &mark.other(), &0, true);
        println!("The value is {:?}", value);
    }
    #[test]
    fn is_marks_working() {
        let mark_1: Marks = Marks::CROSS;
        let mark_2: Marks = mark_1.other();
        assert_eq!(mark_2, Marks::NOUGHT);
        assert_eq!(
            char::from_u32(Marks::NOUGHT as u32).expect("Not a char"),
            '0'
        );
        assert_eq!(mark_1.as_char(), 'X');
        assert_eq!(mark_2.as_char(), '0');
    }
    #[test]
    #[ignore]
    fn is_human_player_working() {
        let player = HumanPlayer::new("John".to_owned());
        println!("You chose: {:?}", player.mark)
    }
}
