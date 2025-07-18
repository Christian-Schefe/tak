use std::collections::HashMap;

use tak_core::{compute_partition_memo, gen_moves, TakGame, TakGameSettings, TakKomi, TakTps};

fn main() {
    println!("{:?}", perft(6, "x2,2,22,2C,1/21221S,1112,x,2211,1,2/x2,111S,x,11S,12S/11S,1S,2S,2,12S,1211C/x,12S,2,122S,x,212S/12,x2,1S,22222S,21121 2 31", 5));
    //println!("{:?}", perft(3, "x3/x3/x3 1 1", 5));
    //println!("{:?}", perft(5, "x5/x5/x5/x5/x5 1 1", 4));
    //println!("{:?}", perft(6, "x6/x6/x6/x6/x6/x6 1 1", 6));
}

fn perft(size: usize, pos: &str, depth: usize) -> Vec<usize> {
    let tps = TakTps::try_from_str(pos).expect("Failed to parse position");
    let settings =
        TakGameSettings::new_with_position(size, tps, None, TakKomi::new(0, false), None);
    let game = TakGame::new(settings).expect("Failed to create game from position");
    let mut memo = HashMap::new();
    let partition_memo = compute_partition_memo(15);

    let mut results = vec![];
    for i in 0..depth {
        let res = run(&game, &mut memo, &partition_memo, i);
        println!("Depth {}: {}", i, res);
        results.push(res);
    }
    results
}

fn run(
    game: &TakGame,
    memo: &mut HashMap<(String, usize), usize>,
    partition_memo: &Vec<Vec<Vec<Vec<usize>>>>,
    depth: usize,
) -> usize {
    if depth == 0 {
        return 1;
    }
    game.validate().expect("Game should be valid");
    let tps = game.to_tps().to_string();
    if let Some(memo_val) = memo.get(&(tps.clone(), depth)) {
        return *memo_val;
    }

    let mut count = 0;
    let moves = gen_moves(game, partition_memo);
    if depth == 1 {
        memo.insert((tps, depth), moves.len());
        return moves.len();
    }
    for action in moves {
        let mut clone = game.clone();
        match clone.try_do_action(action) {
            Ok(_) => {
                let res = run(&clone, memo, partition_memo, depth - 1);
                count += res;
            }
            Err(e) => {
                eprintln!(
                    "Error performing action: {:?}, {:?}, {:?}",
                    e,
                    clone,
                    clone.to_tps()
                );
            }
        }
    }
    memo.insert((tps, depth), count);
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perft() {
        assert_eq!(
            perft(3, "x3/x3/x3 1 1", 7),
            vec![1, 9, 72, 1200, 17792, 271812, 3712952]
        )
    }
}
