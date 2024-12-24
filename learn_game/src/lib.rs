use crate::board::{Board, IsGameOver};
use crate::config::{NUM_EPISODES, EXPLORATION_RATE, K};
use crate::players::{ComputerPlayerRLmax, ComputerPlayerRLmin, HumanPlayer, MinimaxPlayer, Marks, Player};
use crate::q_table::{Moves, QTable};
use anyhow::Error;
use ndarray::prelude::*;
use rand::prelude::SliceRandom;
use rand::{thread_rng, Rng};
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq, Serializer};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::iter::Iterator;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::thread;
use std::{fmt, io, mem};

pub mod board;
pub mod config;
pub mod players;
pub mod q_table;

pub struct Game {
    pub board: Board,
    pub current_player: Box<dyn Player>,
    pub other_player: Box<dyn Player>,

    episode: usize,
}

impl Game {
    pub fn new(mut player1: Box<dyn Player>, mut player2: Box<dyn Player>) -> Self {
        if player1.get_name() == "RLmax".to_owned() {
            let mut rng = rand::thread_rng();
            player1.set_mark(*[Marks::CROSS, Marks::NOUGHT].choose(&mut rng).unwrap());
        }
        player2.set_mark(player1.get_mark().other());
        if *player1.get_mark() == Marks::CROSS {
            Game {
                board: Board::new(),
                current_player: player1,
                other_player: player2,
                episode: 0,
            }
        } else {
            Game {
                board: Board::new(),
                current_player: player2,
                other_player: player1,
                episode: 0,
            }
        }
    }
    pub fn assign_players (&mut self) {
        let mut rng = rand::thread_rng();
        self.current_player.set_mark(*[Marks::CROSS, Marks::NOUGHT].choose(&mut rng).unwrap());
        self.other_player.set_mark(self.current_player.get_mark().other());
        if *self.current_player.get_mark() == Marks::NOUGHT {self.swap_players();}
    }
    pub fn swap_players(&mut self) {
        mem::swap(&mut self.current_player, &mut self.other_player);
    }
    pub fn swap_states(&mut self) {
        mem::swap(
            &mut self.board.previous_state,
            &mut self.board.current_state,
        );
        self.board.current_state = self.board.next_state.clone();
    }
    fn learn_episode(&mut self, q: Option<&mut QTable>) {
        let mut draw_counter = 0_i32;
        let q = q.expect("QTable is initialized and should be valid at this point.");
        self.board = Board::new();
        self.assign_players();
        loop {
            let current_state_key = self.board.current_state.to_state_key() + self.current_player.get_name();
            q.entry(current_state_key.clone())
                .or_insert(Moves::new(self.board.current_state.available_moves()));
            let mv = if self.episode > 200_000 {
                self.current_player.choose_move_k(&self.board, q)
                } else {
                    self.current_player.choose_move(&self.board, q)
                    };
            if self.episode % 100_000 == 0 {
                self.board.current_state.draw();
                println!("Current_player {:?} mark {:?}", self.current_player.get_name(),
                    self.current_player.get_mark());
                println!("Initial state: {:?}", q[&current_state_key]);
                println!("the current player chose move {:?}", &mv);
            }
            self.current_player.make_move(&mut self.board, &mv);
            let next_state_key = self.board.next_state.to_state_key() + self.other_player.get_name();
            q.entry(next_state_key.clone())
                .or_insert(Moves::new(self.board.next_state.available_moves()));
            let is_over = self.board.is_game_over(&self.current_player.get_mark());
            match is_over {
                IsGameOver::InPlay => {
                    q.update_q_table(
                        &current_state_key,
                        &next_state_key,
                        &mv,
                        self.current_player.get_name(),
                        0.0,
                        false,
                    );
                    self.swap_players();
                    self.swap_states();
                }
                IsGameOver::Drawn => {
                    draw_counter += 1;
                    q.update_q_table(
                        &current_state_key,
                        &next_state_key,
                        &mv,
                        self.current_player.get_name(),
                        0.0,
                        true,
                    );
                    if self.episode % 100_000 == 0 {
                        println!("Episode {:?}, total draws {:?}", self.episode, draw_counter);
                        // println!("Game drawn");
                        // println!("Exit from episode, current player and its mark, qtable['---------'] {:?}, {:?}, {:?}",
                        //     self.current_player.get_name(), self.current_player.get_mark(),
                        //     q[&("---------".to_owned() + self.current_player.get_name())]);
                    }
                    self.episode += 1;
                    break;
                }
                IsGameOver::Win => {
                    let mut reward = 0.0;
                    if self.current_player.get_name() == "RLmax" {
                        reward = 1.0;
                    } else {
                        reward = -1.0;
                    }
                    // if self.episode % 1 == 0 {
                    //     println!(
                    //        "Winner: {:?}, reward: {:?}",
                    //        self.current_player.get_name(),
                    //        reward
                    //     );
                    //     println!("After win QTable before update {:?}", q.get(&current_state_key));
                    // }
                    q.update_q_table(
                        &current_state_key,
                        &next_state_key,
                        &mv,
                        self.current_player.get_name(),
                        reward,
                        true,
                    );
                    // if self.episode % 1 == 0 {
                    //     println!("After win QTable after update {:?}", q.get(&current_state_key));
                    //     // println!("Exit from episode, current player and its mark, qtable['---------'] {:?}, {:?}, {:?}, {:?}",
                    //     //     self.episode, self.current_player.get_name(), self.current_player.get_mark(),
                    //     //     q[&("---------".to_owned() + self.current_player.get_name())]);
                    // }
                    self.episode += 1;
                    break;
                }
            }
        }
    }
    fn learn_episode_with_minimax(&mut self, q: Option<&mut QTable>) {
        let q = q.expect("QTable is initialized and should be valid at this point.");
        self.board = Board::new();
        self.assign_players();
        loop {
            let current_state_key = self.board.current_state.to_state_key() + self.current_player.get_name();
            q.entry(current_state_key.clone())
                .or_insert(Moves::new(self.board.current_state.available_moves()));
            let mv = if self.episode > 10_000 {
                self.current_player.choose_move_k(&self.board, q)
                } else {
                    self.current_player.choose_move(&self.board, q)
                    };
            if self.episode % 10_000 == 0 {
                self.board.current_state.draw();
                println!("Current_player {:?} mark {:?}", self.current_player.get_name(),
                    self.current_player.get_mark());
                println!("Initial state: {:?}", q[&current_state_key]);
                println!("the current player chose move {:?}", &mv);
            }
            self.current_player.make_move(&mut self.board, &mv);
            let next_state_key = self.board.next_state.to_state_key() + self.other_player.get_name();
            q.entry(next_state_key.clone())
                .or_insert(Moves::new(self.board.next_state.available_moves()));
            let is_over = self.board.is_game_over(&self.current_player.get_mark());
            match is_over {
                IsGameOver::InPlay => {
                    q.update_q_table(
                        &current_state_key,
                        &next_state_key,
                        &mv,
                        self.current_player.get_name(),
                        0.0,
                        false,
                    );
                    self.swap_players();
                    self.swap_states();
                }
                IsGameOver::Drawn => {
                    q.update_q_table(
                        &current_state_key,
                        &next_state_key,
                        &mv,
                        self.current_player.get_name(),
                        0.0,
                        true,
                    );
                    if self.episode % 10_000 == 0 {
                        println!("Game drawn");
                        println!("Exit from episode, current player and its mark, qtable['---------'] {:?}, {:?}, {:?}",
                            self.current_player.get_name(), self.current_player.get_mark(),
                            q[&("---------".to_owned() + self.current_player.get_name())]);
                    }
                    self.episode += 1;
                    break;
                }
                IsGameOver::Win => {
                    let mut reward = 0.0;
                    if self.current_player.get_name() == "RLmax" {
                        reward = 1.0;
                    } else {
                        reward = -1.0;
                    }
                    if self.episode % 10_000 == 0 {
                        println!(
                           "Winner: {:?}, reward: {:?}",
                           self.current_player.get_name(),
                           reward
                        );
                        println!("After win QTable before update {:?}", q.get(&current_state_key));
                    }
                    q.update_q_table(
                        &current_state_key,
                        &next_state_key,
                        &mv,
                        self.current_player.get_name(),
                        reward,
                        true,
                    );
                    if self.episode % 10_000 == 0 {
                        println!("After win QTable after update {:?}", q.get(&current_state_key));
                        println!("Exit from episode, current player and its mark, qtable['---------'] {:?}, {:?}, {:?}, {:?}",
                            self.episode, self.current_player.get_name(), self.current_player.get_mark(),
                            q[&("---------".to_owned() + self.current_player.get_name())]);
                    }
                    self.episode += 1;
                    break;
                }
            }
        }
    }
    fn learn_q_table(&mut self, mut q: Option<&mut QTable>) {
        loop {
            if self.episode > 100_000 && self.episode % 10_000 == 0 {
                println!("{:?} {:?}", self.episode, NUM_EPISODES);
                if EXPLORATION_RATE.with_borrow(|erate| -> bool {*erate > 0.1}) {
                    EXPLORATION_RATE.with_borrow_mut(|erate| {
                        *erate -= 0.1;
                        println!("exploration rate is {:?}", erate);
                    });
                }
            }
            if self.episode > 100_000 && self.episode % 5_000 == 0 {
                K.with_borrow_mut(|k| {
                    *k += 0.05;
                    println!("K value is {:?} after episode {:?}", k, self.episode);
                });
            }
            if self.episode >= NUM_EPISODES {
                println!("episode {:?}", self.episode);
                break;
            }
            self.learn_episode(q.as_deref_mut());
        }
        let path = Path::new("./q_table_archive/");
        println!("QTable after exit: {:?}", q);
        let is_q_saved = q_table::q_table_to_disk(&path, q.as_deref().unwrap());
    }
    fn learn_q_table_with_minimax(&mut self, mut q: Option<&mut QTable>) {
        loop {
            if self.episode > 5_000 && self.episode % 1_000 == 0 {
                println!("{:?} {:?}", self.episode, NUM_EPISODES);
                if EXPLORATION_RATE.with_borrow(|erate| -> bool {*erate > 0.1}) {
                    EXPLORATION_RATE.with_borrow_mut(|erate| {
                        *erate -= 0.1;
                        println!("exploration rate is {:?}", erate);
                    });
                }
            }
            if self.episode > 10_000 && self.episode % 5_000 == 0 {
                K.with_borrow_mut(|k| {
                    *k += 0.05;
                    println!("K value is {:?} after episode {:?}", k, self.episode);
                });
            }
            if self.episode >= NUM_EPISODES {
                println!("episode {:?}", self.episode);
                break;
            }
            self.learn_episode_with_minimax(q.as_deref_mut());
        }
        let path = Path::new("./q_table_archive/");
        println!("QTable at the exit: {:?}", q);
        let is_q_saved = q_table::q_table_to_disk(&path, q.as_deref().unwrap());
    }
}
pub fn train_rl_agent() {
    let mut rl_max = Box::new(ComputerPlayerRLmax {
        name: "RLmax".to_string(),
        mark: Marks::None,
    });
    let mut rl_min = Box::new(ComputerPlayerRLmin {
        name: "RLmin".to_string(),
        mark: Marks::None,
    });
    let mut game = Game::new(rl_max, rl_min);
    let mut q = QTable::new();
    game.learn_q_table(Some(&mut q));
}

