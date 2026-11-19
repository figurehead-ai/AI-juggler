//! Bot (D-136, AJ-E3804..E3808, E3815, E3819): one continuous
//! conversation about what is going on in AI Juggler, in a docked panel
//! and a full page. It looks things up in AI Juggler's own records by
//! code, links every row to its source, says "I don't know" rather than
//! answering a nearby question, and acts only through a confirm card.

pub mod check;
pub mod commands;
pub mod dto;
pub mod guard;
pub mod history;
pub mod lookup;
pub mod question;
pub mod world;
pub mod written;

#[cfg(test)]
mod tests;
