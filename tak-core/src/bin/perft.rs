use std::collections::HashMap;

use tak_core::{compute_partition_memo, TakGame, TakGameSettings, TakKomi, TakStones, TakTps};

fn main() {
    perft(6, "x,2,2,22S,2,111S/21S,22C,112,x,1112S,11S/x,2,112212,2,2S,2/x,2,121122,x,1112,211/21C,x,1,2S,21S,x/2S,x,212,1S,12S,1S 1 33", 2);
    perft(3, "x3/x3/x3 1 1", 5);
    perft(5, "x5/x5/x5/x5/x5 1 1", 8);
}

fn perft(size: usize, pos: &str, depth: usize) {
    let tps = TakTps::try_from_str(pos).expect("Failed to parse position");
    let settings =
        TakGameSettings::new_with_position(size, tps, None, TakKomi::new(0, false), None);
    let mut game = TakGame::new(settings).expect("Failed to create game from position");
    let mut memo = HashMap::new();
    let partition_memo = compute_partition_memo(15);
    println!("pos {} with depth {}", pos, depth);
    for i in 0..depth {
        let res = run(&mut game, &mut memo, &partition_memo, i);
        println!("depth {i}: {}", res);
    }
}

fn run(
    game: &mut TakGame,
    memo: &mut HashMap<(String, usize), usize>,
    partition_memo: &Vec<Vec<Vec<Vec<usize>>>>,
    depth: usize,
) -> usize {
    game.validate(&TakStones::from_size(game.board.size))
        .unwrap();
    let tps = game.to_tps().to_string();
    if memo.contains_key(&(tps.clone(), depth)) {
        return *memo.get(&(tps, depth)).unwrap();
    }

    let mut count = 0;
    let moves = game.gen_moves(partition_memo);
    for action in moves {
        match game.try_do_action(action) {
            Ok(()) => {
                let res = if depth == 0 {
                    1
                } else {
                    run(game, memo, partition_memo, depth - 1)
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
