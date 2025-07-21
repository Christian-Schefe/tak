use crate::{Action, Board};

pub fn print_memo() {
    let partition_memo = compute_partition_memo(8);
    let mut res = Vec::new();
    for (_, a) in partition_memo.iter().enumerate() {
        let mut possibs = Vec::new();
        for (_, b) in a.iter().enumerate() {
            for (_, possib) in b.iter().enumerate() {
                possibs.push(possib);
            }
        }
        res.push(possibs);
    }
    println!(
        "{}",
        res.iter()
            .enumerate()
            .map(|(i, x)| format!(
                "const SPREAD_PARTITIONS_{}: [u64;{}] = [{}];",
                i,
                x.len(),
                x.iter()
                    .map(|x| format!("0x{:x}", x))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

pub fn compute_partition_memo(max_take: usize) -> Vec<Vec<Vec<u64>>> {
    let mut partition_memo = Vec::new();
    for take in 0..=max_take {
        let mut vec = Vec::new();
        for drop_len in 0..=take {
            let mut encoded: Vec<u64> = partition_number(take, drop_len)
                .into_iter()
                .map(encode_spread_vec)
                .collect();
            encoded.sort();
            vec.push(encoded);
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

fn encode_spread_vec(spread_vec: Vec<usize>) -> u64 {
    let mut res = 0u64;
    for (i, &v) in spread_vec.iter().enumerate() {
        res |= (v as u64) << ((i * 4) as u64);
    }
    res
}

pub fn decode_spread_vec(encoded: u64) -> Vec<usize> {
    let mut res = Vec::new();
    let mut encoded = encoded;
    while encoded > 0 {
        res.push((encoded & 0xF) as usize);
        encoded >>= 4;
    }
    res
}

const SPREAD_PARTITIONS_1: [u64; 1] = [0x1];
const SPREAD_PARTITIONS_2: [u64; 2] = [0x2, 0x11];
const SPREAD_PARTITIONS_3: [u64; 4] = [0x3, 0x12, 0x21, 0x111];
const SPREAD_PARTITIONS_4: [u64; 8] = [0x4, 0x13, 0x22, 0x31, 0x112, 0x121, 0x211, 0x1111];
const SPREAD_PARTITIONS_5: [u64; 16] = [
    0x5, 0x14, 0x23, 0x32, 0x41, 0x113, 0x122, 0x131, 0x212, 0x221, 0x311, 0x1112, 0x1121, 0x1211,
    0x2111, 0x11111,
];
const SPREAD_PARTITIONS_6: [u64; 32] = [
    0x6, 0x15, 0x24, 0x33, 0x42, 0x51, 0x114, 0x123, 0x132, 0x141, 0x213, 0x222, 0x231, 0x312,
    0x321, 0x411, 0x1113, 0x1122, 0x1131, 0x1212, 0x1221, 0x1311, 0x2112, 0x2121, 0x2211, 0x3111,
    0x11112, 0x11121, 0x11211, 0x12111, 0x21111, 0x111111,
];
const SPREAD_PARTITIONS_7: [u64; 64] = [
    0x7, 0x16, 0x25, 0x34, 0x43, 0x52, 0x61, 0x115, 0x124, 0x133, 0x142, 0x151, 0x214, 0x223,
    0x232, 0x241, 0x313, 0x322, 0x331, 0x412, 0x421, 0x511, 0x1114, 0x1123, 0x1132, 0x1141, 0x1213,
    0x1222, 0x1231, 0x1312, 0x1321, 0x1411, 0x2113, 0x2122, 0x2131, 0x2212, 0x2221, 0x2311, 0x3112,
    0x3121, 0x3211, 0x4111, 0x11113, 0x11122, 0x11131, 0x11212, 0x11221, 0x11311, 0x12112, 0x12121,
    0x12211, 0x13111, 0x21112, 0x21121, 0x21211, 0x22111, 0x31111, 0x111112, 0x111121, 0x111211,
    0x112111, 0x121111, 0x211111, 0x1111111,
];
const SPREAD_PARTITIONS_8: [u64; 128] = [
    0x8, 0x17, 0x26, 0x35, 0x44, 0x53, 0x62, 0x71, 0x116, 0x125, 0x134, 0x143, 0x152, 0x161, 0x215,
    0x224, 0x233, 0x242, 0x251, 0x314, 0x323, 0x332, 0x341, 0x413, 0x422, 0x431, 0x512, 0x521,
    0x611, 0x1115, 0x1124, 0x1133, 0x1142, 0x1151, 0x1214, 0x1223, 0x1232, 0x1241, 0x1313, 0x1322,
    0x1331, 0x1412, 0x1421, 0x1511, 0x2114, 0x2123, 0x2132, 0x2141, 0x2213, 0x2222, 0x2231, 0x2312,
    0x2321, 0x2411, 0x3113, 0x3122, 0x3131, 0x3212, 0x3221, 0x3311, 0x4112, 0x4121, 0x4211, 0x5111,
    0x11114, 0x11123, 0x11132, 0x11141, 0x11213, 0x11222, 0x11231, 0x11312, 0x11321, 0x11411,
    0x12113, 0x12122, 0x12131, 0x12212, 0x12221, 0x12311, 0x13112, 0x13121, 0x13211, 0x14111,
    0x21113, 0x21122, 0x21131, 0x21212, 0x21221, 0x21311, 0x22112, 0x22121, 0x22211, 0x23111,
    0x31112, 0x31121, 0x31211, 0x32111, 0x41111, 0x111113, 0x111122, 0x111131, 0x111212, 0x111221,
    0x111311, 0x112112, 0x112121, 0x112211, 0x113111, 0x121112, 0x121121, 0x121211, 0x122111,
    0x131111, 0x211112, 0x211121, 0x211211, 0x212111, 0x221111, 0x311111, 0x1111112, 0x1111121,
    0x1111211, 0x1112111, 0x1121111, 0x1211111, 0x2111111, 0x11111111,
];

pub fn gen_moves(game: &Board) -> Vec<Action> {
    let mut flat_place_moves = Vec::new();
    let mut wall_place_moves = Vec::new();
    let mut capstone_place_moves = Vec::new();
    let mut isolated_place_moves = Vec::new();

    let mut flat_capture_moves = Vec::new();
    let mut wall_capture_moves = Vec::new();
    let mut capstone_capture_moves = Vec::new();
    let mut no_capture_spread_moves = Vec::new();

    if game.result.is_some() {
        return Vec::new();
    }

    for pos in 0..(game.size * game.size) {
        let pos_mask = 1u64 << pos;
        if game.occupied & pos_mask == 0 {
            let has_stone = if game.current_player == Board::PLAYER_WHITE {
                game.white_pieces > 0
            } else {
                game.black_pieces > 0
            };
            let has_capstone = if game.current_player == Board::PLAYER_WHITE {
                game.white_capstones > 0
            } else {
                game.black_capstones > 0
            };
            let mut has_neighbor = false;
            for dir in 0..4 {
                if let Some(neighbor_pos) = game.offset_by_dir(pos, dir) {
                    let neighbor_mask = 1u64 << neighbor_pos;
                    if game.occupied & neighbor_mask != 0 {
                        has_neighbor = true;
                        break;
                    }
                }
            }
            if has_stone {
                if has_neighbor {
                    flat_place_moves.push(Action::Place(pos, Board::VARIANT_FLAT));
                } else {
                    isolated_place_moves.push(Action::Place(pos, Board::VARIANT_FLAT));
                };
            }
            if has_stone && game.ply_index >= 2 {
                if has_neighbor {
                    wall_place_moves.push(Action::Place(pos, Board::VARIANT_WALL));
                } else {
                    isolated_place_moves.push(Action::Place(pos, Board::VARIANT_WALL));
                }
            }
            if has_capstone && game.ply_index >= 2 {
                if has_neighbor {
                    capstone_place_moves.push(Action::Place(pos, Board::VARIANT_CAPSTONE));
                } else {
                    isolated_place_moves.push(Action::Place(pos, Board::VARIANT_CAPSTONE));
                }
            }
        }
    }

    if game.ply_index >= 2 {
        for pos in 0..(game.size * game.size) {
            let pos_mask = 1u64 << pos;
            if game.occupied & pos_mask != 0
                && (game.owner & pos_mask == 0) == (game.current_player == Board::PLAYER_WHITE)
            {
                let max_take = (game.size as u64).min(game.stack_heights[pos]);
                let is_wall = game.walls & pos_mask != 0;
                let is_capstone = game.capstones & pos_mask != 0;

                for dir in 0..4 {
                    let max_len = match dir {
                        Board::DIR_RIGHT => game.size - (pos % game.size) - 1,
                        Board::DIR_LEFT => pos % game.size,
                        Board::DIR_DOWN => game.size - (pos / game.size) - 1,
                        Board::DIR_UP => pos / game.size,
                        _ => unreachable!(),
                    };
                    if max_len == 0 {
                        continue;
                    }
                    let mut is_opp_capture = false;
                    for take in 1..=max_take {
                        let spread_partitions = match take {
                            1 => SPREAD_PARTITIONS_1.as_slice(),
                            2 => SPREAD_PARTITIONS_2.as_slice(),
                            3 => SPREAD_PARTITIONS_3.as_slice(),
                            4 => SPREAD_PARTITIONS_4.as_slice(),
                            5 => SPREAD_PARTITIONS_5.as_slice(),
                            6 => SPREAD_PARTITIONS_6.as_slice(),
                            7 => SPREAD_PARTITIONS_7.as_slice(),
                            8 => SPREAD_PARTITIONS_8.as_slice(),
                            _ => panic!("Unsupported board size: {}", take),
                        };

                        let mut cur_pos = pos;
                        let mut cur_len = 0;

                        for &partition in spread_partitions {
                            let len = (partition.ilog2() / 4) + 1;
                            if len as usize > max_len {
                                break;
                            }
                            if cur_len < len {
                                cur_len = len;
                                match dir {
                                    Board::DIR_RIGHT => cur_pos += 1,
                                    Board::DIR_LEFT => cur_pos -= 1,
                                    Board::DIR_DOWN => cur_pos += game.size,
                                    Board::DIR_UP => cur_pos -= game.size,
                                    _ => {}
                                }
                            }
                            let cur_pos_mask = 1u64 << cur_pos;
                            if game.occupied & cur_pos_mask != 0 {
                                if game.capstones & cur_pos_mask != 0 {
                                    break;
                                }
                                if game.walls & cur_pos_mask != 0 {
                                    if game.capstones & pos_mask == 0
                                        || partition >> ((len - 1) * 4) & 0xF != 1
                                    {
                                        break;
                                    }
                                }
                                if (game.owner & cur_pos_mask == 0) != (game.owner & pos_mask == 0)
                                {
                                    is_opp_capture = true;
                                }
                            }
                            let action = Action::Spread(pos, dir, take, partition);
                            if is_opp_capture {
                                if is_wall {
                                    wall_capture_moves.push(action);
                                } else if is_capstone {
                                    capstone_capture_moves.push(action);
                                } else {
                                    flat_capture_moves.push(action);
                                }
                            } else {
                                no_capture_spread_moves.push(action);
                            }
                        }
                    }
                }
            }
        }
    }
    flat_place_moves
        .into_iter()
        .chain(capstone_place_moves)
        .chain(wall_place_moves)
        .chain(capstone_capture_moves)
        .chain(wall_capture_moves)
        .chain(flat_capture_moves)
        .chain(isolated_place_moves)
        .chain(no_capture_spread_moves)
        .collect::<Vec<_>>()
}

pub fn perft_safe(game: &mut Board, depth: usize) -> usize {
    if depth == 0 {
        return 1;
    }

    let moves = gen_moves(game);
    let mut count = 0;

    for action in moves {
        let mut clone = game.clone();
        let smashed = clone.make(&action);
        count += perft(&mut clone, depth - 1);
        clone.unmake(&action, smashed);
        assert_eq!(game, &clone);
    }

    count
}

pub fn perft(game: &mut Board, depth: usize) -> usize {
    if depth == 0 {
        return 1;
    }

    let moves = gen_moves(game);

    if depth == 1 {
        return moves.len();
    }

    let mut count = 0;

    for action in moves {
        let smashed = game.make(&action);
        count += perft(game, depth - 1);
        game.unmake(&action, smashed);
    }

    count
}

#[cfg(test)]
mod tests {
    use crate::Settings;

    pub use super::*;

    #[test]
    fn run_compute_partition_memo() {
        print_memo();
        //assert!(false);
    }

    #[test]
    fn test_gen_place_moves_opening() {
        let mut game = Board::empty(3, Settings::new(4));
        let moves = gen_moves(&game)
            .into_iter()
            .filter_map(|action| match action {
                Action::Place(pos, variant) => Some((pos, variant)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(moves.len(), 9);
        game.place(0, Board::VARIANT_FLAT);
        let moves = gen_moves(&game)
            .into_iter()
            .filter_map(|action| match action {
                Action::Place(pos, variant) => Some((pos, variant)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(moves.len(), 8);

        for (pos, variant) in moves {
            let clone = game.clone();
            game.place(pos, variant);
            game.unplace(pos, variant);
            assert_eq!(clone, game);
        }
    }

    #[test]
    fn test_gen_spread_moves() {
        let mut game = Board::try_from_pos_str("112,x2/x3/x3 2 10", Settings::new(4)).unwrap();
        let moves = gen_moves(&game)
            .into_iter()
            .filter_map(|action| match action {
                Action::Spread(pos, dir, take, partition) => Some((pos, dir, take, partition)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(moves.len(), 12);
        for (pos, dir, take, partition) in moves {
            println!(
                "Pos: {}, Dir: {}, Take: {}, Partition: 0x{:x}",
                pos, dir, take, partition
            );
            let clone = game.clone();
            let smashed = game.spread(pos, dir, take, partition);
            game.unspread(pos, dir, partition, smashed);
            assert_eq!(clone, game);
        }
    }

    #[test]
    fn test_gen_spread_moves_smash() {
        let mut game =
            Board::try_from_pos_str("112C,11S,x3/x5/1C,x4/x5/x5 2 10", Settings::new(4)).unwrap();
        let moves = gen_moves(&game)
            .into_iter()
            .filter_map(|action| match action {
                Action::Spread(pos, dir, take, partition) => Some((pos, dir, take, partition)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(moves.len(), 4);
        for (pos, dir, take, partition) in moves {
            println!(
                "Pos: {}, Dir: {}, Take: {}, Partition: 0x{:x}",
                pos, dir, take, partition
            );
            let clone = game.clone();
            let smashed = game.spread(pos, dir, take, partition);
            game.unspread(pos, dir, partition, smashed);
            assert_eq!(clone, game);
        }
    }

    #[test]
    fn test_gen_spread_moves_tall_stacks() {
        let mut game =
            Board::try_from_pos_str("11222122C,x,11S,x2/x5/1C,x4/x5/x5 2 10", Settings::new(4))
                .unwrap();
        let moves = gen_moves(&game)
            .into_iter()
            .filter_map(|action| match action {
                Action::Spread(pos, dir, take, partition) => Some((pos, dir, take, partition)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(moves.len(), 14);
        for (pos, dir, take, partition) in moves {
            println!(
                "Pos: {}, Dir: {}, Take: {}, Partition: 0x{:x}",
                pos, dir, take, partition
            );
            let clone = game.clone();
            let smashed = game.spread(pos, dir, take, partition);
            game.unspread(pos, dir, partition, smashed);
            assert_eq!(clone, game);
        }
    }
}
