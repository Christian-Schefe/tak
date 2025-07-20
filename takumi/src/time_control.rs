use crate::Board;

pub fn determine_time_to_use(board: &Board, time_remaining: u64, increment: u64) -> u64 {
    let ply = board.ply_index;
    let base_time = if ply < 6 { 1000 } else { 3000 };
    let estimated_remaining_moves = 20.max(ply) + (ply / 3);
    let time_bank_time = if ply < 6 {
        0
    } else {
        time_remaining / estimated_remaining_moves as u64
    };

    let time_to_use = base_time + time_bank_time + increment;
    time_to_use.min(30000) // Cap at 30 seconds
}
