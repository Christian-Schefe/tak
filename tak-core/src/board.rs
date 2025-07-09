use crate::{
    TakCoord, TakDir, TakInvalidMoveError, TakInvalidPlaceError, TakInvalidUndoMoveError,
    TakInvalidUndoPlaceError, TakPieceVariant, TakPlayer,
};

#[derive(Debug, Clone, PartialEq)]
pub struct TakPiece {
    pub id: usize,
    pub player: TakPlayer,
}

impl TakPiece {
    pub fn new(id: usize, player: TakPlayer) -> Self {
        TakPiece { id, player }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TakTower {
    pub variant: TakPieceVariant,
    pub composition: Vec<TakPiece>,
}

impl TakTower {
    pub fn new(variant: TakPieceVariant, composition: Vec<TakPiece>) -> Self {
        TakTower {
            variant,
            composition,
        }
    }

    pub fn height(&self) -> usize {
        self.composition.len()
    }

    pub fn player(&self) -> TakPlayer {
        self.composition
            .last()
            .expect("TakTower should not be empty")
            .player
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TakBoard {
    pub size: usize,
    board: Vec<Option<TakTower>>,
    id_counter: usize,
    empty_spaces: usize,
}

impl TakBoard {
    pub fn new(size: usize) -> Self {
        TakBoard {
            size,
            board: vec![None; size * size],
            id_counter: 0,
            empty_spaces: size * size,
        }
    }

    pub fn can_place(&self, pos: TakCoord) -> Result<(), TakInvalidPlaceError> {
        match pos.try_get(&self.board, self.size) {
            Some(None) => Ok(()),
            Some(Some(_)) => Err(TakInvalidPlaceError::PositionOccupied),
            None => Err(TakInvalidPlaceError::InvalidPosition),
        }
    }

    pub fn try_place(
        &mut self,
        pos: TakCoord,
        variant: TakPieceVariant,
        player: TakPlayer,
    ) -> Result<(), TakInvalidPlaceError> {
        self.can_place(pos)?;
        self.do_place_unchecked(pos, variant, player);
        Ok(())
    }

    pub fn do_place_unchecked(
        &mut self,
        pos: TakCoord,
        variant: TakPieceVariant,
        player: TakPlayer,
    ) {
        let tower = TakTower::new(variant, vec![TakPiece::new(self.id_counter, player)]);
        self.id_counter += 1;
        self.empty_spaces -= 1;
        *pos.get_mut(&mut self.board, self.size) = Some(tower);
    }

    pub fn can_undo_place(
        &mut self,
        pos: TakCoord,
        variant: TakPieceVariant,
        player: TakPlayer,
    ) -> Result<(), TakInvalidUndoPlaceError> {
        match pos.try_get_mut(&mut self.board, self.size) {
            Some(Some(tower)) => {
                if tower.variant == variant
                    && tower.composition.len() == 1
                    && tower.player() == player
                    && tower.composition[0].id + 1 == self.id_counter
                {
                    Ok(())
                } else {
                    Err(TakInvalidUndoPlaceError::NotAllowed)
                }
            }
            Some(None) => Err(TakInvalidUndoPlaceError::PositionEmpty),
            None => Err(TakInvalidUndoPlaceError::InvalidPosition),
        }
    }

    pub fn try_undo_place(
        &mut self,
        pos: TakCoord,
        variant: TakPieceVariant,
        player: TakPlayer,
    ) -> Result<(), TakInvalidUndoPlaceError> {
        self.can_undo_place(pos, variant, player)?;
        self.undo_place_unchecked(pos);
        Ok(())
    }

    pub fn undo_place_unchecked(&mut self, pos: TakCoord) {
        let tower = pos.get_mut(&mut self.board, self.size);
        *tower = None;
        self.id_counter -= 1;
        self.empty_spaces += 1;
    }

    pub fn can_move(
        &self,
        pos: TakCoord,
        dir: TakDir,
        take: usize,
        drops: &[usize],
    ) -> Result<bool, TakInvalidMoveError> {
        if take < 1 || take > self.size {
            return Err(TakInvalidMoveError::InvalidTakeCount);
        }
        let tower = match pos.try_get(&self.board, self.size) {
            Some(Some(tower)) => tower,
            Some(None) => return Err(TakInvalidMoveError::InvalidPosition),
            None => return Err(TakInvalidMoveError::PositionEmpty),
        };
        if tower.height() < take {
            return Err(TakInvalidMoveError::NotEnoughPieces);
        }
        let drop_len = drops.len();
        let mut drop_sum = 0;
        let mut current_pos = pos;
        let mut is_flattening = false;
        for i in 0..drop_len {
            let drop = drops[i];
            drop_sum += drop;
            if drop < 1 || drop_sum > take {
                return Err(TakInvalidMoveError::InvalidDropCount);
            }
            current_pos = current_pos.offset_dir(dir);
            match current_pos.try_get(&self.board, self.size) {
                Some(None) => {}
                Some(Some(other_tower)) => match other_tower.variant {
                    TakPieceVariant::Flat => {}
                    TakPieceVariant::Capstone => {
                        return Err(TakInvalidMoveError::Blocked);
                    }
                    TakPieceVariant::Wall => {
                        if i != drop_len - 1
                            || drop != 1
                            || tower.variant != TakPieceVariant::Capstone
                        {
                            return Err(TakInvalidMoveError::Blocked);
                        } else {
                            is_flattening = true;
                        }
                    }
                },
                None => return Err(TakInvalidMoveError::InvalidDirection),
            }
        }
        if drop_sum != take {
            return Err(TakInvalidMoveError::InvalidDropCount);
        }
        Ok(is_flattening)
    }

    pub fn try_move(
        &mut self,
        pos: TakCoord,
        dir: TakDir,
        take: usize,
        drops: &[usize],
    ) -> Result<bool, TakInvalidMoveError> {
        let is_flattening = self.can_move(pos, dir, take, drops)?;
        self.do_move_unchecked(pos, dir, take, drops);
        Ok(is_flattening)
    }

    pub fn do_move_unchecked(&mut self, pos: TakCoord, dir: TakDir, take: usize, drops: &[usize]) {
        let drop_len = drops.len();
        let tile = pos.get_mut(&mut self.board, self.size);
        let tower = tile.as_mut().expect("Tile should contain a tower");
        let variant = tower.variant;
        let mut moved_pieces = if tower.height() == take {
            self.empty_spaces += 1;
            let mut composition = tile
                .take()
                .expect("Tile should contain a tower")
                .composition;
            composition.reverse();
            composition
        } else {
            tower.variant = TakPieceVariant::Flat;
            tower
                .composition
                .drain(tower.composition.len() - take..)
                .rev()
                .collect::<Vec<_>>()
        };
        let mut current_pos = pos;
        for i in 0..drop_len {
            current_pos = current_pos.offset_dir(dir);
            let moved_pieces = moved_pieces.drain(moved_pieces.len() - drops[i]..).rev();
            let other_tile = current_pos.get_mut(&mut self.board, self.size);
            let new_variant = if i == drop_len - 1 {
                variant
            } else {
                TakPieceVariant::Flat
            };
            match other_tile {
                None => {
                    *other_tile = Some(TakTower::new(new_variant, moved_pieces.collect()));
                    self.empty_spaces -= 1;
                }
                Some(other_tower) => {
                    other_tower.composition.extend(moved_pieces);
                    other_tower.variant = new_variant;
                }
            }
        }
    }

    pub fn can_undo_move(
        &mut self,
        pos: TakCoord,
        dir: TakDir,
        take: usize,
        drops: &[usize],
        flattened: bool,
    ) -> Result<(), TakInvalidUndoMoveError> {
        if take < 1 || take > self.size {
            return Err(TakInvalidUndoMoveError::InvalidTakeCount);
        }
        match pos.try_get(&self.board, self.size) {
            Some(Some(tower)) => {
                if tower.variant != TakPieceVariant::Flat {
                    return Err(TakInvalidUndoMoveError::ActionMismatch);
                }
            }
            Some(None) => {}
            None => return Err(TakInvalidUndoMoveError::InvalidPosition),
        }
        let drop_len = drops.len();
        let mut drop_sum = 0;
        let mut current_pos = pos;
        for i in 0..drop_len {
            let drop = drops[i];
            drop_sum += drop;
            if drop < 1 || drop_sum > take {
                return Err(TakInvalidUndoMoveError::InvalidDropCount);
            }
            current_pos = current_pos.offset_dir(dir);
            match current_pos.try_get(&self.board, self.size) {
                Some(Some(other_tower)) => {
                    if other_tower.height() < drop {
                        return Err(TakInvalidUndoMoveError::InvalidDropCount);
                    }
                    if i != drop_len - 1 {
                        match other_tower.variant {
                            TakPieceVariant::Flat => {}
                            TakPieceVariant::Capstone => {
                                return Err(TakInvalidUndoMoveError::ActionMismatch);
                            }
                            TakPieceVariant::Wall => {
                                return Err(TakInvalidUndoMoveError::ActionMismatch);
                            }
                        };
                    } else if flattened
                        && (other_tower.variant != TakPieceVariant::Capstone
                            || other_tower.height() < 2)
                    {
                        return Err(TakInvalidUndoMoveError::ActionMismatch);
                    }
                }
                Some(None) => return Err(TakInvalidUndoMoveError::ActionMismatch),
                None => return Err(TakInvalidUndoMoveError::InvalidPosition),
            }
        }
        if drop_sum != take {
            return Err(TakInvalidUndoMoveError::InvalidDropCount);
        }
        Ok(())
    }

    pub fn try_undo_move(
        &mut self,
        pos: TakCoord,
        dir: TakDir,
        take: usize,
        drops: &[usize],
        flattened: bool,
    ) -> Result<(), TakInvalidUndoMoveError> {
        self.can_undo_move(pos, dir, take, drops, flattened)?;
        self.undo_move_unchecked(pos, dir, drops, flattened);
        Ok(())
    }

    fn undo_move_unchecked(
        &mut self,
        pos: TakCoord,
        dir: TakDir,
        drops: &[usize],
        flattened: bool,
    ) {
        let drop_len = drops.len();
        let mut current_pos = pos;
        let mut moved_pieces = Vec::new();
        let mut original_variant = TakPieceVariant::Flat;
        for i in 0..drop_len {
            current_pos = current_pos.offset_dir(dir);
            let tile = current_pos.get_mut(&mut self.board, self.size);
            let tower = tile.as_mut().expect("Tile should contain a tower");
            if i == drop_len - 1 {
                original_variant = tower.variant;
            }

            let pieces_to_move = tower
                .composition
                .drain(tower.composition.len() - drops[i]..);
            moved_pieces.extend(pieces_to_move);
            if tower.composition.is_empty() {
                *tile = None;
                self.empty_spaces += 1;
            } else {
                tower.variant = if i == drop_len - 1 && flattened {
                    TakPieceVariant::Wall
                } else {
                    TakPieceVariant::Flat
                };
            }
        }
        let tile = pos.get_mut(&mut self.board, self.size);
        match tile {
            Some(original_tower) => {
                original_tower.composition.extend(moved_pieces);
                original_tower.variant = original_variant;
            }
            None => {
                *tile = Some(TakTower::new(original_variant, moved_pieces));
                self.empty_spaces -= 1;
            }
        }
    }

    pub fn to_partial_tps(&self) -> String {
        let mut tps = String::new();
        for y in 0..self.size {
            let mut empty_count = 0;
            for x in 0..self.size {
                let pos = TakCoord::new(x as i32, y as i32);
                match pos.get(&self.board, self.size) {
                    Some(tower) => {
                        match empty_count {
                            0 => {}
                            1 => tps.push_str("x,"),
                            _ => {
                                tps.push_str(format!("x{},", empty_count).as_str());
                            }
                        }
                        empty_count = 0;
                        let composition_chars = tower
                            .composition
                            .iter()
                            .map(|p| match p.player {
                                TakPlayer::White => '1',
                                TakPlayer::Black => '2',
                            })
                            .collect::<String>();
                        tps.push_str(&composition_chars);
                        match tower.variant {
                            TakPieceVariant::Flat => {}
                            TakPieceVariant::Wall => tps.push('S'),
                            TakPieceVariant::Capstone => tps.push('C'),
                        };
                        if x < self.size - 1 {
                            tps.push(',');
                        }
                    }
                    _ => empty_count += 1,
                }
            }
            match empty_count {
                0 => {}
                1 => tps.push('x'),
                _ => {
                    tps.push_str(format!("x{}", empty_count).as_str());
                }
            }
            if y < self.size - 1 {
                tps.push('/');
            }
        }
        tps
    }

    pub fn try_from_partial_tps(tps: &str) -> Option<Self> {
        let mut size = None;
        let mut board = Vec::new();
        let mut id_counter = 0;
        let mut empty_spaces = 0;
        for line in tps.split('/') {
            let mut x = 0;
            for part in line.split(',') {
                if part.starts_with('x') {
                    let empty_count: usize = part[1..].parse().unwrap_or(1);
                    for _ in 0..empty_count {
                        board.push(None);
                    }
                    x += empty_count;
                    empty_spaces += empty_count;
                } else {
                    let mut composition = Vec::new();
                    let mut variant = TakPieceVariant::Flat;
                    for c in part.chars() {
                        match c {
                            '1' => {
                                composition.push(TakPiece::new(id_counter, TakPlayer::White));
                                id_counter += 1;
                            }
                            '2' => {
                                composition.push(TakPiece::new(id_counter, TakPlayer::Black));
                                id_counter += 1;
                            }
                            'S' => variant = TakPieceVariant::Wall,
                            'C' => variant = TakPieceVariant::Capstone,
                            _ => continue,
                        }
                    }
                    board.push(Some(TakTower::new(variant, composition)));
                    x += 1;
                }
            }
            match size {
                None => size = Some(x),
                Some(s) => {
                    if s != x {
                        return None;
                    }
                }
            }
        }
        let size = size?;
        if board.len() != size * size {
            return None;
        }
        Some(TakBoard {
            size,
            board,
            id_counter,
            empty_spaces,
        })
    }

    pub fn check_for_road(
        &self,
        positions: &[TakCoord],
        player: TakPlayer,
    ) -> Option<(TakCoord, TakCoord)> {
        let mut visited = vec![false; self.size * self.size];
        let max_pos_val = self.size as i32 - 1;
        for &pos in positions {
            let mut stack = vec![pos];
            let mut found_left = None;
            let mut found_right = None;
            let mut found_top = None;
            let mut found_bottom = None;
            while let Some(current_pos) = stack.pop() {
                match current_pos.try_get_mut(&mut visited, self.size) {
                    Some(x) if !*x => *x = true,
                    _ => continue,
                }
                let Some(Some(tower)) = current_pos.try_get(&self.board, self.size) else {
                    continue;
                };
                if tower.player() != player || tower.variant == TakPieceVariant::Wall {
                    continue;
                }

                if current_pos.x == 0 && found_left.is_none() {
                    found_left = Some(current_pos);
                } else if current_pos.x == max_pos_val && found_right.is_none() {
                    found_right = Some(current_pos);
                }
                if current_pos.y == 0 && found_top.is_none() {
                    found_top = Some(current_pos);
                } else if current_pos.y == max_pos_val && found_bottom.is_none() {
                    found_bottom = Some(current_pos);
                }

                if found_left.is_some() && found_right.is_some() {
                    return Some((found_left.unwrap(), found_right.unwrap()));
                } else if found_top.is_some() && found_bottom.is_some() {
                    return Some((found_top.unwrap(), found_bottom.unwrap()));
                }

                TakDir::ALL.iter().for_each(|&dir| {
                    stack.push(current_pos.offset_dir(dir));
                });
            }
        }
        None
    }

    pub fn has_empty_space(&self) -> bool {
        self.empty_spaces > 0
    }

    pub fn count_stones(&self, player: TakPlayer) -> (usize, usize) {
        let mut stone_count = 0;
        let mut capstone_count = 0;
        for tile in &self.board {
            if let Some(tower) = tile {
                for i in 0..tower.composition.len() {
                    if tower.composition[i].player != player {
                        continue;
                    }
                    if tower.variant == TakPieceVariant::Capstone
                        && i == tower.composition.len() - 1
                    {
                        capstone_count += 1;
                    } else {
                        stone_count += 1;
                    }
                }
            }
        }
        (stone_count, capstone_count)
    }

    pub fn count_flats(&self) -> [usize; 2] {
        let mut counts = [0, 0];
        for tile in &self.board {
            if let Some(tower) = tile {
                if tower.variant == TakPieceVariant::Flat {
                    for piece in &tower.composition {
                        counts[piece.player.index()] += 1;
                    }
                }
            }
        }
        counts
    }

    pub fn iter_pieces(&self, player: TakPlayer) -> impl Iterator<Item = (TakCoord, &TakTower)> {
        TakCoord::iter_board(self.size).filter_map(move |pos| {
            if let Some(tower) = pos.get(&self.board, self.size) {
                if tower.player() == player {
                    Some((pos, tower))
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    pub fn try_get_tower(&self, pos: TakCoord) -> Option<&TakTower> {
        pos.try_get(&self.board, self.size).and_then(|x| x.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_board_is_empty() {
        let board = TakBoard::new(5);
        for y in 0..5 {
            for x in 0..5 {
                let pos = TakCoord::new(x as i32, y as i32);
                assert!(board.can_place(pos).is_ok());
            }
        }
    }

    #[test]
    fn test_place_piece() {
        let mut board = TakBoard::new(3);
        let pos = TakCoord::new(1, 1);
        assert!(board
            .try_place(pos, TakPieceVariant::Flat, TakPlayer::White)
            .is_ok());
        assert!(board.can_place(pos).is_err());
        assert!(board
            .try_place(pos, TakPieceVariant::Wall, TakPlayer::White)
            .is_err());
    }

    #[test]
    fn test_can_move_simple() {
        let mut board = TakBoard::new(3);
        let pos = TakCoord::new(0, 0);
        assert!(board
            .try_place(pos, TakPieceVariant::Flat, TakPlayer::White)
            .is_ok());
        assert!(board.can_move(pos, TakDir::Right, 1, &[1]).is_ok());
    }

    #[test]
    fn test_try_move_simple() {
        let mut board = TakBoard::new(3);
        let pos = TakCoord::new(0, 0);
        assert!(board
            .try_place(pos, TakPieceVariant::Flat, TakPlayer::White)
            .is_ok());
        assert!(board.try_move(pos, TakDir::Right, 1, &[1]).is_ok());
        let new_pos = TakCoord::new(1, 0);
        assert!(board.can_place(new_pos).is_err());
    }

    #[test]
    fn test_to_tps_empty() {
        let board = TakBoard::new(3);
        assert_eq!(board.to_partial_tps(), "x3/x3/x3");
    }

    #[test]
    fn test_from_tps_empty() {
        let board = TakBoard::try_from_partial_tps("x3/x3/x3").unwrap();
        assert_eq!(board.size, 3);
        for y in 0..3 {
            for x in 0..3 {
                let pos = TakCoord::new(x as i32, y as i32);
                assert!(board.can_place(pos).is_ok());
            }
        }
    }

    #[test]
    fn test_to_tps_with_pieces() {
        let mut board = TakBoard::new(2);
        assert!(board
            .try_place(TakCoord::new(0, 0), TakPieceVariant::Flat, TakPlayer::White)
            .is_ok());
        assert!(board
            .try_place(TakCoord::new(1, 0), TakPieceVariant::Wall, TakPlayer::Black)
            .is_ok());
        assert_eq!(board.to_partial_tps(), "1,2S/x2");
    }

    #[test]
    fn test_try_move_multiple() {
        let tps = "12211C,2S,x/x3/x3";
        let mut board = TakBoard::try_from_partial_tps(tps).unwrap();
        assert!(board
            .try_move(TakCoord::new(0, 0), TakDir::Down, 3, &[3])
            .is_ok());
        assert_eq!(board.to_partial_tps(), "12,2S,x/211C,x2/x3");
    }

    #[test]
    fn test_try_move_multiple2() {
        let tps = "x,2S,x/21C,x2/x3";
        let mut board = TakBoard::try_from_partial_tps(tps).unwrap();
        assert!(board
            .try_move(TakCoord::new(0, 1), TakDir::Up, 2, &[2])
            .is_ok());
        assert_eq!(board.to_partial_tps(), "21C,2S,x/x3/x3");
    }

    #[test]
    fn test_from_tps_with_pieces() {
        let mut board = TakBoard::new(3);
        let pos = TakCoord::new(0, 0);
        assert!(board
            .try_place(pos, TakPieceVariant::Flat, TakPlayer::White)
            .is_ok());
        assert!(board.try_move(pos, TakDir::Right, 1, &[1]).is_ok());
        let new_pos = TakCoord::new(1, 0);
        assert!(board.can_place(new_pos).is_err());
    }

    #[test]
    fn test_try_move_over_wall_with_capstone() {
        let mut board = TakBoard::new(3);
        let pos = TakCoord::new(0, 0);
        assert!(board
            .try_place(pos, TakPieceVariant::Capstone, TakPlayer::White)
            .is_ok());
        let wall_pos = TakCoord::new(1, 0);
        assert!(board
            .try_place(wall_pos, TakPieceVariant::Wall, TakPlayer::Black)
            .is_ok());
        assert!(board.can_move(pos, TakDir::Right, 1, &[1]).is_ok());
        assert!(board.try_move(pos, TakDir::Right, 1, &[1]).is_ok());
    }

    #[test]
    fn test_try_move_over_capstone_blocked() {
        let mut board = TakBoard::new(3);
        let pos = TakCoord::new(0, 0);
        assert!(board
            .try_place(pos, TakPieceVariant::Flat, TakPlayer::White)
            .is_ok());
        let cap_pos = TakCoord::new(1, 0);
        assert!(board
            .try_place(cap_pos, TakPieceVariant::Capstone, TakPlayer::Black)
            .is_ok());
        assert!(board.can_move(pos, TakDir::Right, 1, &[1]).is_err());
        assert!(board.try_move(pos, TakDir::Right, 1, &[1]).is_err());
    }

    #[test]
    fn test_try_undo_place() {
        let mut board = TakBoard::try_from_partial_tps("x,1221C,x,21,2S/x5/x5/x5/x5").unwrap();
        assert!(board
            .try_place(TakCoord::new(2, 0), TakPieceVariant::Wall, TakPlayer::White)
            .is_ok());
        println!("board: {board:?}");
        assert_eq!(board.to_partial_tps(), "x,1221C,1S,21,2S/x5/x5/x5/x5");
        assert!(board
            .try_undo_place(TakCoord::new(2, 0), TakPieceVariant::Wall, TakPlayer::White)
            .is_ok());
        assert_eq!(board.to_partial_tps(), "x,1221C,x,21,2S/x5/x5/x5/x5");
    }

    #[test]
    fn test_try_undo_move() {
        let mut board = TakBoard::try_from_partial_tps("x,1221C,x,21,2S/x5/x5/x5/x5").unwrap();
        assert!(board
            .try_move(TakCoord::new(1, 0), TakDir::Right, 3, &[1, 1, 1])
            .is_ok());
        assert_eq!(board.to_partial_tps(), "x,1,2,212,21C/x5/x5/x5/x5");
        assert!(board
            .try_undo_move(TakCoord::new(1, 0), TakDir::Right, 3, &[1, 1, 1], true)
            .is_ok());
        assert_eq!(board.to_partial_tps(), "x,1221C,x,21,2S/x5/x5/x5/x5");
    }
}
