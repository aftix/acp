/* This file is part of acp.
 * Copyright (c) 2021 Wyatt Campbell
 *
 * See repository LICENSE for information.
 */

use serde::{Deserialize, Serialize};

// Information about database fields found at
// https://github.com/ankidroid/Anki-Android/wiki/Database-Structure

// The card as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Card {
    id: i64,
    note_id: i64,
    deck_id: i64,
    ordinal: i64, // determines which of the card templates or cloze deletions it belongs to
    modification_time: i64, // seconds since epoch
    usn: i64,     // Update sequence number, used for syncs
    card_type: i64, // 0 = new, 1 = learning, 2 = review, 3 = relearning
    queue: i64,   // Where in the queue is the card
    due: i64,     // When the card is due, usage depends on card type
    interval: i64, // Interval, - is seconds, + is days
    factor: i64,  // The ease factor of the card is parts per thousand (permille)
    reps: i64,    // The number of reviews
    left: i64,    // the number of reps left until graduation
    original_due: i64, // Original due
    original_deck_id: i64, // Used for filtered decks
    flags: i64,   // The card flags
}

// A field of the model as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Field {
    font: String,
    name: String,
    ordinal: i64,
    right_to_left: bool,
    font_size: i64,
    sticky: bool,
}

// A template of the model as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Template {
    answer_format: String,
    back_format: String,
    browser_format: String,
    deck_overide: Option<i64>,
    name: String,
    ordinal: i64,
    question_format: String,
}

// A request of the model as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Request {
    ordinal: i64,
    string: String,
    list: Vec<i64>,
}

// Model of note as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Model {
    epoch: i64,
    id: i64,
    css: String,
    deck_id: i64,
    fields: Vec<Field>,
    latex_post: String,
    latex_pre: String,
    modification_time: i64,
    name: String,
    sort_field: i64,
    templates: Vec<Template>,
    model_type: i64,
    usn: i64,
    req: Option<Vec<Vec<Request>>>,
}

// The note as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Note {
    id: i64,                  // Note id
    guid: i64,                // Globally unique ID
    model_id: i64,            // Model ID
    mod_time: i64,            // Modification time
    usn: i64,                 // update sequence number
    tags: Vec<String>,        // tags on the note
    fields: Vec<String>,      // Field values
    sort_field: i64,          // Sort field,
    sum: i64,                 // Field checksum
    cards: Option<Vec<Card>>, // cards using this note
}

// The collection information as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Collection {
    id: i64,                // arbritrary
    crt: i64,               // creation date in seconds
    modification_time: i64, // Last modified time in milliseconds
    schema_time: i64,       // schema modification time
    version: i64,           // version
    usn: i64,               // update sequence number
    last_sync: i64,         // last sync time
    config: String,         // JSON, synced config options
    models: String,         // JSON, Note types
    decks: String,          // JSON, the decks
    deck_configs: String,   // JSON, group options for decks
    tags: String,           // tag cache
}

// The review log as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Revlog {
    id: i64,            // epoch-milliseconds of when the review was done
    card_id: i64,       // Card id
    usn: i64,           // update sequence number
    ease: i64,          // Which button was pressed on the review
    interval: i64,      // Card interval
    last_interval: i64, // Previous card interval
    factor: i64,        // factor
    time: i64,          // How long the review took in milliseconds
    card_type: i64,     // As in card_db
}
