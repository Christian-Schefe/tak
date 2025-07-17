use takumi::{perft, Board};

fn main() {
    let mut results = vec![];
    let now = std::time::Instant::now();
    for depth in 0..9 {
        let mut game = Board::empty(3);
        let res = perft(&mut game, depth);
        results.push(res);
        println!("Depth {}: {}", depth, res);
    }
    let elapsed = now.elapsed();
    println!("Total time: {:.2?}", elapsed);
    assert_eq!(
        results,
        vec![1, 9, 72, 1200, 17792, 271812, 3712952, 52364896, 679639648]
    );
}
