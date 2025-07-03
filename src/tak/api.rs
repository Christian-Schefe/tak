use crate::tak::{TakPlayer, TakAction, TakCoord, TakGameState, TakInvalidAction, TakTower};

pub trait TakGameAPI {
    type Settings;
    fn try_do_action(&mut self, action: TakAction) -> Result<TakGameState, TakInvalidAction>;
    fn new_game(size: usize, settings: Self::Settings) -> Self;
    fn current_player(&self) -> TakPlayer;
    fn size(&self) -> usize;
    fn get_actions(&self) -> &Vec<TakAction>;
    fn try_get_tower(&self, pos: TakCoord) -> Option<&TakTower>;
}
