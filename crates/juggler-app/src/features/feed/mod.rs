//! D-118 (AJ-E3702) — Home's activity feed: one read model over the
//! journals AI Juggler already keeps (F1), a page at a time.
//!
//! - [`page`]: the sources ([`page::FeedSource`], one per journal) read
//!   past the page's start, merged newest first; never a load-all.
//! - [`items`]: each journal's rows as items: their type, level (the one
//!   attention table, D-117), filter chip, and the keys that coalesce and
//!   roll them up. A rollup key carries the level, so nothing broken is
//!   ever folded into a quiet rollup.
//! - [`conversations`]: other apps' conversations, from the index.
//! - Facts only: every sentence is the window's (`ui/src/lib/feed-words.ts`).
//!
//! F2 (AJ-E3704, AJ-E3705) and F3 (AJ-E3706) add their journals as more
//! sources here, recorded by the modules that own those facts.

pub mod commands;
pub mod conversations;
pub mod dto;
pub mod items;
pub mod page;

#[cfg(test)]
mod tests;