pub fn train_rl_agent_with_minimax() {
    let mut rl_max = Box::new(ComputerPlayerRLmax {
        name: "RLmax".to_string(),
        mark: Marks::None,
    });
    let path = Path::new("./q_table_archive/qtable-max");
    let mut q_max = RefCell::new(q_table::q_table_from_disk_pickle(&path)
        .expect("The QTable for minimax should be present"));
    let mut rl_min = Box::new(MinimaxPlayer {
        name: "minimax".to_string(),
        mark: Marks::None,
        q_max,
    });
    let mut game = Game::new(rl_max, rl_min);
    let mut q = QTable::new();
    game.learn_q_table(Some(&mut q));
    // let path = Path::new("./q_table_archive/qtable_max");
    // if game.current_player.get_name() == "minimax" {
    //     Box::leak(game.current_player).q_to_disk().expect("Saving minimax QTable to disk")
    // } else {
    //     Box::leak(game.other_player).q_to_disk().expect("Saving minimax QTable to disk")};

}

pub fn play_game_2_humans() {
    let player_1 = Box::new(HumanPlayer::new("Bob".to_string()));
    let m: &Marks = player_1.get_mark();
    let player_2 = Box::new(players::HumanPlayer {
        name: "Alice".to_string(),
        mark: m.other(),
    });
    let mut game = Game::new(player_1, player_2);
    let mut q = QTable::new();
    loop {
        if game.board.is_full() {break;}
        let mv = game.current_player.choose_move(&game.board, &mut q);
        game.current_player.make_move(&mut game.board, &mv);
        // println!("Drawing next state");
        // game.board.next_state.draw();
        let is_over = game.board.is_game_over(&game.current_player.get_mark());
        match is_over {
            IsGameOver::InPlay => {
                game.swap_players();
                game.swap_states();
            }
            IsGameOver::Drawn => {
                println!("The game ended in a draw.");
                break;
            }
            IsGameOver::Win => {
                if game.current_player.get_name() == "Bob" {
                    println!("Congratulations, Bob! You have won!");
                    break;
                } else {
                    println!("Congratulations, Alice! You have won!");
                    break;
                }
            }
        }
    }
}

