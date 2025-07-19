use crate::{
    gen_moves, Action, Board, TranspositionEntry, TranspositionNodeType, TranspositionTable,
};

pub fn now() -> u64 {
    use web_sys::js_sys::Date;
    Date::new_0().get_time() as u64
}

pub fn iterative_deepening(
    board: &mut Board,
    max_depth: usize,
    max_duration: u64,
) -> (i32, usize, Option<Action>) {
    let mut tt = TranspositionTable::new(16);
    let mut best_move = None;
    let mut best_score = 0;
    let mut best_depth = 0;

    let end_time = now() + max_duration;

    for depth in 1..=max_depth {
        let Some((score, mv)) = alphabeta(board, depth, end_time, &mut tt) else {
            break;
        };
        best_move = mv;
        best_score = score;
        best_depth = depth;

        if score.abs() >= 900_000 {
            break;
        }
    }

    if best_move.is_none() {
        let moves = gen_moves(board);
        best_move = moves.first().cloned();
    }

    (best_score, best_depth, best_move)
}

pub fn alphabeta(
    board: &mut Board,
    depth: usize,
    end_time: u64,
    tt: &mut TranspositionTable,
) -> Option<(i32, Option<Action>)> {
    let mut moves = gen_moves(board);
    if let Some(entry) = tt.get(board.zobrist) {
        if let Some(tt_move) = entry.best_move.as_ref() {
            if let Some(pos) = moves.iter().position(|m| m == tt_move) {
                moves.swap(0, pos);
            }
        }
    }
    let maximizing = board.current_player == 0;
    let mut best = if maximizing { i32::MIN } else { i32::MAX };
    let mut best_move = None;

    for mv in moves {
        let smash = board.make(&mv);
        let score = inner_alphabeta(board, depth - 1, i32::MIN, i32::MAX, !maximizing, tt);
        board.unmake(&mv, smash);

        if maximizing && score > best {
            best = score;
            best_move = Some(mv);
        } else if !maximizing && score < best {
            best = score;
            best_move = Some(mv);
        }

        if now() > end_time {
            return None;
        }
    }
    Some((best, best_move))
}

fn inner_alphabeta(
    board: &mut Board,
    depth: usize,
    mut alpha: i32,
    mut beta: i32,
    maximizing: bool,
    tt: &mut TranspositionTable,
) -> i32 {
    let prev_best_move = if let Some(entry) = tt.get(board.zobrist) {
        if entry.depth >= depth {
            match entry.node_type {
                TranspositionNodeType::Exact => return entry.score,
                TranspositionNodeType::LowerBound if entry.score > alpha => alpha = entry.score,
                TranspositionNodeType::UpperBound if entry.score < beta => beta = entry.score,
                _ => {}
            }
            None
        } else {
            entry.best_move.as_ref()
        }
    } else {
        None
    };

    if depth == 0 || board.result.is_some() {
        return evaluate(board);
    }

    let mut moves = gen_moves(board);
    if let Some(prev_move_pos) = prev_best_move.and_then(|m| moves.iter().position(|x| x == m)) {
        moves.swap(0, prev_move_pos);
    }

    let alpha_orig = alpha;
    let mut best_move = None;

    let value = if maximizing {
        let mut value = i32::MIN;
        for mv in moves {
            let smash = board.make(&mv);
            let child_value = inner_alphabeta(board, depth - 1, alpha, beta, false, tt);
            board.unmake(&mv, smash);
            if child_value > value {
                value = child_value;
                best_move = Some(mv);
            }
            alpha = alpha.max(value);
            if alpha >= beta {
                break; // Beta cut-off
            }
        }
        value
    } else {
        let mut value = i32::MAX;
        for mv in moves {
            let smash = board.make(&mv);
            let child_value = inner_alphabeta(board, depth - 1, alpha, beta, true, tt);
            board.unmake(&mv, smash);
            if child_value < value {
                value = child_value;
                best_move = Some(mv);
            }
            beta = beta.min(value);
            if beta <= alpha {
                break; // Alpha cut-off
            }
        }
        value
    };

    let (node_type, best_move) = if value <= alpha_orig {
        (TranspositionNodeType::UpperBound, None)
    } else if value >= beta {
        (TranspositionNodeType::LowerBound, None)
    } else {
        (TranspositionNodeType::Exact, best_move)
    };

    tt.insert(TranspositionEntry {
        zobrist: board.zobrist,
        depth,
        score: value,
        node_type,
        best_move,
    });

    value
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
    let mut flat_count_diff = board.double_komi as i32;
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
    let longest_road_white = find_longest_road(board, 0);
    let longest_road_black = find_longest_road(board, 1);
    let longest_road =
        longest_road_white * longest_road_white - longest_road_black * longest_road_black;
    piece_count * 100 + flat_count_diff * 10 + longest_road * 10
}

fn find_longest_road(board: &Board, player: usize) -> i32 {
    let mut longest = 0;
    let mut visited = vec![false; board.size * board.size];
    for pos in 0..(board.size * board.size) {
        let pos_mask = 1u64 << pos;
        if visited[pos]
            || (board.occupied & pos_mask == 0)
            || (board.owner & pos_mask == 0) != (player == 0)
        {
            continue;
        }
        let road_length = find_road_length(board, pos, player, &mut visited);
        if road_length > longest {
            longest = road_length;
        }
    }
    longest
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
        {
            continue;
        }
        visited[pos] = true;

        let size = board.size;
        let x = pos % size;
        let y = pos / size;
        dist_to_left = x as i32;
        dist_to_right = (size - 1 - x) as i32;
        dist_to_top = y as i32;
        dist_to_bottom = (size - 1 - y) as i32;

        for dir in 0..4 {
            if let Some(new_pos) = board.offset_by_dir(pos, dir) {
                stack.push(new_pos);
            }
        }
    }

    let dist_horizontal = dist_to_left + dist_to_right;
    let dist_vertical = dist_to_top + dist_to_bottom;

    dist_horizontal.min(dist_vertical)
}
