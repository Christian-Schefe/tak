use std::collections::HashMap;

use tak_core::{TakGame, TakKomi, TakStones};

fn main() {
    perft("x,2,2,22S,2,111S/21S,22C,112,x,1112S,11S/x,2,112212,2,2S,2/x,2,121122,x,1112,211/21C,x,1,2S,21S,x/2S,x,212,1S,12S,1S 1 33", 2);
    perft("x3/x3/x3 1 1", 8);
}

fn perft(pos: &str, depth: usize) {
    let mut game = TakGame::try_from_tps(&pos, TakKomi::new(0, false)).expect("Invalid position");
    let mut memo = HashMap::new();
    println!("pos {} with depth {}", pos, depth);
    for i in 0..depth {
        let res = run(&mut game, &mut memo, i);
        println!("depth {i}: {}", res);
    }
}

fn run(game: &mut TakGame, memo: &mut HashMap<(String, usize), usize>, depth: usize) -> usize {
    game.validate(&TakStones::from_size(game.board.size));
    let tps = game.to_tps();
    if memo.contains_key(&(tps.clone(), depth)) {
        return *memo.get(&(tps.clone(), depth)).unwrap();
    }

    let mut count = 0;
    let moves = game.gen_moves();
    for action in moves {
        match game.try_do_action(action) {
            Ok(()) => {
                let res = if depth == 0 {
                    1
                } else {
                    run(game, memo, depth - 1)
                };
                count += res;
                game.undo_action().expect("Undo should succeed");
            }
            Err(e) => {
                eprintln!(
                    "Error performing action: {:?}, {:?}, {:?}",
                    e,
                    game,
                    game.to_tps()
                );
            }
        }
    }
    memo.insert((tps, depth), count);
    count
}