pub fn play_game_human_computer_player() {
    EXPLORATION_RATE.replace(0.0_f32);
    let player_1 = Box::new(HumanPlayer::new("John".to_owned()));
    let player_2 = Box::new(ComputerPlayerRLmax {
        name: "RLmax".to_owned(),
        mark: Marks::None,
    });
    println!("Player_1 is {:?}", &player_1);
    let mut game = Game::new(player_1, player_2);
    let path = Path::new("./q_table_archive/qtable");
    let mut q = q_table::q_table_from_disk_pickle(&path).expect("QTable is always present");
    println!("QTable's length is: {:?}", q.len());
    loop {
        //if game.board.is_full() {break;}
        let mv = game.current_player.choose_move(&game.board, &mut q);
        game.current_player.make_move(&mut game.board, &mv);
        // println!("Drawing next state");
        // game.board.next_state.draw();
        let is_over = game.board.is_game_over(&game.current_player.get_mark());
        match is_over {
            IsGameOver::InPlay => {
                game.swap_players();
                game.swap_states();
            }
            IsGameOver::Drawn => {
                println!("The game ended in a draw.");
                break;
            }
            IsGameOver::Win => {
                if game.current_player.get_name() == "John" {
                    println!("Congratulations, John! You have won!");
                    break;
                } else {
                    println!("Really sorry, John, you have lost.");
                    break;
                }
            }
        }
     }
}

