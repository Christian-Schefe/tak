use crate::{gen_moves, Action, Board};

pub fn minimax(board: &mut Board, depth: usize) -> (i32, Option<Action>) {
    let moves = gen_moves(board);
    let maximizing = board.current_player == 0;
    let mut best = if maximizing { i32::MIN } else { i32::MAX };
    let mut best_move = None;

    for mv in moves {
        let smash = board.make(&mv);
        //let score = inner_minimax(board, depth - 1, !maximizing);
        let score = inner_alphabeta(board, depth - 1, i32::MIN, i32::MAX, !maximizing);
        board.unmake(&mv, smash);

        if maximizing && score > best {
            best = score;
            best_move = Some(mv);
        } else if !maximizing && score < best {
            best = score;
            best_move = Some(mv);
        } else if best_move.is_none() {
            best_move = Some(mv);
        }
    }
    (best, best_move)
}

fn inner_alphabeta(
    board: &mut Board,
    depth: usize,
    mut alpha: i32,
    mut beta: i32,
    maximizing: bool,
) -> i32 {
    if depth == 0 || board.result.is_some() {
        return evaluate(board, depth);
    }

    let moves = gen_moves(board);

    if maximizing {
        let mut value = i32::MIN;
        for mv in moves {
            board.make(&mv);
            value = value.max(inner_alphabeta(board, depth - 1, alpha, beta, false));
            board.unmake(&mv, false);
            alpha = alpha.max(value);
            if alpha >= beta {
                break; // Beta cut-off
            }
        }
        value
    } else {
        let mut value = i32::MAX;
        for mv in moves {
            board.make(&mv);
            value = value.min(inner_alphabeta(board, depth - 1, alpha, beta, true));
            board.unmake(&mv, false);
            beta = beta.min(value);
            if beta <= alpha {
                break; // Alpha cut-off
            }
        }
        value
    }
}

fn inner_minimax(board: &mut Board, depth: usize, maximizing: bool) -> i32 {
    if depth == 0 || board.result.is_some() {
        return evaluate(board, depth);
    }

    let moves = gen_moves(board);
    let mut best = if maximizing { i32::MIN } else { i32::MAX };

    for mv in moves {
        let smash = board.make(&mv);
        let score = inner_minimax(board, depth - 1, !maximizing);
        board.unmake(&mv, smash);

        if maximizing && score > best {
            best = score;
        } else if !maximizing && score < best {
            best = score;
        }
    }

    best
}

#[inline(always)]
fn manhattan_from_center(index: usize, size: usize) -> usize {
    let x = index % size;
    let y = index / size;
    let center = (size - 1) / 2;
    let cx = center;
    let cy = center;
    x.abs_diff(cx) + y.abs_diff(cy)
}

fn evaluate(board: &Board, depth: usize) -> i32 {
    if let Some(result) = board.result {
        return match result {
            0 => i32::MAX - depth as i32,
            1 => i32::MIN + depth as i32,
            _ => 0,
        };
    }

    let mut score_total = 0;
    let mut flat_count_diff = board.double_komi as i32;
    for pos in 0..(board.size * board.size) {
        let pos_mask = 1u64 << pos;
        let flat_map = board.occupied & !(board.walls | board.capstones);
        if flat_map & pos_mask != 0 {
            let manhattan = manhattan_from_center(pos, board.size) as i32;
            let score = 100 + (10 - manhattan);
            if board.owner & pos_mask == 0 {
                score_total += score;
                flat_count_diff += 2;
            } else {
                score_total -= score;
                flat_count_diff -= 2;
            }
            for dir in 0..4 {
                let Some(new_pos) = board.offset_by_dir(pos, dir) else {
                    continue;
                };
                let new_mask = 1u64 << new_pos;
                if flat_map & new_mask != 0 {
                    if board.owner & new_mask == 0 {
                        score_total -= 10;
                    } else {
                        score_total += 10;
                    }
                }
            }
        }
    }
    score_total + flat_count_diff * 10
}
