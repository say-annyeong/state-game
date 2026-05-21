use crate::state_game_compiler::token::Token;

pub struct TokenTree {
    token: Vec<Token>,
    current_position: usize,
    position: usize,
    start_position: usize,
}