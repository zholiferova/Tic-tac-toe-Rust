use learn_game::players::{ComputerPlayerRLmax, HumanPlayer, Marks};
use learn_game::q_table::QTable;
use learn_game::Game;
use learn_game::board::IsGameOver;

#[test]
fn outside_test() {
    let player_1 = Box::new(HumanPlayer::new("Oscar".to_owned()));
    let player_2 = Box::new(ComputerPlayerRLmax {
        name: "RLmax".to_owned(),
        mark: Marks::None,
    });
    println!("{:?}", &player_1);
    let mut game = Game::new(player_1, player_2);
    let mut q = QTable::new();
     loop {
        if game.board.is_full() {break;}
        let mv = game.current_player.choose_move(&game.board, &mut q);
        game.current_player.make_move(&mut game.board, &mv);
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
                if game.current_player.get_name() == "Oscar" {
                    println!("Congratulations, Oscar! You have won!");
                    break;
                } else {
                    println!("Really sorry, Oscar, you have lost.");
                    break;
                }
            }
        }
     }
}
