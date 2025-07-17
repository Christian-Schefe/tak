#[derive(Debug, Clone, PartialEq)]
pub struct Board {
    pub size: usize,
    pub result: Option<u64>, // 0 for white win, 1 for black win, 2 for draw

    pub ply_index: usize,
    pub current_player: u64,
    pub empty_positions: usize,

    pub white_pieces: u64,
    pub black_pieces: u64,
    pub white_capstones: u64,
    pub black_capstones: u64,

    pub occupied: u64,
    pub walls: u64,
    pub capstones: u64,
    pub owner: u64,
    pub stacks: Vec<u64>,
    pub stack_heights: Vec<u64>,
}

impl Board {
    pub const VARIANT_FLAT: usize = 0;
    pub const VARIANT_WALL: usize = 1;
    pub const VARIANT_CAPSTONE: usize = 2;

    pub const PLAYER_WHITE: u64 = 0;
    pub const PLAYER_BLACK: u64 = 1;

    pub const DIR_RIGHT: usize = 0;
    pub const DIR_LEFT: usize = 1;
    pub const DIR_DOWN: usize = 2;
    pub const DIR_UP: usize = 3;

    pub fn empty(size: usize) -> Self {
        let (white_pieces, white_capstones) = Self::default_pieces(size);
        let (black_pieces, black_capstones) = Self::default_pieces(size);

        Self {
            size,
            result: None,

            ply_index: 0,
            current_player: Self::PLAYER_WHITE,
            empty_positions: size * size,

            white_pieces,
            black_pieces,
            white_capstones,
            black_capstones,

            occupied: 0,
            walls: 0,
            capstones: 0,
            owner: 0,
            stacks: vec![0; size * size],
            stack_heights: vec![0; size * size],
        }
    }

    pub fn default_pieces(size: usize) -> (u64, u64) {
        match size {
            3 => (10, 0),
            4 => (15, 0),
            5 => (21, 1),
            6 => (30, 1),
            7 => (40, 2),
            8 => (50, 2),
            _ => panic!("Invalid board size"),
        }
    }

    fn neighbors(pos: usize, size: usize) -> [Option<usize>; 4] {
        let mut neighbors = [None; 4];
        if pos % size > 0 {
            neighbors[0] = Some(pos - 1); // left
        }
        if pos % size < size - 1 {
            neighbors[1] = Some(pos + 1); // right
        }
        if pos / size > 0 {
            neighbors[2] = Some(pos - size); // up
        }
        if pos / size < size - 1 {
            neighbors[3] = Some(pos + size); // down
        }
        neighbors
    }

    fn check_road_win(&mut self, player: u64, pos: usize) -> bool {
        let pos_mask = 1u64 << pos;
        if self.occupied & pos_mask == 0 || self.walls & pos_mask != 0 {
            return false;
        }

        let mut visited = 0u64;
        let mut stack = [0usize; 64];
        stack[0] = pos;
        visited |= 1u64 << pos;
        let mut top = 1;

        let mut has_top = false;
        let mut has_bottom = false;
        let mut has_left = false;
        let mut has_right = false;

        let size = self.size;
        let size_max = size - 1;

        while top > 0 {
            top -= 1;
            let cur = stack[top];
            let x = cur % size;
            let y = cur / size;
            if x == 0 {
                has_left = true;
            } else if x == size_max {
                has_right = true;
            }
            if y == 0 {
                has_top = true;
            } else if y == size_max {
                has_bottom = true;
            }
            if (has_bottom && has_top) || (has_left && has_right) {
                self.result = Some(player);
                return true;
            }

            for neighbor in Self::neighbors(cur, size) {
                let Some(neighbor) = neighbor else {
                    continue;
                };
                let neighbor_mask = 1u64 << neighbor;
                if self.occupied & neighbor_mask == 0
                    || visited & neighbor_mask != 0
                    || self.walls & neighbor_mask != 0
                    || (self.owner & neighbor_mask == 0) != (player == Self::PLAYER_WHITE)
                {
                    continue;
                }
                visited |= neighbor_mask;
                stack[top] = neighbor;
                top += 1;
            }
        }

        false
    }

