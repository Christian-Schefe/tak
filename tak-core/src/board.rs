use std::collections::VecDeque;

use crate::{
    TakCoord, TakDir, TakInvalidMoveError, TakInvalidPlaceError, TakInvalidUndoMoveError,
    TakInvalidUndoPlaceError, TakPieceVariant, TakPlayer,
};

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakStack {
    pub variant: TakPieceVariant,
    pub composition: Vec<TakPiece>,
}

impl TakStack {
    pub fn new(variant: TakPieceVariant, composition: Vec<TakPiece>) -> Self {
        TakStack {
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
            .expect("TakStack should not be empty")
            .player
    }
}

/// Represents a Tak board with a specified size and a vector of stacks.
/// It provides methods to place, move, and undo moves of pieces on the board,
/// as well as to check for valid placements and moves.
/// It also includes methods to check for roads, count stones, and convert the board to a partial TPS (Tak Position String) representation.
/// The board is represented as a flat vector of optional `TakStack` instances, where each position can either be empty (`None`) or occupied by a `TakStack`.
/// Each piece can be identified by a unique ID.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakBoard {
    pub size: usize,
    board: Vec<Option<TakStack>>,
    id_counter: usize,
    empty_spaces: usize,
}

impl TakBoard {
    /// Creates an empty TakBoard with the given size.
    pub fn new(size: usize) -> Self {
        TakBoard {
            size,
            board: vec![None; size * size],
            id_counter: 0,
            empty_spaces: size * size,
        }
    }

    /// Resets the board to its initial state, clearing all pieces and resetting the ID counter.
    pub fn reset(&mut self) {
        self.board.fill(None);
        self.id_counter = 0;
        self.empty_spaces = self.size * self.size;
    }

    /// Checks if a piece can be placed at the given position.
    /// Returns `Ok(())` if the position is valid and empty, or an error if the position is occupied or invalid.
    pub fn can_place(&self, pos: TakCoord) -> Result<(), TakInvalidPlaceError> {
        match pos.try_get(&self.board, self.size) {
            Some(None) => Ok(()),
            Some(Some(_)) => Err(TakInvalidPlaceError::PositionOccupied),
            None => Err(TakInvalidPlaceError::InvalidPosition),
        }
    }

    /// Attempts to place a piece at the given position.
    /// Returns `Ok(())` if the placement is successful, or an error if it fails.
    /// If the placement fails, the board state remains unchanged.
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

    /// Places a piece at the given position without checking if the placement is valid.
    /// Using this method can lead to an invalid board state or panic if the position is occupied or not valid.
    pub fn do_place_unchecked(
        &mut self,
        pos: TakCoord,
        variant: TakPieceVariant,
        player: TakPlayer,
    ) {
        let stack = TakStack::new(variant, vec![TakPiece::new(self.id_counter, player)]);
        self.id_counter += 1;
        self.empty_spaces -= 1;
        *pos.get_mut(&mut self.board, self.size) = Some(stack);
    }

