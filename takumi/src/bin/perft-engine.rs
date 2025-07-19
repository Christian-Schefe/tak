use takumi::{perft, Board, Settings};

fn main() {
    let mut results = vec![];
    let now = std::time::Instant::now();
    for depth in 0..7 {
        //let mut game = Board::empty(8);
        let mut game = Board::try_from_pos_str("x2,2,22,2C,1/21221S,1112,x,2211,1,2/x2,111S,x,11S,12S/11S,1S,2S,2,12S,1211C/x,12S,2,122S,x,212S/12,x2,1S,22222S,21121 2 31", Settings::new(4)).unwrap();
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

#[cfg(test)]
mod tests {
    use takumi::perft_safe;

    use super::*;

    #[test]
    fn test_perft() {
        let mut game = Board::try_from_pos_str("x2,2,22,2C,1/21221S,1112,x,2211,1,2/x2,111S,x,11S,12S/11S,1S,2S,2,12S,1211C/x,12S,2,122S,x,212S/12,x2,1S,22222S,21121 2 31", Settings::new(4)).unwrap();
        let results = perft(&mut game, 4);
        let results_safe = perft_safe(&mut game, 4);
        assert_eq!(results, 92392763);
        assert_eq!(results_safe, 92392763);
    }
}