    fn check_flat_win(&mut self) -> bool {
        if self.empty_positions == 0 {
            let mut white_count = 0;
            let mut black_count = 0;
            for pos in 0..(self.size * self.size) {
                let pos_mask = 1u64 << pos;
                if self.stack_heights[pos] == 0
                    || self.walls & pos_mask != 0
                    || self.capstones & pos_mask != 0
                {
                    continue;
                }
                if self.stacks[pos] & (1 << (self.stack_heights[pos] - 1)) == 0 {
                    white_count += 1;
                } else {
                    black_count += 1;
                };
            }
            if white_count > black_count {
                self.result = Some(Self::PLAYER_WHITE);
            } else if black_count > white_count {
                self.result = Some(Self::PLAYER_BLACK);
            } else {
                self.result = Some(2);
            }
            true
        } else {
            false
        }
    }

    fn controlling_player(&self, pos: usize) -> u64 {
        if self.stacks[pos] & (1 << (self.stack_heights[pos] - 1)) == 0 {
            Self::PLAYER_WHITE
        } else {
            Self::PLAYER_BLACK
        }
    }

    pub fn place(&mut self, pos: usize, variant: usize) {
        let pos_mask = 1u64 << pos;

        let mut effective_player = self.current_player;
        if self.ply_index < 2 {
            effective_player = 1 - effective_player;
        }

        self.occupied |= pos_mask;
        if effective_player == Self::PLAYER_BLACK {
            self.owner |= pos_mask;
        }

        if variant == Self::VARIANT_WALL {
            self.walls |= pos_mask;
        }

        if variant == Self::VARIANT_CAPSTONE {
            self.capstones |= pos_mask;
            if self.current_player == Self::PLAYER_WHITE {
                self.white_capstones -= 1;
            } else {
                self.black_capstones -= 1;
            }
        } else {
            if self.current_player == Self::PLAYER_WHITE {
                self.white_pieces -= 1;
            } else {
                self.black_pieces -= 1;
            }
        }
        self.stack_heights[pos] = 1;
        self.stacks[pos] = effective_player;

        self.ply_index += 1;
        self.current_player = 1 - self.current_player;
        self.empty_positions -= 1;

        if self.check_road_win(effective_player, pos) {
            return;
        }
        self.check_flat_win();
    }

    pub fn unplace(&mut self, pos: usize, variant: usize) {
        self.ply_index -= 1;
        self.current_player = 1 - self.current_player;
        self.empty_positions += 1;

        let not_pos_mask = !(1u64 << pos);

        self.owner &= not_pos_mask;
        self.occupied &= not_pos_mask;
        self.walls &= not_pos_mask;
        self.capstones &= not_pos_mask;
        self.stack_heights[pos] = 0;
        self.stacks[pos] = 0;
        self.result = None;

        if variant == Self::VARIANT_CAPSTONE {
            if self.current_player == Self::PLAYER_WHITE {
                self.white_capstones += 1;
            } else {
                self.black_capstones += 1;
            }
        } else {
            if self.current_player == Self::PLAYER_WHITE {
                self.white_pieces += 1;
            } else {
                self.black_pieces += 1;
            }
        }
    }

