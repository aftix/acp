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
struct CardDB {
    id: i64,        // Card id
    nid: i64,       // Note id
    did: i64,       // Deck id
    ord: i64, //ordinal, determines which of the card templates or cloze deletions it belongs to
    mod_time: i64, // Modification time in seconds since epoch
    usn: i64, // Update sequence number, used for syncs
    card_type: i64, // 0 = new, 1 = learning, 2 = review, 3 = relearning
    queue: i64, // Where in the queue is the card
    due: i64, // When the card is due, usage depends on card type
    ivl: i64, // Interval, - is seconds, + is days
    factor: i64, // The ease factor of the card is parts per thousand (permille)
    reps: i64, // The number of reviews
    left: i64, // the number of reps left until graduation
    odue: i64, // Original due
    odid: i64, // Used for filtered decks
    flags: i64, // The card flags
    data: String, // Unused
}

// The collection information as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CollectionDB {
    id: i64,        // arbritrary
    crt: i64,       // creation date in seconds
    mod_time: i64,  // Last modified time in milliseconds
    scm: i64,       // schema modification time
    ver: i64,       // version
    dty: i64,       // Unused, 0
    usn: i64,       // update sequence number
    ls: i64,        // last sync time
    conf: String,   // JSON, synced config options
    models: String, // JSON, Note types
    decks: String,  // JSON, the decks
    dconf: String,  // JSON, group options for decks
    tags: String,   // tag cache
}

// The review log as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RevlogDB {
    id: i64,       // epoch-milliseconds of when the review was done
    cid: i64,      // Card id
    usn: i64,      // update sequence number
    ease: i64,     // Which button was pressed on the review
    ivl: i64,      // Card interval
    last_ivl: i64, // Previous card interval
    factor: i64,   // factor
    time: i64,     // How long the review took in milliseconds
    cardtype: i64, // As in card_db
}
