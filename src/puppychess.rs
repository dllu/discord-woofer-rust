use anyhow::{anyhow, Result};
use serenity::prelude::*;
use shakmaty::Position;
use std::{collections::HashMap, sync::Arc};

pub struct ChessGame;
impl TypeMapKey for ChessGame {
    type Value = Arc<RwLock<HashMap<String, Box<(shakmaty::Chess, Option<String>, Vec<String>)>>>>;
}

pub async fn chess_illegal_move(
    ctx: &serenity::prelude::Context,
    msg: &serenity::all::Message,
) -> Result<String> {
    let game_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<ChessGame>()
            .expect("Expected ChessGame")
            .clone()
    };
    let mut map = game_lock.write().await;
    let entry = map
        .entry(msg.channel_id.to_string())
        .or_insert_with(|| Box::new((shakmaty::Chess::default(), None, Vec::new())));
    let pos = &entry.0;

    let moves = pos.legal_moves();
    let move_strings: Vec<String> = moves
        .iter()
        .map(|m| {
            let san = shakmaty::san::San::from_move(pos, m);
            san.to_string()
        })
        .collect();

    let moves_string = move_strings.join(", ");
    Ok(format!(
        "Illegal move!!!!! The valid moves are {}.",
        moves_string
    ))
}

pub async fn chess(
    ctx: &Context,
    msg: &serenity::all::Message,
) -> Result<String> {
    let san_str = &msg.content[12..];
    let san: shakmaty::san::San = san_str.parse()?;

    let game_lock = {
        let data_read = ctx.data.read().await;
        data_read
            .get::<ChessGame>()
            .expect("Expected ChessGame")
            .clone()
    };
    let current_player = msg.author.id.to_string();
    let mut map = game_lock.write().await;
    let entry = map
        .entry(msg.channel_id.to_string())
        .or_insert_with(|| Box::new((shakmaty::Chess::default(), None, Vec::new())));
    if let Some(previous_player) = &(entry.1) {
        if previous_player == &current_player {
            return Ok(format!(
                "Someone else has to make a move first!!!!! The game so far is {}.",
                format_pgn(&entry.2)
            ));
        }
    }
    let pos = &entry.0;
    let mov = san.to_move(pos)?;
    let pos_next = pos.clone().play(&mov)?;
    let fen = shakmaty::fen::Epd::from_position(pos_next.clone(), shakmaty::EnPassantMode::Legal)
        .to_string();
    let f = fen
        .split(' ')
        .next()
        .ok_or_else(|| anyhow!("FEN is malformed"))?;

    let status: String;

    let mut new_moves = entry.2.clone();
    new_moves.push(san_str.to_string());

    match pos_next.outcome() {
        None => {
            **entry = (pos_next, Some(current_player), new_moves);
            status = "".to_string();
        }
        Some(outcome) => {
            let pgn = format_pgn(&new_moves);
            match outcome {
                shakmaty::Outcome::Decisive { winner: w } => match w {
                    shakmaty::Color::White => status = format!("White wins! {} 1-0", pgn),
                    shakmaty::Color::Black => status = format!("Black wins! {} 0-1", pgn),
                },
                shakmaty::Outcome::Draw => status = format!("Draw! {} 1/2-1/2", pgn),
            }
            **entry = (shakmaty::Chess::default(), None, Vec::new());
        }
    }
    return Ok(format!("{} https://chess.dllu.net/{}.png", status, f));
}

fn format_pgn(moves: &Vec<String>) -> String {
    let mut pgn = String::new();
    let mut move_count = 1;

    for (i, move_) in moves.iter().enumerate() {
        if i % 2 == 0 {
            // For every even index, print the move number
            pgn.push_str(&format!("{}. {}", move_count, move_));
            move_count += 1;
        } else {
            // For every odd index, it's a black move, so just append the move
            pgn.push_str(&format!(" {}", move_));
        }
    }

    pgn
}