    pub fn spread(&mut self, pos: usize, dir: usize, take: u64, spreads: u64) -> bool {
        let pos_mask = 1u64 << pos;
        let not_pos_mask = !pos_mask;

        let prev_height = self.stack_heights[pos];
        let new_height = prev_height - take;
        self.stack_heights[pos] = new_height;

        let take_mask = (1 << take) - 1;
        let mut took_pattern = self.stacks[pos] >> new_height;
        self.stacks[pos] &= !(take_mask << new_height);

        let is_wall = (self.walls & pos_mask) != 0;
        let is_capstone = (self.capstones & pos_mask) != 0;

        if new_height == 0 {
            self.occupied &= not_pos_mask;
            self.owner &= not_pos_mask;
            self.empty_positions += 1;
        } else {
            if self.controlling_player(pos) == Self::PLAYER_WHITE {
                self.owner &= not_pos_mask;
            } else {
                self.owner |= pos_mask;
            }
        }

        let mut cur_pos = pos;
        let mut cur_pos_mask = pos_mask;
        let mut move_distance = 0;

        for i in 0..7 {
            let spread = (spreads >> (i * 4)) & 0xF;
            if spread == 0 {
                break;
            }
            move_distance += 1;
            match dir {
                Self::DIR_RIGHT => cur_pos += 1,
                Self::DIR_LEFT => cur_pos -= 1,
                Self::DIR_DOWN => cur_pos += self.size,
                Self::DIR_UP => cur_pos -= self.size,
                _ => {}
            }
            cur_pos_mask = 1u64 << cur_pos;
            let prev_height = self.stack_heights[cur_pos];
            self.stack_heights[cur_pos] += spread;
            let this_pattern = took_pattern & ((1 << spread) - 1);
            took_pattern >>= spread;
            self.stacks[cur_pos] |= this_pattern << prev_height;
            if prev_height == 0 {
                self.empty_positions -= 1;
                self.occupied |= cur_pos_mask;
            }
            if self.controlling_player(cur_pos) == Self::PLAYER_WHITE {
                self.owner &= !cur_pos_mask;
            } else {
                self.owner |= cur_pos_mask;
            }
        }

        let smash = (self.walls & cur_pos_mask) != 0;
        if smash {
            self.walls &= !cur_pos_mask;
        }

        if is_wall {
            self.walls &= not_pos_mask;
            self.walls |= cur_pos_mask;
        } else if is_capstone {
            self.capstones &= not_pos_mask;
            self.capstones |= cur_pos_mask;
        }

        let moving_player = self.current_player;

        self.ply_index += 1;
        self.current_player = 1 - self.current_player;

        for player in [moving_player, 1 - moving_player] {
            let mut check_pos = pos;
            for i in 0..=move_distance {
                if i > 0 {
                    match dir {
                        Self::DIR_RIGHT => check_pos += 1,
                        Self::DIR_LEFT => check_pos -= 1,
                        Self::DIR_DOWN => check_pos += self.size,
                        Self::DIR_UP => check_pos -= self.size,
                        _ => {}
                    }
                }
                let check_pos_mask = 1u64 << check_pos;
                if self.stack_heights[check_pos] != 0
                    && (self.owner & check_pos_mask == 0) == (player == Self::PLAYER_WHITE)
                    && self.check_road_win(player, check_pos)
                {
                    return smash;
                }
            }
        }

        self.check_flat_win();

        smash
    }

