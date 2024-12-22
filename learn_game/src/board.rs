use crate::players::Marks;
use itertools::Itertools;
use ndarray::prelude::*;
use std::{
    fmt,
    ops::{Deref, DerefMut},
};

#[derive(Debug, PartialEq)]
pub enum IsGameOver {
    InPlay,
    Drawn,
    Win,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub state: Array<char, Dim<[usize; 2]>>,
}

#[derive(Debug)]
pub struct Board {
    pub previous_state: GameState,
    pub current_state: GameState,
    pub next_state: GameState,
}

impl Deref for GameState {
    type Target = Array<char, Dim<[usize; 2]>>;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl DerefMut for GameState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl fmt::Display for GameState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", &self.iter().collect::<String>())
    }
}

impl GameState {
    pub fn new() -> Self {
        GameState {
            state: Array::from_elem((3, 3), '-'),
        }
    }
    pub fn to_state_key(&self) -> String {
        let state: String = self.state.clone().into_iter().collect::<String>();
        state
    }
    pub fn draw(&self) {
        println!("* * * * *");
        self.to_string()
            .chars()
            .tuples::<(_, _, _)>()
            .for_each(|a| println!("* {} {} {} *", a.0, a.1, a.2));
        println!("* * * * *");
    }
    pub fn available_moves(&self) -> Vec<(usize, usize)> {
        (*self)
            .indexed_iter()
            .filter(|(_index, &value)| value == '-')
            .map(|(index, _)| index)
            .collect()
    }
    pub fn is_game_over(&self, mark: &Marks) -> IsGameOver {
        match *self == mark.as_char() {
            true => IsGameOver::Win,
            false => {
                if self.is_full() {
                    IsGameOver::Drawn
                } else {
                    IsGameOver::InPlay
                }
            }
        }
    }
    pub fn is_full(&self) -> bool {
        let v = (*self)
            .indexed_iter()
            .filter(|(_index, &value)| value == '-')
            .map(|(index, _)| index)
            .collect::<Vec<(usize, usize)>>();
        v.is_empty()
    }
}

impl PartialEq<char> for GameState {
    fn eq(&self, other: &char) -> bool {
        for row in self.rows() {
            let accum = row.fold(true, |acc, x| acc && (x == other));
            if accum == true {
                return true;
            }
        }
        for column in self.columns() {
            let accum = column.fold(true, |acc, x| acc && (x == other));
            if accum == true {
                return true;
            }
        }
        let accum = self.diag().fold(true, |acc, x| acc && (x == other));
        if accum == true {
            return true;
        }
        if self[[0, 2]] == *other && self[[1, 1]] == *other && self[[2, 0]] == *other {
            return true;
        }
        false
    }
}

impl Board {
    pub fn new() -> Self {
        Board {
            previous_state: GameState::new(),
            current_state: GameState::new(),
            next_state: GameState::new(),
        }
    }

    pub fn is_full(&self) -> bool {
        let v = (*self.next_state)
            .indexed_iter()
            .filter(|(_index, &value)| value == '-')
            .map(|(index, _)| index)
            .collect::<Vec<(usize, usize)>>();
        v.is_empty()
    }

    pub fn is_game_over(&self, mark: &Marks) -> IsGameOver {
        match self.next_state == mark.as_char() {
            true => IsGameOver::Win,
            false => {
                if self.is_full() {
                    IsGameOver::Drawn
                } else {
                    IsGameOver::InPlay
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_board_working() {
        let mut board: Board = Board::new();
        board.current_state.state[[0, 0]] = 'X';
        let moves = board.current_state.available_moves();
        println!("Moves: {:?}", moves);
        let b_c = board.current_state.to_string();
        println!("Board.current_state.to_string: {b_c:?}");
        for c in (*board.current_state).indexed_iter() {
            println!("Iterator of board.current_state: {c:?}");
        }
        board.previous_state.draw();
        board.current_state.draw();
        board.next_state.draw();
    }

    #[test]
    fn is_game_over_working() {
        let mark: Marks = Marks::CROSS;
        let mut test_board = Board::new();
        test_board.next_state.state[[1, 1]] = 'X';
        test_board.next_state.state[[0, 0]] = 'X';
        test_board.next_state.state[[2, 2]] = 'X';
        let t: IsGameOver = test_board.is_game_over(&mark);
        assert_eq!(IsGameOver::Win, t);
    }
}
