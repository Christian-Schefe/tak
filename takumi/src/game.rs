#[derive(Debug, Clone, PartialEq)]
pub struct Board {
    size: usize,
    occupied: u64,
    walls: u64,
    capstones: u64,
    stacks: Vec<u64>,
    stack_heights: Vec<u64>,
}

impl Board {
    pub fn empty(size: usize) -> Self {
        Self {
            size,
            occupied: 0,
            walls: 0,
            capstones: 0,
            stacks: vec![0; size * size],
            stack_heights: vec![0; size * size],
        }
    }

    pub fn place(&mut self, pos: usize, player: u64, variant: usize) {
        let pos_mask = 1u64 << pos;

        self.occupied |= pos_mask;
        if variant == 1 {
            self.walls |= pos_mask;
        } else {
            self.walls &= !pos_mask;
        }
        if variant == 2 {
            self.capstones |= pos_mask;
        } else {
            self.capstones &= !pos_mask;
        }
        self.stack_heights[pos] = 1;
        self.stacks[pos] = player;
    }

    pub fn unplace(&mut self, pos: usize) {
        let not_pos_mask = !(1u64 << pos);

        self.occupied &= not_pos_mask;
        self.walls &= not_pos_mask;
        self.capstones &= not_pos_mask;
        self.stack_heights[pos] = 0;
        self.stacks[pos] = 0;
    }