    /// Checks if the placement of a piece at the given position can be undone.
    /// This requires that the piece is the last placed piece.
    pub fn can_undo_place(
        &mut self,
        pos: TakCoord,
        variant: TakPieceVariant,
        player: TakPlayer,
    ) -> Result<(), TakInvalidUndoPlaceError> {
        match pos.try_get_mut(&mut self.board, self.size) {
            Some(Some(stack)) => {
                if stack.variant == variant
                    && stack.composition.len() == 1
                    && stack.player() == player
                    && stack.composition[0].id + 1 == self.id_counter
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

    /// Attempts to undo the placement of a piece at the given position.
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

    /// Undoes the placement of a piece at the given position without checking if the undo is valid.
    /// Using this method can lead to an invalid board state or panic if the position or undo is not valid.
    pub fn undo_place_unchecked(&mut self, pos: TakCoord) {
        let stack = pos.get_mut(&mut self.board, self.size);
        *stack = None;
        self.id_counter -= 1;
        self.empty_spaces += 1;
    }

    /// Checks if a move can be made from the given position in the specified direction,
    /// taking a specified number of pieces and drops into account.
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
        let stack = match pos.try_get(&self.board, self.size) {
            Some(Some(stack)) => stack,
            Some(None) => return Err(TakInvalidMoveError::InvalidPosition),
            None => return Err(TakInvalidMoveError::PositionEmpty),
        };
        if stack.height() < take {
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
                Some(Some(other_stack)) => match other_stack.variant {
                    TakPieceVariant::Flat => {}
                    TakPieceVariant::Capstone => {
                        return Err(TakInvalidMoveError::Blocked);
                    }
                    TakPieceVariant::Wall => {
                        if i != drop_len - 1
                            || drop != 1
                            || stack.variant != TakPieceVariant::Capstone
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

    /// Attempts to move pieces from the given position in the specified direction,
    /// taking a specified number of pieces and drops into account.
    /// Returns true if the move results in a flattening, and false otherwise.
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

    /// Moves pieces from the given position in the specified direction,
    /// taking a specified number of pieces and drops into account.
    /// This method does not check if the move is valid and can lead to an invalid board state.
    pub fn do_move_unchecked(&mut self, pos: TakCoord, dir: TakDir, take: usize, drops: &[usize]) {
        let drop_len = drops.len();
        let tile = pos.get_mut(&mut self.board, self.size);
        let stack = tile.as_mut().expect("Tile should contain a stack");
        let variant = stack.variant;
        let mut moved_pieces = if stack.height() == take {
            self.empty_spaces += 1;
            let mut composition = tile
                .take()
                .expect("Tile should contain a stack")
                .composition;
            composition.reverse();
            composition
        } else {
            stack.variant = TakPieceVariant::Flat;
            stack
                .composition
                .drain(stack.composition.len() - take..)
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
                    *other_tile = Some(TakStack::new(new_variant, moved_pieces.collect()));
                    self.empty_spaces -= 1;
                }
                Some(other_stack) => {
                    other_stack.composition.extend(moved_pieces);
                    other_stack.variant = new_variant;
                }
            }
        }
    }

    /// Checks if a move can be undone from the given position in the specified direction,
    /// taking a specified number of pieces and drops into account.
    /// Returns an error if the move cannot be undone, or `Ok(())` if it can be undone.
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
            Some(Some(stack)) => {
                if stack.variant != TakPieceVariant::Flat {
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
                Some(Some(other_stack)) => {
                    if other_stack.height() < drop {
                        return Err(TakInvalidUndoMoveError::InvalidDropCount);
                    }
                    if i != drop_len - 1 {
                        match other_stack.variant {
                            TakPieceVariant::Flat => {}
                            TakPieceVariant::Capstone => {
                                return Err(TakInvalidUndoMoveError::ActionMismatch);
                            }
                            TakPieceVariant::Wall => {
                                return Err(TakInvalidUndoMoveError::ActionMismatch);
                            }
                        };
                    } else if flattened
                        && (other_stack.variant != TakPieceVariant::Capstone
                            || other_stack.height() < 2)
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

    /// Attempts to undo a move from the given position in the specified direction,
    /// taking a specified number of pieces and drops into account.
    /// Returns `Ok(())` if the undo is successful, or an error if it fails.
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

    /// Undoes a move from the given position in the specified direction,
    /// taking a specified number of pieces and drops into account.
    /// This method does not check if the undo is valid and can lead to an invalid board state.
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
            let stack = tile.as_mut().expect("Tile should contain a stack");
            if i == drop_len - 1 {
                original_variant = stack.variant;
            }

            moved_pieces.extend(
                stack
                    .composition
                    .drain(stack.composition.len() - drops[i]..),
            );
            if stack.composition.is_empty() {
                *tile = None;
                self.empty_spaces += 1;
            } else {
                stack.variant = if i == drop_len - 1 && flattened {
                    TakPieceVariant::Wall
                } else {
                    TakPieceVariant::Flat
                };
            }
        }
        let tile = pos.get_mut(&mut self.board, self.size);
        match tile {
            Some(original_stack) => {
                original_stack.composition.extend(moved_pieces);
                original_stack.variant = original_variant;
            }
            None => {
                *tile = Some(TakStack::new(original_variant, moved_pieces));
                self.empty_spaces -= 1;
            }
        }
    }

    /// Converts the board to a partial TPS (Tak Position String) representation.
    /// The TPS format is a string representation of the board state.
    /// The partial TPS format is a simplified version that only includes the occupied positions and their stack compositions.
    pub fn to_partial_tps(&self) -> String {
        let mut tps = String::new();
        for y in (0..self.size).rev() {
            let mut empty_count = 0;
            for x in 0..self.size {
                let pos = TakCoord::new(x as i32, y as i32);
                match pos.get(&self.board, self.size) {
                    Some(stack) => {
                        match empty_count {
                            0 => {}
                            1 => tps.push_str("x,"),
                            _ => {
                                tps.push_str(format!("x{},", empty_count).as_str());
                            }
                        }
                        empty_count = 0;
                        let composition_chars = stack
                            .composition
                            .iter()
                            .map(|p| match p.player {
                                TakPlayer::White => '1',
                                TakPlayer::Black => '2',
                            })
                            .collect::<String>();
                        tps.push_str(&composition_chars);
                        match stack.variant {
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
            if y > 0 {
                tps.push('/');
            }
        }
        tps
    }

    /// Attempts to create a TakBoard from a partial TPS string.
    /// The partial TPS format is a simplified version that only includes the occupied positions and their stack compositions.
    pub fn try_from_partial_tps(tps: &str) -> Option<Self> {
        let mut size = None;
        let mut board = Vec::new();
        let mut id_counter = 0;
        let mut empty_spaces = 0;
        for line in tps.split('/') {
            let mut x = 0;
            let mut row = match size {
                None => Vec::new(),
                Some(s) => Vec::with_capacity(s),
            };
            for part in line.split(',') {
                if part.starts_with('x') {
                    let empty_count: usize = part[1..].parse().unwrap_or(1);
                    for _ in 0..empty_count {
                        row.push(None);
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
                    row.push(Some(TakStack::new(variant, composition)));
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
            board.extend(row.into_iter().rev());
        }
        let size = size?;
        if board.len() != size * size {
            return None;
        }
        board.reverse();
        Some(TakBoard {
            size,
            board,
            id_counter,
            empty_spaces,
        })
    }

    /// Checks if there is a road for the given player that passes through the specified positions.
    /// Returns `Some((start, end))` if a road is found, where `start` and `end` are the coordinates of the road's endpoints.
    pub fn check_for_road(
        &self,
        positions: &[TakCoord],
        player: TakPlayer,
    ) -> Option<(TakCoord, TakCoord)> {
        let mut visited = vec![false; self.size * self.size];
        let max_pos_val = self.size as i32 - 1;
        for &pos in positions {
            let mut position_stack = vec![pos];
            let mut found_top = None;
            let mut found_bottom = None;
            let mut found_right = None;
            let mut found_left = None;
            while let Some(current_pos) = position_stack.pop() {
                match current_pos.try_get_mut(&mut visited, self.size) {
                    Some(x) if !*x => *x = true,
                    _ => continue,
                }
                let Some(Some(stack)) = current_pos.try_get(&self.board, self.size) else {
                    continue;
                };
                if stack.player() != player || stack.variant == TakPieceVariant::Wall {
                    continue;
                }

                if current_pos.x == 0 && found_left.is_none() {
                    found_left = Some(current_pos);
                } else if current_pos.x == max_pos_val && found_right.is_none() {
                    found_right = Some(current_pos);
                }

                if current_pos.y == 0 && found_bottom.is_none() {
                    found_bottom = Some(current_pos);
                } else if current_pos.y == max_pos_val && found_top.is_none() {
                    found_top = Some(current_pos);
                }

                if found_left.is_some() && found_right.is_some() {
                    return Some((found_left.unwrap(), found_right.unwrap()));
                } else if found_bottom.is_some() && found_top.is_some() {
                    return Some((found_bottom.unwrap(), found_top.unwrap()));
                }

                TakDir::ALL.iter().for_each(|&dir| {
                    position_stack.push(current_pos.offset_dir(dir));
                });
            }
        }
        None
    }

    /// Finds the shortest path from the start position to the end position for the specified player.
    /// This path follows the rules of a Tak road, meaning it can only traverse through pieces of the same player
    /// and cannot pass through walls.
    pub fn find_shortest_path(&self, start: TakCoord, end: TakCoord) -> Option<Vec<TakCoord>> {
        let Some(player) = self
            .try_get_stack(start)
            .filter(|stack| stack.variant != TakPieceVariant::Wall)
            .map(|stack| stack.player())
        else {
            return None;
        };
        if start == end {
            return Some(vec![start]);
        }
        let mut visited = vec![None; self.size * self.size];
        let mut queue = VecDeque::new();
        queue.push_back(start);
        *start.get_mut(&mut visited, self.size) = Some(start);

        let construct_path = |end: TakCoord, visited: &[Option<TakCoord>]| -> Vec<TakCoord> {
            let mut path = Vec::new();
            let mut current = end;
            while let Some(prev) = current.get(visited, self.size) {
                path.push(current);
                if prev == &start {
                    path.push(start);
                    break;
                }
                current = *prev;
            }
            path.reverse();
            path
        };

        while let Some(current_pos) = queue.pop_front() {
            if current_pos == end {
                return Some(construct_path(end, &visited));
            }
            for dir in TakDir::ALL.iter() {
                let next_pos = current_pos.offset_dir(*dir);
                if let Some(Some(next_stack)) = next_pos.try_get(&self.board, self.size) {
                    if next_stack.player() == player
                        && next_stack.variant != TakPieceVariant::Wall
                        && next_pos.get(&visited, self.size).is_none()
                    {
                        queue.push_back(next_pos);
                        *next_pos.get_mut(&mut visited, self.size) = Some(current_pos);
                    }
                }
            }
        }
        None
    }

    /// Checks if there is any empty space left on the board.
    pub fn has_empty_space(&self) -> bool {
        self.empty_spaces > 0
    }

    /// Counts the number of placed stones and capstones for the given player.
    /// Returns a tuple with the number of stones and capstones.
    pub fn count_stones(&self, player: TakPlayer) -> (usize, usize) {
        let mut stone_count = 0;
        let mut capstone_count = 0;
        for tile in &self.board {
            if let Some(stack) = tile {
                for i in 0..stack.composition.len() {
                    if stack.composition[i].player != player {
                        continue;
                    }
                    if stack.variant == TakPieceVariant::Capstone
                        && i == stack.composition.len() - 1
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

    /// Counts the number of topmost flats for each player on the board.
    /// This is used to determine the winner if a player has no stones left or there are no
    /// empty spaces left.
    pub fn count_flats(&self) -> [usize; 2] {
        let mut counts = [0, 0];
        for tile in &self.board {
            if let Some(stack) = tile {
                if stack.variant == TakPieceVariant::Flat {
                    for piece in &stack.composition {
                        counts[piece.player.index()] += 1;
                    }
                }
            }
        }
        counts
    }

    /// Returns an iterator over all pieces of the specified player on the board.
    /// Each item in the iterator is a tuple containing the position and a reference to the stack
    /// at that position.
    pub fn iter_pieces(
        &self,
        player: Option<TakPlayer>,
    ) -> impl Iterator<Item = (TakCoord, &TakStack)> {
        TakCoord::iter_board(self.size).filter_map(move |pos| {
            if let Some(stack) = pos.get(&self.board, self.size) {
                if player.is_none_or(|p| stack.player() == p) {
                    Some((pos, stack))
                } else {
                    None
                }
            } else {
                None
            }
        })
    }

    /// Returns an iterator over all empty spaces on the board.
    /// Each item in the iterator is a `TakCoord` representing an empty position.
    pub fn iter_empty_spaces<'a>(&'a self) -> impl Iterator<Item = TakCoord> + 'a {
        TakCoord::iter_board(self.size).filter(|pos| pos.try_get(&self.board, self.size).is_none())
    }

    /// Returns a reference to the stack at the specified position, if it exists.
    /// If the position is invalid or empty, it returns `None`.
    pub fn try_get_stack(&self, pos: TakCoord) -> Option<&TakStack> {
        pos.try_get(&self.board, self.size).and_then(|x| x.as_ref())
    }

    /// Validates the board state.
    /// Discovers if the board state is inconsistent or invalid.
    pub fn validate(&self) -> Result<(), String> {
        if self.size < 1 {
            return Err("Board size must be at least 1".to_string());
        }
        if self.board.len() != self.size * self.size {
            return Err(format!(
                "Board size mismatch: expected {}, got {}",
                self.size * self.size,
                self.board.len()
            ));
        }
        let mut id_set = std::collections::HashSet::new();
        let mut empty_count = 0;
        for tile in &self.board {
            if let Some(stack) = tile {
                if stack.composition.is_empty() {
                    return Err("Stack cannot be empty".to_string());
                }
                for piece in &stack.composition {
                    if !id_set.insert(piece.id) {
                        return Err(format!("Duplicate piece ID found: {}", piece.id));
                    }
                    if piece.id >= self.id_counter {
                        return Err(format!(
                            "Piece ID {} is greater than or equal to id_counter {}",
                            piece.id, self.id_counter
                        ));
                    }
                }
            } else {
                empty_count += 1;
            }
        }
        if self.id_counter != id_set.len() {
            return Err(format!(
                "ID counter mismatch: expected {}, got {}",
                id_set.len(),
                self.id_counter
            ));
        }
        if self.empty_spaces != empty_count {
            return Err(format!(
                "Empty spaces mismatch: expected {}, got {}",
                self.empty_spaces, empty_count
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_place() {
        let board = TakBoard::try_from_partial_tps("1,2,12112/2C,1S,112S/x,111C,x").unwrap();
        let occupied_cases = vec![
            TakCoord::new(0, 2),
            TakCoord::new(1, 2),
            TakCoord::new(2, 2),
            TakCoord::new(0, 1),
            TakCoord::new(1, 1),
            TakCoord::new(2, 1),
            TakCoord::new(1, 0),
        ];
        let valid_cases = vec![TakCoord::new(0, 0), TakCoord::new(2, 0)];
        let invalid_cases = vec![
            TakCoord::new(3, 0),
            TakCoord::new(0, 3),
            TakCoord::new(4, 4),
            TakCoord::new(-1, 0),
            TakCoord::new(0, -1),
        ];
        let cases = [
            (occupied_cases, Err(TakInvalidPlaceError::PositionOccupied)),
            (valid_cases, Ok(())),
            (invalid_cases, Err(TakInvalidPlaceError::InvalidPosition)),
        ];
        for (cases, expected) in cases {
            for pos in cases {
                let result = board.can_place(pos);
                assert_eq!(result, expected);
            }
        }
    }

    #[test]
    fn test_place_unchecked() {
        let mut board = TakBoard::try_from_partial_tps("x3/2C,1S,112S/x,111C,x").unwrap();
        assert_eq!(board.empty_spaces, 5);
        assert_eq!(board.id_counter, 8);
        board.do_place_unchecked(
            TakCoord::new(0, 2),
            TakPieceVariant::Capstone,
            TakPlayer::White,
        );
        assert_eq!(board.empty_spaces, 4);
        assert_eq!(board.id_counter, 9);
        assert_eq!(board.to_partial_tps(), "1C,x2/2C,1S,112S/x,111C,x");
        board.do_place_unchecked(TakCoord::new(1, 2), TakPieceVariant::Wall, TakPlayer::Black);
        board.do_place_unchecked(TakCoord::new(2, 2), TakPieceVariant::Flat, TakPlayer::White);

        assert_eq!(board.empty_spaces, 2);
        assert_eq!(board.id_counter, 11);
        assert_eq!(board.to_partial_tps(), "1C,2S,1/2C,1S,112S/x,111C,x");
    }

    #[test]
    fn test_can_undo_place() {
        let mut board = TakBoard::try_from_partial_tps("x3/2C,1S,112S/x,111C,x").unwrap();
        let pos = TakCoord::new(0, 2);
        assert!(board
            .try_place(pos, TakPieceVariant::Capstone, TakPlayer::White)
            .is_ok());

        assert_eq!(
            board.can_undo_place(pos, TakPieceVariant::Wall, TakPlayer::White),
            Err(TakInvalidUndoPlaceError::NotAllowed)
        );
        assert_eq!(
            board.can_undo_place(pos, TakPieceVariant::Capstone, TakPlayer::Black),
            Err(TakInvalidUndoPlaceError::NotAllowed)
        );
        assert_eq!(
            board.can_undo_place(TakCoord::new(1, 1), TakPieceVariant::Wall, TakPlayer::White),
            Err(TakInvalidUndoPlaceError::NotAllowed)
        );

        assert!(board
            .can_undo_place(pos, TakPieceVariant::Capstone, TakPlayer::White)
            .is_ok());
        assert!(board
            .try_undo_place(pos, TakPieceVariant::Capstone, TakPlayer::White)
            .is_ok());
        assert!(board.can_place(pos).is_ok());
        assert_eq!(board.to_partial_tps(), "x3/2C,1S,112S/x,111C,x");

        assert_eq!(
            board.can_undo_place(pos, TakPieceVariant::Capstone, TakPlayer::White),
            Err(TakInvalidUndoPlaceError::PositionEmpty)
        );
    }

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
        assert!(board.can_move(pos, TakDir::Up, 1, &[1]).is_ok());
        assert!(board.can_move(pos, TakDir::Down, 1, &[1]).is_err());
        assert!(board.can_move(pos, TakDir::Left, 1, &[1]).is_err());
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
        assert!(board.try_move(new_pos, TakDir::Up, 1, &[1]).is_ok());
        let new_pos = TakCoord::new(1, 1);
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
        assert_eq!(board.to_partial_tps(), "x2/1,2S");
    }

    #[test]
    fn test_try_move_multiple() {
        let tps = "12211C,2S,x/x3/x3";
        let mut board = TakBoard::try_from_partial_tps(tps).unwrap();
        assert!(board
            .try_move(TakCoord::new(0, 2), TakDir::Down, 3, &[3])
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
            .try_place(TakCoord::new(2, 4), TakPieceVariant::Wall, TakPlayer::White)
            .is_ok());
        assert_eq!(board.to_partial_tps(), "x,1221C,1S,21,2S/x5/x5/x5/x5");
        assert!(board
            .try_undo_place(TakCoord::new(2, 4), TakPieceVariant::Wall, TakPlayer::White)
            .is_ok());
        assert_eq!(board.to_partial_tps(), "x,1221C,x,21,2S/x5/x5/x5/x5");
    }

    #[test]
    fn test_try_undo_move() {
        let mut board = TakBoard::try_from_partial_tps("x,1221C,x,21,2S/x5/x5/x5/x5").unwrap();
        assert!(board
            .try_move(TakCoord::new(1, 4), TakDir::Right, 3, &[1, 1, 1])
            .is_ok());
        assert_eq!(board.to_partial_tps(), "x,1,2,212,21C/x5/x5/x5/x5");
        assert!(board
            .try_undo_move(TakCoord::new(1, 4), TakDir::Right, 3, &[1, 1, 1], true)
            .is_ok());
        assert_eq!(board.to_partial_tps(), "x,1221C,x,21,2S/x5/x5/x5/x5");
    }

    #[test]
    fn test_find_shortest_path() {
        let board = TakBoard::try_from_partial_tps("1,1S,1,x/221C,2,221,1/1,1,1,1/x4").unwrap();
        let start = TakCoord::new(0, 3);
        let end = TakCoord::new(2, 3);
        let path = board.find_shortest_path(start, end);
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(
            path,
            vec![
                TakCoord::new(0, 3),
                TakCoord::new(0, 2),
                TakCoord::new(0, 1),
                TakCoord::new(1, 1),
                TakCoord::new(2, 1),
                TakCoord::new(2, 2),
                TakCoord::new(2, 3)
            ]
        );
    }
}