pub fn play_human_minimax() {
    let path = Path::new("./q_table_archive/qtable-max");
    let mut q_max = RefCell::new(q_table::q_table_from_disk_pickle(&path)
        .expect("The QTable for minimax should be present"));
    let player_1 = Box::new(HumanPlayer::new("John".to_owned()));
    let player_2 = Box::new(MinimaxPlayer {
        name: "MinMax".to_owned(),
        mark: player_1.mark.other(),
        q_max,
    });
    println!("Player_1 is {:?}", &player_1);
    println!("Player_2 is {:?}", &player_2);
    let mut game = Game::new(player_1, player_2);
    // let path = Path::new("./q_table_archive/qtable");
    // let q = q_table::q_table_from_disk_pickle(&path).expect("QTable is always present");
    // println!("QTable's length is: {:?}", q.len());
    let mut q = QTable::new();
    loop {
        let mv = game.current_player.choose_move(&game.board, &mut q);
        game.current_player.make_move(&mut game.board, &mv);
        // println!("Drawing next state");
        // game.board.next_state.draw();
        let is_over = game.board.is_game_over(&game.current_player.get_mark());
        match is_over {
            IsGameOver::InPlay => {
                game.swap_players();
                game.swap_states();
            }
            IsGameOver::Drawn => {
                println!("The game ended in a draw.");
                break;
            }
            IsGameOver::Win => {
                if game.current_player.get_name() == "John" {
                    println!("Congratulations, John! You have won!");
                    break;
                } else {
                    println!("Really sorry, John, you have lost.");
                    break;
                }
            }
        }
     }
     println!("The QTable is {:?}", q);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn is_minimax_player_working(){
        let mut minimax1 = Box::new(MinimaxPlayer {
            name: "Minimax1".to_string(),
            mark: Marks::None,
        });
        let mut minimax2 = Box::new(MinimaxPlayer {
            name: "Minimax2".to_string(),
            mark: Marks::None,
        });
        let mut game = Game::new(minimax1, minimax2);
        game.current_player.set_mark(Marks::NOUGHT);
        println!("Game initiation minimax1 name {:?}, mark {:?}, minimax2 name {:?}, mark {:?}",
            game.current_player.get_name(), game.current_player.get_mark(),
            game.other_player.get_name(), game.other_player.get_mark());
        let mut q = QTable::new();
        let mv = game.current_player.choose_move(&game.board, &mut q);
        println!("After choosing the first move: move {:?}, QTable {:?}", mv, q);
    }

    #[test]
    fn is_game_working() {
        let mut rl_max = Box::new(ComputerPlayerRLmax {
            name: "RLmax".to_string(),
            mark: Marks::None,
        });
        let mut rl_min = Box::new(ComputerPlayerRLmin {
            name: "RLmin".to_string(),
            mark: Marks::None,
        });
        let mut game = Game::new(rl_max, rl_min);
        let mut q = QTable::new();
        game.learn_q_table(Some(&mut q));
    }
}