    pub fn spread(&mut self, pos: usize, dir: usize, take: u64, spreads: [u64; 7]) -> bool {
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

        if self.stack_heights[pos] == 0 {
            self.occupied &= not_pos_mask;
        }

        let mut cur_pos = pos;
        let mut cur_pos_mask = pos_mask;

        for i in 0..7 {
            let spread = spreads[i];
            if spread == 0 {
                break;
            }
            match dir {
                0 => cur_pos += 1,
                1 => cur_pos -= 1,
                2 => cur_pos += self.size,
                3 => cur_pos -= self.size,
                _ => {}
            }
            cur_pos_mask = 1u64 << cur_pos;
            let prev_height = self.stack_heights[cur_pos];
            self.stack_heights[cur_pos] += spread;
            let this_pattern = took_pattern & ((1 << spread) - 1);
            took_pattern >>= spread;
            self.stacks[cur_pos] |= this_pattern << prev_height;
            self.occupied |= cur_pos_mask;
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

        smash
    }

    pub fn unspread(&mut self, pos: usize, dir: usize, spreads: [u64; 7], smash: bool) {
        let pos_mask = 1u64 << pos;

        let mut cur_pos = pos;
        let mut cur_pos_mask = pos_mask;

        let mut took_pattern = 0;
        let mut spread_sum = 0;

        for i in 0..7 {
            let spread = spreads[i];
            if spread == 0 {
                break;
            }
            match dir {
                0 => cur_pos += 1,
                1 => cur_pos -= 1,
                2 => cur_pos += self.size,
                3 => cur_pos -= self.size,
                _ => {}
            }
            cur_pos_mask = 1u64 << cur_pos;
            let prev_height = self.stack_heights[cur_pos];
            let new_height = prev_height - spread;
            self.stack_heights[cur_pos] = new_height;
            if new_height == 0 {
                self.occupied &= !cur_pos_mask;
            }

            let spread_mask = (1u64 << spread) - 1;
            let this_pattern = self.stacks[cur_pos] >> new_height;
            self.stacks[cur_pos] &= !(spread_mask << new_height);

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

        self.occupied |= pos_mask;
        let prev_height = self.stack_heights[pos];
        self.stack_heights[pos] += spread_sum;
        self.stacks[pos] |= took_pattern << prev_height;
    }

    pub fn try_from_pos_str(position: &str) -> Self {
        let mut occupied = 0;
        let mut walls = 0;
        let mut capstones = 0;
        let mut stack_heights = Vec::new();
        let mut stacks = Vec::new();
        let mut size = 0;

        let mut pos = 0;
        for row in position.split('/') {
            for part in row.split(',') {
                if part.starts_with('x') {
                    let amount = part[1..].parse::<usize>().unwrap_or(1);
                    pos += amount;
                    for _ in 0..amount {
                        stack_heights.push(0);
                        stacks.push(0);
                    }
                } else {
                    let pos_mask = 1u64 << pos;
                    occupied |= pos_mask;
                    if part.ends_with('S') {
                        walls |= pos_mask;
                    } else if part.ends_with('C') {
                        capstones |= pos_mask;
                    }
                    let mut height = 0;
                    let mut stack = 0;
                    let mut chars = part.chars();
                    while let Some(char) = chars.next() {
                        if char == '2' {
                            stack |= 1 << height;
                        } else if char != '1' {
                            break;
                        }
                        height += 1;
                    }
                    stack_heights.push(height);
                    stacks.push(stack);
                    pos += 1;
                }
            }
            if size == 0 {
                size = stack_heights.len();
            }
        }

        Self {
            size,
            occupied,
            walls,
            capstones,
            stack_heights,
            stacks,
        }
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
        let mut board = Board::empty(3);
        board.place(0, 0, 0);
        board.place(1, 1, 0);
        board.place(2, 0, 1);
        board.place(3, 1, 1);
        board.place(4, 0, 2);
        board.place(5, 1, 2);
        assert_eq!(board.capstones, 0b110000);
        assert_eq!(board.walls, 0b001100);
        assert_eq!(board.occupied, 0b111111);
        assert_eq!(board.stack_heights, vec![1, 1, 1, 1, 1, 1, 0, 0, 0]);
        assert_eq!(board.stacks, vec![0, 1, 0, 1, 0, 1, 0, 0, 0]);
    }

    #[test]
    fn test_unplace() {
        let mut board = Board::empty(3);
        board.place(0, 0, 0);
        board.place(1, 1, 0);
        board.place(2, 0, 1);
        board.place(3, 1, 1);
        board.place(4, 0, 2);
        board.place(5, 1, 2);
        board.unplace(0);
        board.unplace(1);
        board.unplace(2);
        board.unplace(3);
        board.unplace(4);
        board.unplace(5);
        assert_eq!(board, Board::empty(3));
    }

    #[test]
    fn test_spread() {
        let mut board = Board::empty(3);
        board.place(0, 0, 0);
        board.place(1, 1, 0);

        board.spread(0, 0, 1, [1, 0, 0, 0, 0, 0, 0]);
        assert_eq!(board.capstones, 0);
        assert_eq!(board.walls, 0);
        assert_eq!(board.occupied, 0b10);
        assert_eq!(board.stack_heights, vec![0, 2, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(board.stacks, vec![0, 1, 0, 0, 0, 0, 0, 0, 0]);

        board.spread(1, 0, 2, [1, 1, 0, 0, 0, 0, 0]);
        assert_eq!(board.occupied, 0b1100);
        assert_eq!(board.stack_heights, vec![0, 0, 1, 1, 0, 0, 0, 0, 0]);
        assert_eq!(board.stacks, vec![0, 0, 1, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_unspread() {
        let mut board = Board::empty(3);
        board.place(0, 0, 0);
        board.place(1, 1, 0);
        board.place(2, 1, 2);
        board.place(5, 1, 1);
        let clone = board.clone();

        board.spread(0, 0, 1, [1, 0, 0, 0, 0, 0, 0]);
        board.spread(1, 2, 2, [1, 1, 0, 0, 0, 0, 0]);
        assert!(board.spread(2, 2, 1, [1, 0, 0, 0, 0, 0, 0]));

        assert_eq!(board, Board::try_from_pos_str("x3/x,2,22C/x,1,x"));

        board.unspread(2, 2, [1, 0, 0, 0, 0, 0, 0], true);
        board.unspread(1, 2, [1, 1, 0, 0, 0, 0, 0], false);
        board.unspread(0, 0, [1, 0, 0, 0, 0, 0, 0], false);
        assert_eq!(board, clone);
    }

    #[test]
    fn test_from_pos_str() {
        let board = Board::try_from_pos_str("x3/x3/x3");
        assert_eq!(board, Board::empty(3));

        let board = Board::try_from_pos_str("x,1121C,1/x2,11S/x3");
        assert_eq!(
            board,
            Board {
                size: 3,
                occupied: 0b000100110,
                walls: 0b000100000,
                capstones: 0b000000010,
                stack_heights: vec![0, 4, 1, 0, 0, 2, 0, 0, 0],
                stacks: vec![0, 0b0100, 0, 0, 0, 0, 0, 0, 0]
            }
        );
    }
}
