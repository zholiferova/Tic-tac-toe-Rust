use learn_game::players::{ComputerPlayerRLmax, HumanPlayer, MinimaxPlayer, Marks};
use learn_game::q_table::QTable;
use learn_game::Game;
use learn_game::board::IsGameOver;

fn main() {
    learn_game::play_game_human_computer_player();
    //learn_game::play_game_2_humans();
    //learn_game::train_rl_agent();
    //learn_game::train_rl_agent_with_minimax();
    //learn_game::play_human_minimax();
}
