use crate::{TakAction, TakCoord, TakDir, TakGame, TakGameState, TakPieceVariant};

pub fn gen_moves(game: &TakGame, partition_memo: &Vec<Vec<Vec<Vec<usize>>>>) -> Vec<TakAction> {
    if game.game_state != TakGameState::Ongoing {
        return Vec::new();
    }

    let mut moves = Vec::new();
    let player = game.current_player;
    let hand = &game.hands[player.index()];

    for pos in TakCoord::iter_board(game.board.size) {
        if game.board.can_place(pos).is_ok() {
            if hand.stones > 0 {
                moves.push(TakAction::PlacePiece {
                    pos,
                    variant: TakPieceVariant::Flat,
                });
                if game.ply_index >= 2 {
                    moves.push(TakAction::PlacePiece {
                        pos,
                        variant: TakPieceVariant::Wall,
                    });
                }
            }
            if hand.capstones > 0 && game.ply_index >= 2 {
                moves.push(TakAction::PlacePiece {
                    pos,
                    variant: TakPieceVariant::Capstone,
                });
            }
        }
    }

    if game.ply_index < 2 {
        return moves;
    }

    for (pos, stack) in game.board.iter_pieces(Some(player)) {
        for take in 1..=stack.height().min(game.board.size) {
            for &dir in &TakDir::ALL {
                for drop_len in 1..=take {
                    let offset_pos = pos.offset_dir_many(dir, drop_len as i32);
                    if !offset_pos.is_valid(game.board.size) {
                        break;
                    }
                    let drops_vec = if partition_memo.len() > take {
                        partition_memo[take][drop_len].clone()
                    } else {
                        partition_number(take, drop_len)
                    };
                    for drops in drops_vec {
                        if game.board.try_get_stack(offset_pos).is_some_and(|t| {
                            t.variant == TakPieceVariant::Capstone
                                || (t.variant == TakPieceVariant::Wall
                                    && !(*drops.last().expect("Drops should not be empty") == 1
                                        && stack.variant == TakPieceVariant::Capstone))
                        }) {
                            continue;
                        }
                        moves.push(TakAction::MovePiece {
                            pos,
                            dir,
                            take,
                            drops,
                        });
                    }
                    if game
                        .board
                        .try_get_stack(offset_pos)
                        .is_some_and(|t| t.variant != TakPieceVariant::Flat)
                    {
                        break;
                    }
                }
            }
        }
    }

    moves
}

pub fn compute_partition_memo(max_take: usize) -> Vec<Vec<Vec<Vec<usize>>>> {
    let mut partition_memo = Vec::new();
    for take in 0..=max_take {
        let mut vec = Vec::new();
        for drop_len in 0..=take {
            vec.push(partition_number(take, drop_len));
        }
        partition_memo.push(vec);
    }
    partition_memo
}

fn partition_number(num: usize, n: usize) -> Vec<Vec<usize>> {
    if num < n || n == 0 || num == 0 {
        Vec::new()
    } else if n == 1 {
        if num == 0 {
            Vec::new()
        } else {
            vec![vec![num]]
        }
    } else {
        let mut result = Vec::new();
        for first in 1..=(num - n + 1) {
            for mut rest in partition_number(num - first, n - 1) {
                let mut partition = vec![first];
                partition.append(&mut rest);
                result.push(partition);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_number_basic() {
        // partition 5 into 2 parts: should be [[1,4], [2,3], [3,2], [4,1]]
        let mut result = partition_number(5, 2);
        result.sort();
        let mut expected = vec![vec![1, 4], vec![2, 3], vec![3, 2], vec![4, 1]];
        expected.sort();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_partition_number_three_parts() {
        // partition 6 into 3 parts: should be [[1,1,4], [1,2,3], [1,3,2], [1,4,1], [2,1,3], [2,2,2], [2,3,1], [3,1,2], [3,2,1], [4,1,1]]
        let mut result = partition_number(6, 3);
        result.sort();
        let mut expected = vec![
            vec![1, 1, 4],
            vec![1, 2, 3],
            vec![1, 3, 2],
            vec![1, 4, 1],
            vec![2, 1, 3],
            vec![2, 2, 2],
            vec![2, 3, 1],
            vec![3, 1, 2],
            vec![3, 2, 1],
            vec![4, 1, 1],
        ];
        expected.sort();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_partition_number_single_part() {
        // partition 7 into 1 part: should be [[7]]
        let result = partition_number(7, 1);
        assert_eq!(result, vec![vec![7]]);
    }

    #[test]
    fn test_partition_number_no_parts() {
        // partition 0 into 0 parts: should be []
        let result = partition_number(0, 0);
        assert_eq!(result, Vec::<Vec<usize>>::new());
    }

    #[test]
    fn test_partition_number_invalid() {
        // partition 3 into 5 parts: not possible, should be []
        let result = partition_number(3, 5);
        assert_eq!(result, Vec::<Vec<usize>>::new());
    }
}
