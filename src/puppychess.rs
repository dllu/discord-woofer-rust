use crate::utils;
use anyhow::{anyhow, Result};
use serenity::prelude::*;
use shakmaty::Position;
use std::{collections::HashMap, sync::Arc};

pub struct ChessGame;
impl TypeMapKey for ChessGame {
    type Value = Arc<RwLock<HashMap<String, Box<ChessState>>>>;
}

pub struct ChessState {
    pos: shakmaty::Chess,
    user_id: Option<String>,
    user_name: Option<String>,
    moves: Vec<String>,
}

impl ChessState {
    fn new() -> Self {
        ChessState {
            pos: shakmaty::Chess::default(),
            user_id: None,
            user_name: None,
            moves: Vec::new(),
        }
    }
}

pub struct ChessOutput {
    status: String,
    url: String,
    pgn: String,
}

pub async fn reply(
    ctx: &serenity::prelude::Context,
    msg: &serenity::all::Message,
    chess: ChessOutput,
) -> Result<()> {
    let embed = serenity::builder::CreateEmbed::new()
        .description(chess.status)
        .image(chess.url)
        .field("move history", chess.pgn, false)
        .timestamp(serenity::model::Timestamp::now());
    let builder = serenity::builder::CreateMessage::new().embed(embed);

    if let Err(why) = msg.channel_id.send_message(&ctx.http, builder).await {
        println!("Error sending message: {why:?}");
    }
    Ok(())
}

pub async fn chess_illegal_move(
    ctx: &serenity::prelude::Context,
    msg: &serenity::all::Message,
) -> Result<ChessOutput> {
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
        .or_insert_with(|| Box::new(ChessState::new()));
    let pos = &entry.pos;

    let moves = pos.legal_moves();
    let move_strings: Vec<String> = moves
        .iter()
        .map(|m| {
            let san = shakmaty::san::San::from_move(pos, m);
            san.to_string()
        })
        .collect();

    let moves_string = move_strings.join(", ");

    Ok(ChessOutput {
        status: format!("Illegal move!!!!! The valid moves are {moves_string}."),
        url: fen_url(entry.pos.clone())?,
        pgn: format_pgn(&entry.moves),
    })
}

pub async fn chess(ctx: &Context, msg: &serenity::all::Message) -> Result<ChessOutput> {
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
        .or_insert_with(|| Box::new(ChessState::new()));
    if let Some(previous_player) = &(entry.user_id) {
        if previous_player == &current_player {
            let username = entry.user_name.as_ref().unwrap();
            return Ok(ChessOutput {
                status : format!("Someone else has to make a move first!!!!! The last player to make a move is {username}."),
                url : fen_url(entry.pos.clone())?,
                pgn : format_pgn(&entry.moves),
                });
        }
    }
    let pos = &entry.pos;
    let mov = san.to_move(pos)?;
    let pos_next = pos.clone().play(&mov)?;

    let status: String;

    let mut new_moves = entry.moves.clone();
    new_moves.push(san_str.to_string());
    let pgn = format_pgn(&new_moves);

    match pos_next.outcome() {
        None => {
            **entry = ChessState {
                pos: pos_next.clone(),
                user_id: Some(current_player),
                user_name: Some(utils::author_name_from_msg(msg)),
                moves: new_moves,
            };
            status = "".to_string();
        }
        Some(outcome) => {
            match outcome {
                shakmaty::Outcome::Decisive { winner: w } => match w {
                    shakmaty::Color::White => status = format!("White wins! {} 1-0", pgn),
                    shakmaty::Color::Black => status = format!("Black wins! {} 0-1", pgn),
                },
                shakmaty::Outcome::Draw => status = format!("Draw! {} 1/2-1/2", pgn),
            }
            **entry = ChessState::new();
        }
    }

    Ok(ChessOutput {
        status,
        url: fen_url(pos_next)?,
        pgn,
    })
}

fn fen_url(pos: shakmaty::Chess) -> Result<String> {
    let fen = shakmaty::fen::Epd::from_position(pos, shakmaty::EnPassantMode::Legal).to_string();
    let f = fen
        .split(' ')
        .next()
        .ok_or_else(|| anyhow!("FEN is malformed"))?;
    Ok(format!("https://chess.dllu.net/{f}.png"))
}

fn format_pgn(moves: &[String]) -> String {
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