    pub fn unspread(&mut self, pos: usize, dir: usize, spreads: u64, smash: bool) {
        self.ply_index -= 1;
        self.current_player = 1 - self.current_player;
        self.result = None;

        let pos_mask = 1u64 << pos;

        let mut cur_pos = pos;
        let mut cur_pos_mask = pos_mask;

        let mut took_pattern = 0;
        let mut spread_sum = 0;

        for i in 0..7 {
            let spread = (spreads >> (i * 4)) & 0xF;
            if spread == 0 {
                break;
            }
            match dir {
                Self::DIR_RIGHT => cur_pos += 1,
                Self::DIR_LEFT => cur_pos -= 1,
                Self::DIR_DOWN => cur_pos += self.size,
                Self::DIR_UP => cur_pos -= self.size,
                _ => {}
            }
            cur_pos_mask = 1u64 << cur_pos;
            let prev_height = self.stack_heights[cur_pos];
            let new_height = prev_height - spread;
            self.stack_heights[cur_pos] = new_height;

            let spread_mask = (1u64 << spread) - 1;
            let this_pattern = self.stacks[cur_pos] >> new_height;
            self.stacks[cur_pos] &= !(spread_mask << new_height);

            if new_height == 0 {
                self.occupied &= !cur_pos_mask;
                self.empty_positions += 1;
                self.owner &= !cur_pos_mask;
            } else {
                if self.controlling_player(cur_pos) == Self::PLAYER_WHITE {
                    self.owner &= !cur_pos_mask;
                } else {
                    self.owner |= cur_pos_mask;
                }
            }

            took_pattern |= this_pattern << spread_sum;
            spread_sum += spread;
        }

        let is_wall = (self.walls & cur_pos_mask) != 0;
        let is_capstone = (self.capstones & cur_pos_mask) != 0;

        if is_wall {
            self.walls &= !cur_pos_mask;
            self.walls |= pos_mask;
        } else if is_capstone {
            self.capstones &= !cur_pos_mask;
            self.capstones |= pos_mask;
        }

        if smash {
            self.walls |= cur_pos_mask;
        }

        let prev_height = self.stack_heights[pos];
        if prev_height == 0 {
            self.occupied |= pos_mask;
            self.empty_positions -= 1;
        }
        self.stack_heights[pos] += spread_sum;
        self.stacks[pos] |= took_pattern << prev_height;

        if self.controlling_player(pos) == Self::PLAYER_WHITE {
            self.owner &= !pos_mask;
        } else {
            self.owner |= pos_mask;
        }
    }

    pub fn try_from_pos_str(position: &str) -> Option<Self> {
        let mut occupied = 0;
        let mut walls = 0;
        let mut capstones = 0;
        let mut owner = 0;
        let mut stack_heights = Vec::new();
        let mut stacks = Vec::new();
        let mut size = 0;

        let sections = position.split(' ').collect::<Vec<_>>();
        if sections.len() != 3 {
            return None;
        }

        let current_player = match sections[1] {
            "1" => 0,
            "2" => 1,
            _ => return None,
        };
        let mut move_index = sections[2].parse::<usize>().ok()?;
        if move_index == 0 {
            return None;
        }
        move_index -= 1;

        let ply_index = move_index * 2 + current_player as usize;

        let mut white_pieces = 0;
        let mut white_capstones = 0;
        let mut black_pieces = 0;
        let mut black_capstones = 0;

        let mut empty_positions = 0;

        let mut pos = 0;
        for row in sections[0].split('/') {
            let mut part_count = 0;
            for part in row.split(',') {
                if part.starts_with('x') {
                    let amount = if part.len() == 1 {
                        1
                    } else {
                        part[1..].parse::<usize>().ok()?
                    };
                    pos += amount;
                    part_count += amount;
                    empty_positions += amount;
                    for _ in 0..amount {
                        stack_heights.push(0);
                        stacks.push(0);
                    }
                } else {
                    let pos_mask = 1u64 << pos;
                    occupied |= pos_mask;
                    let is_capstone = part.ends_with('C');
                    let is_wall = part.ends_with('S');
                    if is_capstone {
                        capstones |= pos_mask;
                    } else if is_wall {
                        walls |= pos_mask;
                    }
                    let mut height = 0;
                    let mut stack = 0;
                    let mut chars = part.chars();
                    while let Some(char) = chars.next() {
                        if char == '2' {
                            stack |= 1 << height;
                            black_pieces += 1;
                        } else if char == '1' {
                            white_pieces += 1;
                        } else {
                            if char != 'S' && char != 'C' && char != 'F' {
                                return None;
                            }
                            break;
                        }
                        height += 1;
                    }
                    if chars.next().is_some() {
                        return None;
                    }
                    if height == 0 {
                        return None;
                    }
                    let is_white = stack & (1 << (height - 1)) == 0;
                    if is_capstone {
                        if is_white {
                            white_capstones += 1;
                        } else {
                            black_capstones += 1;
                        };
                    }
                    if !is_white {
                        owner |= pos_mask;
                    }
                    stack_heights.push(height);
                    stacks.push(stack);
                    pos += 1;
                    part_count += 1;
                }
            }
            if size == 0 {
                size = part_count;
            } else if size != part_count {
                return None;
            }
        }
        if pos != size * size {
            return None;
        }
        if size < 3 || size > 8 {
            return None;
        }

        if white_capstones > white_pieces {
            return None;
        }
        white_pieces -= white_capstones;
        if black_capstones > black_pieces {
            return None;
        }
        black_pieces -= black_capstones;

        let (piece_count, capstone_count) = Self::default_pieces(size);
        if white_pieces > piece_count || black_pieces > piece_count {
            return None;
        }
        if white_capstones > capstone_count || black_capstones > capstone_count {
            return None;
        }

        let mut board = Self {
            size,
            result: None,

            ply_index,
            current_player,
            empty_positions,

            white_pieces: piece_count - white_pieces,
            white_capstones: capstone_count - white_capstones,
            black_pieces: piece_count - black_pieces,
            black_capstones: capstone_count - black_capstones,

            occupied,
            walls,
            capstones,
            owner,
            stack_heights,
            stacks,
        };
        board.check_flat_win();
        for pos in 0..(size * size) {
            if board.stack_heights[pos] > 0 {
                board.check_road_win(board.controlling_player(pos), pos);
            }
        }

        Some(board)
    }

