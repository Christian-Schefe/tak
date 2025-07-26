use crate::{
    Action, Board, TRANSPOSITION_TABLE, TranspositionEntry, TranspositionNodeType,
    TranspositionTable, console_log, gen_moves,
};

#[cfg(target_arch = "wasm32")]
pub fn now() -> u64 {
    use web_sys::js_sys::Date;
    Date::new_0().get_time() as u64
}

#[cfg(not(target_arch = "wasm32"))]
pub fn now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

#[derive(Debug, Clone)]
struct Stats {
    node_count: usize,
    found_in_tt: usize,
    saved_by_tt: usize,
}

pub fn iterative_deepening(
    board: &mut Board,
    max_depth: usize,
    max_duration: u64,
) -> (usize, Option<(i32, Action)>) {
    TRANSPOSITION_TABLE.with(|tt| {
        let mut tt = tt.borrow_mut();
        iterative_deepening_with_tt(board, max_depth, max_duration, &mut tt)
    })
}

const INF: i32 = 100_000_000;

fn iterative_deepening_with_tt(
    board: &mut Board,
    max_depth: usize,
    max_duration: u64,
    tt: &mut TranspositionTable,
) -> (usize, Option<(i32, Action)>) {
    let mut best = None;
    let mut best_depth = 0;

    let start_time = now();
    let end_time = start_time + max_duration;

    let mut prev_now = start_time;

    let moves = gen_moves(board);

    for depth in 1..=max_depth {
        let mut stats = Stats {
            node_count: 0,
            found_in_tt: 0,
            saved_by_tt: 0,
        };
        let res = 'l: {
            let mut best_score = -INF;
            let mut best_move = None;
            for mv in moves.iter() {
                let smash = board.make(mv);
                let Some(score) =
                    alphabeta(board, depth, 0, end_time, -INF, INF, tt, &mut stats).map(|s| -s)
                else {
                    break 'l None;
                };
                board.unmake(mv, smash);
                if score > best_score {
                    best_score = score;
                    best_move = Some(mv.clone());
                }
            }
            best_move.map(|m| (best_score, m))
        };

        if res.is_none() {
            console_log!("Timeout at {}", depth);
            break;
        }

        let new_now = now();
        let used_time = new_now - prev_now;
        let grow_factor = 10000.min((used_time * 1000) / (prev_now - start_time + 1));
        prev_now = new_now;

        console_log!(
            "Depth: {}, Score: {:?}, Time: {}ms, Stat: {:?}",
            depth,
            res,
            used_time,
            stats
        );

        best = res;
        best_depth = depth;

        let estimated_time_for_next_depth = (used_time * grow_factor) / 1500;
        if now() + estimated_time_for_next_depth > end_time {
            console_log!(
                "Won't have enough time for next depth (estimated {}), stopping search.",
                estimated_time_for_next_depth
            );
            break;
        }

        if best
            .as_ref()
            .is_some_and(|(score, _)| score.abs() >= 900_000)
        {
            break;
        }
    }

    (best_depth, best)
}

fn alphabeta(
    board: &mut Board,
    depth: usize,
    inv_depth: usize,
    end_time: u64,
    mut alpha: i32,
    beta: i32,
    tt: &mut TranspositionTable,
    stats: &mut Stats,
) -> Option<i32> {
    stats.node_count += 1;

    let is_leaf = depth == 0 || board.result.is_some();

    let prev_best_move = if let Some(entry) = tt.get(board.zobrist) {
        stats.found_in_tt += 1;
        if entry.depth >= depth {
            stats.saved_by_tt += 1;
            match entry.node_type {
                TranspositionNodeType::Exact => return Some(entry.score),
                TranspositionNodeType::Alpha if entry.score <= alpha && !is_leaf => {
                    return Some(entry.score);
                }
                TranspositionNodeType::Beta if entry.score >= beta && !is_leaf => {
                    return Some(entry.score);
                }
                _ => {}
            }
            stats.saved_by_tt -= 1;
            None
        } else {
            entry.best_move.as_ref()
        }
    } else {
        None
    };

    if is_leaf {
        return Some(evaluate_for_active_player(board));
    }

    let mut moves = gen_moves(board);
    if let Some(prev_move_pos) = prev_best_move.and_then(|m| moves.iter().position(|x| x == m)) {
        moves.swap(0, prev_move_pos);
    }

    let mut flag = TranspositionNodeType::Alpha;
    let mut best_move = None;

    for mv in moves {
        let smash = board.make(&mv);
        let score = -alphabeta(
            board,
            depth - 1,
            inv_depth + 1,
            end_time,
            -beta,
            -alpha,
            tt,
            stats,
        )?;
        board.unmake(&mv, smash);
        if score >= beta {
            tt.maybe_insert(TranspositionEntry {
                zobrist: board.zobrist,
                score: beta,
                depth,
                ply: board.ply_index,
                node_type: TranspositionNodeType::Beta,
                best_move: Some(mv),
            });

            return Some(beta);
        }

        if score > alpha {
            flag = TranspositionNodeType::Exact;
            alpha = score;
            best_move = Some(mv);
        }

        if inv_depth < 2 {
            let now = now();
            if now >= end_time {
                return None;
            }
        }
    }

    tt.maybe_insert(TranspositionEntry {
        zobrist: board.zobrist,
        depth,
        ply: board.ply_index,
        score: alpha,
        node_type: flag,
        best_move,
    });

    Some(alpha)
}

