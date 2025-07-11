#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TakCoord {
    pub x: i32,
    pub y: i32,
}

impl TakCoord {
    pub fn new(x: i32, y: i32) -> Self {
        TakCoord { x, y }
    }

    pub fn iter_board(size: usize) -> impl Iterator<Item = TakCoord> {
        (0..size).flat_map(move |y| (0..size).map(move |x| TakCoord::new(x as i32, y as i32)))
    }

    pub fn is_valid(&self, size: usize) -> bool {
        self.x >= 0 && self.y >= 0 && (self.x as usize) < size && (self.y as usize) < size
    }

    pub fn offset(&self, dx: i32, dy: i32) -> Self {
        TakCoord {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    pub fn offset_dir(&self, dir: TakDir) -> Self {
        self.offset_dir_many(dir, 1)
    }

    pub fn offset_dir_many(&self, dir: TakDir, count: i32) -> Self {
        match dir {
            TakDir::Up => self.offset(0, -count),
            TakDir::Down => self.offset(0, count),
            TakDir::Left => self.offset(-count, 0),
            TakDir::Right => self.offset(count, 0),
        }
    }

    pub fn try_get<'a, T>(&self, board: &'a [T], size: usize) -> Option<&'a T> {
        if self.is_valid(size) {
            let index = (self.y as usize) * size + (self.x as usize);
            board.get(index)
        } else {
            None
        }
    }

    pub fn get<'a, T>(&self, board: &'a [T], size: usize) -> &'a T {
        self.try_get(board, size).expect("TakCoord should be valid")
    }

    pub fn try_get_mut<'a, T>(&self, board: &'a mut [T], size: usize) -> Option<&'a mut T> {
        if self.is_valid(size) {
            let index = (self.y as usize) * size + (self.x as usize);
            board.get_mut(index)
        } else {
            None
        }
    }

    pub fn get_mut<'a, T>(&self, board: &'a mut [T], size: usize) -> &'a mut T {
        self.try_get_mut(board, size)
            .expect("TakCoord should be valid")
    }

    pub fn is_adjacent(&self, other: &TakCoord) -> Option<TakDir> {
        if self.x == other.x {
            if self.y == other.y - 1 {
                Some(TakDir::Up)
            } else if self.y == other.y + 1 {
                Some(TakDir::Down)
            } else {
                None
            }
        } else if self.y == other.y {
            if self.x == other.x - 1 {
                Some(TakDir::Left)
            } else if self.x == other.x + 1 {
                Some(TakDir::Right)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TakDir {
    Up,
    Down,
    Left,
    Right,
}

impl TakDir {
    pub const ALL: [TakDir; 4] = [TakDir::Up, TakDir::Down, TakDir::Left, TakDir::Right];
    pub fn index(&self) -> usize {
        match self {
            TakDir::Up => 0,
            TakDir::Down => 1,
            TakDir::Left => 2,
            TakDir::Right => 3,
        }
    }
    pub fn opposite(&self) -> TakDir {
        match self {
            TakDir::Up => TakDir::Down,
            TakDir::Down => TakDir::Up,
            TakDir::Left => TakDir::Right,
            TakDir::Right => TakDir::Left,
        }
    }
}