    pub fn to_pos_str(&self) -> String {
        let mut result = String::new();
        let size = self.size;

        for y in 0..size {
            if y > 0 {
                result.push('/');
            }
            let mut empty_count = 0;
            let mut first = true;
            for x in 0..size {
                let pos = y * size + x;
                if self.stack_heights[pos] == 0 {
                    empty_count += 1;
                } else {
                    if !first {
                        result.push(',');
                    }
                    first = false;
                    if empty_count > 0 {
                        if empty_count == 1 {
                            result.push_str("x,");
                        } else {
                            result.push_str(&format!("x{},", empty_count));
                        }
                        empty_count = 0;
                    }
                    let stack = self.stacks[pos];
                    for height in 0..self.stack_heights[pos] {
                        if stack & (1 << height) != 0 {
                            result.push('2');
                        } else {
                            result.push('1');
                        }
                    }
                    if self.walls & (1u64 << pos) != 0 {
                        result.push('S');
                    } else if self.capstones & (1u64 << pos) != 0 {
                        result.push('C');
                    }
                }
            }
            if empty_count == self.size {
                result.push_str(&format!("x{}", empty_count));
            } else if empty_count > 0 {
                result.push_str(&format!(",x{}", empty_count));
            }
        }

        let move_index = (self.ply_index / 2) + 1;
        let player_index = if self.current_player == 0 { "1" } else { "2" };

        result.push_str(&format!(" {} {}", player_index, move_index));

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty() {
        let board = Board::empty(5);
        assert_eq!(board.size, 5);
    }

    #[test]
    fn test_place() {
        let mut board = Board::try_from_pos_str("x5/x5/x5/x5/x5 1 25").unwrap();
        board.place(0, 0);
        board.place(1, 0);
        board.place(2, 1);
        board.place(3, 1);
        board.place(4, 2);
        board.place(5, 2);
        assert_eq!(board.ply_index, (25 - 1) * 2 + 6);
        assert_eq!(board.current_player, 0);
        assert_eq!(board.white_pieces, 19);
        assert_eq!(board.black_pieces, 19);
        assert_eq!(board.white_capstones, 0);
        assert_eq!(board.black_capstones, 0);
        assert_eq!(board.capstones, 0b110000);
        assert_eq!(board.walls, 0b001100);
        assert_eq!(board.occupied, 0b111111);
        assert_eq!(board.owner, 0b101010);
        assert_eq!(
            board.stack_heights,
            vec![1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            board.stacks,
            vec![0, 1, 0, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_unplace() {
        let mut board = Board::empty(5);
        board.place(0, 0);
        board.place(1, 0);
        board.place(2, 1);
        board.place(3, 1);
        board.place(4, 2);
        board.place(5, 2);
        board.unplace(5, 2);
        board.unplace(4, 2);
        board.unplace(3, 1);
        board.unplace(2, 1);
        board.unplace(1, 0);
        board.unplace(0, 0);
        assert_eq!(board, Board::empty(5));
    }

    #[test]
    fn test_spread() {
        let mut board = Board::try_from_pos_str("x3/x3/x3 1 25").unwrap();
        board.place(0, 0);
        board.place(1, 0);

        board.spread(0, 0, 1, 0x1);
        assert_eq!(board.capstones, 0);
        assert_eq!(board.walls, 0);
        assert_eq!(board.occupied, 0b10);
        assert_eq!(board.stack_heights, vec![0, 2, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(board.stacks, vec![0, 1, 0, 0, 0, 0, 0, 0, 0]);

        board.spread(1, 0, 2, 0x11);
        assert_eq!(board.occupied, 0b1100);
        assert_eq!(board.stack_heights, vec![0, 0, 1, 1, 0, 0, 0, 0, 0]);
        assert_eq!(board.stacks, vec![0, 0, 1, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_unspread() {
        let mut board = Board::try_from_pos_str("1,2,2C,x2/x2,2S,x2/x5/x5/x5 2 3").unwrap();
        let clone = board.clone();

        board.spread(0, Board::DIR_RIGHT, 1, 0x1);
        assert_eq!(board.to_pos_str(), "x,21,2C,x2/x2,2S,x2/x5/x5/x5 1 4");

        board.spread(1, Board::DIR_DOWN, 2, 0x11);
        assert!(board.spread(2, Board::DIR_DOWN, 1, 0x1));

        assert_eq!(board.to_pos_str(), "x5/x,2,22C,x2/x,1,x3/x5/x5 1 5");
        assert_eq!(
            board,
            Board::try_from_pos_str("x5/x,2,22C,x2/x,1,x3/x5/x5 1 5").unwrap()
        );

        board.unspread(2, 2, 0x1, true);
        board.unspread(1, 2, 0x11, false);
        board.unspread(0, 0, 0x1, false);
        assert_eq!(board, clone);
    }

    #[test]
    fn test_from_pos_str() {
        let board = Board::try_from_pos_str("x3/x3/x3 1 1");
        assert_eq!(board, Some(Board::empty(3)));

        let board = Board::try_from_pos_str("x,1121S,1/x2,11S/x3 2 3");
        assert_eq!(
            board,
            Some(Board {
                size: 3,
                result: None,
                ply_index: 5,
                current_player: 1,
                empty_positions: 6,
                white_pieces: 4,
                white_capstones: 0,
                black_pieces: 9,
                black_capstones: 0,
                occupied: 0b000100110,
                walls: 0b000100010,
                capstones: 0b000000000,
                owner: 0,
                stack_heights: vec![0, 4, 1, 0, 0, 2, 0, 0, 0],
                stacks: vec![0, 0b0100, 0, 0, 0, 0, 0, 0, 0]
            })
        );

        let board = Board::try_from_pos_str("x3,1121C,22C/x4,11S/x5/x5/x5 2 3").unwrap();
        assert_eq!(board.white_pieces, 21 - 4);
        assert_eq!(board.white_capstones, 1 - 1);
        assert_eq!(board.black_pieces, 21 - 2);
        assert_eq!(board.black_capstones, 1 - 1);
    }

    #[test]
    fn test_to_pos_str() {
        let cases = [
            "x3/x3/x3 1 1",
            "x,1121S,1/x2,11S/x3 2 22",
            "x2,2C,1121C,1/x4,11S/x5/x5/x5 2 22",
            "x3,222S,1C/x4,21S/x5/x5/x5 2 3",
        ];
        for &case in &cases {
            let board = Board::try_from_pos_str(case).unwrap();
            assert_eq!(board.to_pos_str(), case);
        }
    }

    #[test]
    fn test_invalid_from_pos_str() {
        assert!(Board::try_from_pos_str("x2,,x/x3/x3 1 1").is_none());
        assert!(Board::try_from_pos_str("x3/x3/x3 1").is_none());
        assert!(Board::try_from_pos_str("x3/x3/x3 1 1 1").is_none());
        assert!(Board::try_from_pos_str("x3/x3/x4 1 1").is_none());
        assert!(Board::try_from_pos_str("x,1121C,1/x2,11S/x3/x2 2 3").is_none());
        assert!(Board::try_from_pos_str("x,C,1/x3/x3 2 3").is_none());
        assert!(Board::try_from_pos_str("x,2,1,1/x3/x3 2 3").is_none());

        assert!(Board::try_from_pos_str("x3,2C,1S/x4,21C/x5/x5/x5 2 3").is_some());
        assert!(Board::try_from_pos_str("x3,2C,1C/x4,21C/x5/x5/x5 2 3").is_none());

        assert!(Board::try_from_pos_str("x3,222S,1C/x4,21S/x5/x5/x5 2 3").is_some());
        assert!(Board::try_from_pos_str("x3,222S,1C/x4,21C/x5/x5/x5 2 3").is_none());
    }

    #[test]
    fn test_road_win() {
        let mut board = Board::try_from_pos_str("1,1,x/x3/x3 1 10").unwrap();
        assert_eq!(board.result, None);
        board.place(2, Board::VARIANT_FLAT);
        assert_eq!(board.result, Some(Board::PLAYER_WHITE));

        let mut board = Board::try_from_pos_str("1,1,x/x3/x3 1 10").unwrap();
        assert_eq!(board.result, None);
        board.place(2, Board::VARIANT_WALL);
        assert_eq!(board.result, None);

        let mut board = Board::try_from_pos_str("x,1,x/1,221,x/x,2121S,x 1 10").unwrap();
        assert_eq!(board.result, None);
        board.place(2, Board::VARIANT_FLAT);
        assert_eq!(board.result, Some(Board::PLAYER_WHITE));

        let mut board = Board::try_from_pos_str("1,1,x/2,2,x/x2,121 1 10").unwrap();
        assert_eq!(board.result, None);
        board.spread(8, Board::DIR_UP, 3, 0x12);
        assert_eq!(board.result, Some(Board::PLAYER_WHITE));

        let mut board = Board::try_from_pos_str("1,1S,x/2,2,x/x2,121 1 10").unwrap();
        assert_eq!(board.result, None);
        board.spread(8, Board::DIR_UP, 3, 0x12);
        assert_eq!(board.result, Some(Board::PLAYER_BLACK));
    }

    #[test]
    fn test_flat_win() {
        let mut board = Board::try_from_pos_str("2,1,x/1,2,1/2,1,2 1 10").unwrap();
        assert_eq!(board.result, None);
        board.place(2, Board::VARIANT_FLAT);
        assert_eq!(board.result, Some(Board::PLAYER_WHITE));

        let mut board = Board::try_from_pos_str("2,1,x/1,2,1/2,1,2 2 10").unwrap();
        assert_eq!(board.result, None);
        board.place(2, Board::VARIANT_FLAT);
        assert_eq!(board.result, Some(Board::PLAYER_BLACK));

        let mut board = Board::try_from_pos_str("2S,1,x/1,2,1/2,1,2 2 10").unwrap();
        assert_eq!(board.result, None);
        board.place(2, Board::VARIANT_FLAT);
        assert_eq!(board.result, Some(2));

        let mut board = Board::try_from_pos_str("2S,1,x/1S,2S,1/2S,1,2 2 10").unwrap();
        assert_eq!(board.result, None);
        board.place(2, Board::VARIANT_FLAT);
        assert_eq!(board.result, Some(Board::PLAYER_WHITE));
    }
}