fn evaluate_for_active_player(board: &Board) -> i32 {
    let white_score = evaluate(board);
    if board.current_player == 0 {
        white_score
    } else {
        -white_score
    }
}

fn evaluate(board: &Board) -> i32 {
    if let Some(result) = board.result {
        return match result {
            0 => 1_000_000 - board.ply_index as i32,
            1 => -1_000_000 + board.ply_index as i32,
            _ => 0,
        };
    }

    let mut piece_count = 0;
    let mut flat_count_diff = -(board.double_komi as i32);
    for pos in 0..(board.size * board.size) {
        let pos_mask = 1u64 << pos;
        let flat_map = board.occupied & !(board.walls | board.capstones);
        if board.occupied & pos_mask != 0 {
            let is_white_owner = board.owner & pos_mask == 0;
            if is_white_owner {
                piece_count += 1;
                if flat_map & pos_mask != 0 {
                    flat_count_diff += 2;
                }
            } else {
                piece_count -= 1;
                if flat_map & pos_mask != 0 {
                    flat_count_diff -= 2;
                }
            }
        }
    }
    let (longest_road_white, disjoint_count_white) = find_longest_road(board, 0);
    let (longest_road_black, disjoint_count_black) = find_longest_road(board, 1);
    let longest_road =
        longest_road_white * longest_road_white - longest_road_black * longest_road_black;
    let disjoint_count_diff = disjoint_count_white as i32 - disjoint_count_black as i32;
    piece_count * 100 + flat_count_diff * 10 + longest_road * 20 - disjoint_count_diff * 5
}

fn find_longest_road(board: &Board, player: usize) -> (i32, usize) {
    let mut longest = 0;
    let mut visited = vec![false; board.size * board.size];
    let mut disjoint_count = 0;
    for pos in 0..(board.size * board.size) {
        let pos_mask = 1u64 << pos;
        if visited[pos]
            || (board.occupied & pos_mask == 0)
            || (board.owner & pos_mask == 0) != (player == 0)
            || (board.walls & pos_mask != 0)
        {
            continue;
        }
        disjoint_count += 1;
        let road_length = find_road_length(board, pos, player, &mut visited);
        if road_length > longest {
            longest = road_length;
        }
    }
    (longest, disjoint_count)
}

fn find_road_length(
    board: &Board,
    start_pos: usize,
    player: usize,
    visited: &mut Vec<bool>,
) -> i32 {
    let mut stack = vec![start_pos];

    let mut dist_to_left = 0;
    let mut dist_to_right = 0;
    let mut dist_to_top = 0;
    let mut dist_to_bottom = 0;

    while let Some(pos) = stack.pop() {
        let pos_mask = 1u64 << pos;
        if visited[pos]
            || (board.occupied & pos_mask == 0)
            || (board.owner & pos_mask == 0) != (player == 0)
            || (board.walls & pos_mask != 0)
        {
            continue;
        }
        visited[pos] = true;

        let size = board.size;
        let x = pos % size;
        let y = pos / size;
        dist_to_left = dist_to_left.min(x as i32);
        dist_to_right = dist_to_right.min((size - 1 - x) as i32);
        dist_to_top = dist_to_top.min(y as i32);
        dist_to_bottom = dist_to_bottom.min((size - 1 - y) as i32);

        for dir in 0..4 {
            if let Some(new_pos) = board.offset_by_dir(pos, dir) {
                stack.push(new_pos);
            }
        }
    }

    let dist_horizontal = dist_to_left + dist_to_right;
    let dist_vertical = dist_to_top + dist_to_bottom;

    board.size as i32 - dist_horizontal.min(dist_vertical)
}

#[cfg(test)]
mod tests {
    use crate::Settings;

    use super::*;

    #[test]
    fn test_evaluate() {
        let mut board = Board::try_from_pos_str(
            "2,1,1,2,2/2C,1221221221C,111112S,112,2/x,1,21,12,2/1212S,1,2,x,1/1,x4 1 36",
            Settings::new(4),
        )
        .unwrap();
        let mut tt = TranspositionTable::new(16);
        let res = iterative_deepening_with_tt(&mut board, 2, 10_000_000, &mut tt);
        println!("Result: {:?}", res);

        let mut board = Board::try_from_pos_str(
            "1,2,2,1,1/1C,2112112112C,222221S,221,1/x,2,12,21,1/2121S,2,1,x,2/2,x4 2 36",
            Settings::new(4),
        )
        .unwrap();
        let mut tt = TranspositionTable::new(16);
        let res = iterative_deepening_with_tt(&mut board, 2, 10_000_000, &mut tt);
        println!("Result: {:?}", res);

        let mut board = Board::try_from_pos_str(
            "2,1,1,2,2/2C,1221221221C,111112S,112,2/x,1,21,12,2/121,x,212S,x,1/1,x4 2 35",
            Settings::new(4),
        )
        .unwrap();
        let mut tt = TranspositionTable::new(16);
        let res = iterative_deepening_with_tt(&mut board, 2, 10_000_000, &mut tt);
        println!("Result: {:?}", res);
        assert_eq!(res.0, 100_000);
    }
}
