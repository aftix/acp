/* This file is part of acp.
 * Copyright (c) 2021 Wyatt Campbell
 *
 * See repository LICENSE for information.
 */

use json;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};

// Information about database fields found at
// https://github.com/ankidroid/Anki-Android/wiki/Database-Structure

// The card as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
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
pub struct Field {
    font: String,
    name: String,
    ordinal: i64,
    right_to_left: bool,
    font_size: i64,
    sticky: bool,
}

// A template of the model as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
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
pub struct Request {
    ordinal: i64,
    string: String,
    list: Vec<i64>,
}

impl Request {
    // json is assumed to be an array
    pub fn new(json: &json::JsonValue) -> json::JsonResult<Self> {
        let mut req = Request {
            ordinal: 0,
            string: String::from(""),
            list: Vec::new(),
        };

        // Manually iterate through the 3 members
        let mut iter = json.members();

        let ordinal = iter.next();
        if let Some(ord) = ordinal {
            if let Some(o) = ord.as_i64() {
                req.ordinal = o;
            } else {
                return Err(json::JsonError::WrongType(String::from(
                    "Request array has improrper ordinal",
                )));
            }
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Request array too small",
            )));
        }

        let string = iter.next();
        if let Some(s) = string {
            if let Some(val) = s.as_str() {
                req.string = String::from(val);
            } else {
                return Err(json::JsonError::WrongType(String::from(
                    "Request array has improper string",
                )));
            }
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Request array too small",
            )));
        }

        let list = iter.next();
        if let Some(l) = list {
            if !l.is_array() {
                return Err(json::JsonError::WrongType(String::from(
                    "Request array list not an array",
                )));
            }

            for m in l.members() {
                if let Some(i) = m.as_i64() {
                    req.list.push(i);
                } else {
                    return Err(json::JsonError::WrongType(String::from(
                        "Request array list has non-integer",
                    )));
                }
            }
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Request array too small",
            )));
        }

        Ok(req)
    }
}

// Model of note as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    epoch: i64,
    id: i64,
    css: String,
    deck_id: Option<i64>,
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

impl Template {
    pub fn new(json: &json::JsonValue) -> json::JsonResult<Self> {
        let mut template = Template {
            answer_format: String::from(""),
            back_format: String::from(""),
            browser_format: String::from(""),
            deck_overide: None,
            name: String::from(""),
            ordinal: 0,
            question_format: String::from(""),
        };

        if !json.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "Template is not object",
            )));
        }

        // Parse template object
        if let Some(afmt) = json["afmt"].as_str() {
            template.answer_format = String::from(afmt);
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Template afmt is missing or incorrect",
            )));
        }

        if let Some(fmt) = json["bafmt"].as_str() {
            template.back_format = String::from(fmt);
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Template bafmt is missing or incorrect",
            )));
        }

        if let Some(bqfmt) = json["bqfmt"].as_str() {
            template.browser_format = String::from(bqfmt);
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Template bqfmt is missing or incorrect",
            )));
        }

        if let Some(qfmt) = json["qfmt"].as_str() {
            template.question_format = String::from(qfmt);
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Template qfmt is missing or incorrect",
            )));
        }

        if let Some(name) = json["name"].as_str() {
            template.name = String::from(name);
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Template qfmt is missing or incorrect",
            )));
        }

        if let Some(over) = json["did"].as_i64() {
            template.deck_overide = Some(over);
        }

        if let Some(ord) = json["ord"].as_i64() {
            template.ordinal = ord;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Template ord is missing or incorrect",
            )));
        }

        Ok(template)
    }
}

impl Model {
    // Parse a model from a JSON object
    pub fn new(epoch: i64, json_model: &json::JsonValue) -> json::Result<Self> {
        let mut model = Model {
            epoch,
            id: 0,
            css: String::from(""),
            deck_id: None,
            fields: Vec::new(),
            latex_post: String::from(""),
            latex_pre: String::from(""),
            modification_time: 0,
            name: String::from(""),
            sort_field: 0,
            templates: Vec::new(),
            model_type: 0,
            usn: 0,
            req: None,
        };

        // The model is an object at root level
        if !json_model.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "Model is not an object",
            )));
        }

        // Get the easy fields from the JSONValue
        // tags, vers ignored

        if let Some(css) = json_model["css"].as_str() {
            model.css = String::from(css);
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "CSS field missing or incorrect",
            )));
        }

        // Can be missing
        if let Some(deck_id) = json_model["did"].as_i64() {
            model.deck_id = Some(deck_id);
        } else if let Some(deck_id) = json_model["did"].as_str() {
            let deck_id = deck_id.parse::<i64>();
            if let Err(_) = deck_id {
                return Err(json::JsonError::WrongType(String::from(
                    "Deck ID field missing or incorrect",
                )));
            }
            model.deck_id = Some(deck_id.unwrap());
        }

        if let Some(id) = json_model["id"].as_i64() {
            model.id = id;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "ID field missing or incorrect",
            )));
        }

        if let Some(pre) = json_model["latexPre"].as_str() {
            model.latex_pre = String::from(pre);
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "latexPre field missing or incorrect",
            )));
        }

        if let Some(post) = json_model["latexPost"].as_str() {
            model.latex_post = String::from(post);
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "latexPost field missing or incorrect",
            )));
        }

        if let Some(modification) = json_model["mod"].as_i64() {
            model.modification_time = modification;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "mod field missing or incorrect",
            )));
        }

        if let Some(name) = json_model["name"].as_str() {
            model.name = String::from(name);
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "name field missing or incorrect",
            )));
        }

        if let Some(sort) = json_model["sortf"].as_i64() {
            model.sort_field = sort;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "sortf field missing or incorrect",
            )));
        }

        if let Some(t) = json_model["sortf"].as_i64() {
            model.model_type = t;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "type field missing or incorrect",
            )));
        }

        if let Some(usn) = json_model["usn"].as_i64() {
            model.usn = usn;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "usn field missing or incorrect",
            )));
        }

        // Parse the req field, if it's there
        let ref req = json_model["req"];
        if req.is_array() {
            let mut req_vec: Vec<Vec<Request>> = Vec::new();
            for member in req.members() {
                let mut req_vec_vec = Vec::new();
                if !member.is_array() {
                    return Err(json::JsonError::WrongType(String::from(
                        "req inner member not array",
                    )));
                }
                for submember in req.members() {
                    req_vec_vec.push(Request::new(submember)?);
                }

                req_vec.push(req_vec_vec);
            }

            model.req = Some(req_vec);
        }

        // Parse the template field
        let ref templates = json_model["tmpls"];
        if !templates.is_array() {
            return Err(json::JsonError::WrongType(String::from(
                "tmpls is not array",
            )));
        }

        for member in templates.members() {
            model.templates.push(Template::new(member)?);
        }

        Ok(model)
    }

    // Parse all models from a string
    pub fn parse(data: &str) -> json::JsonResult<Vec<Self>> {
        let mut models = Vec::new();

        let parsed = json::parse(data)?;

        if !parsed.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "Models are not in an object",
            )));
        }

        for (epoch, model) in parsed.entries() {
            let epoch = epoch.parse::<i64>();
            if let Err(_) = epoch {
                return Err(json::JsonError::WrongType(String::from(
                    "Model does not have proper id",
                )));
            }
            let epoch = epoch.unwrap();

            models.push(Model::new(epoch, model)?);
        }

        Ok(models)
    }
}

// The note as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    id: i64,             // Note id
    guid: String,        // Globally unique ID
    model_id: i64,       // Model ID
    mod_time: i64,       // Modification time
    usn: i64,            // update sequence number
    tags: Vec<String>,   // tags on the note
    fields: Vec<String>, // Field values
    sort_field: String,  // Sort field,
    sum: i64,            // Field checksum
}

// A deck as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deck {
    epoch: i64,
    name: String,
    extended_review_limit: i64,
    usn: i64,
    collapsed: bool,
    browser_collapsed: bool,
    dynamic: i64,
    extended_new_limit: i64,
    config_id: i64,
    id: i64,
    modification_time: i64,
    description: String,
    new_today: (i64, i64),
    learned_today: (i64, i64),
    reviewed_today: (i64, i64),
}

impl Deck {
    // Parse a single deck JSON
    pub fn new(epoch: i64, json: &json::JsonValue) -> json::JsonResult<Deck> {
        let mut deck = Deck {
            epoch,
            name: String::new(),
            extended_review_limit: 10,
            usn: 0,
            collapsed: false,
            browser_collapsed: false,
            dynamic: 0,
            extended_new_limit: 10,
            config_id: 0,
            id: 0,
            modification_time: 0,
            description: String::new(),
            new_today: (0, 0),
            learned_today: (0, 0),
            reviewed_today: (0, 0),
        };

        if !json.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "Deck is not an object",
            )));
        }

        // Parse the deck!
        if let Some(name) = json["name"].as_str() {
            deck.name = String::from(name);
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck name field missing or incorect",
            )));
        }

        // This value is OK to be missing, defaults to 10
        if let Some(extended_rev) = json["extended_rev"].as_i64() {
            deck.extended_review_limit = extended_rev;
        }

        if let Some(usn) = json["usn"].as_i64() {
            deck.usn = usn;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck usn field missing or incorect",
            )));
        }

        if let Some(collapsed) = json["collapsed"].as_bool() {
            deck.collapsed = collapsed;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck collapsed field missing or incorect",
            )));
        }

        if let Some(browser_collapsed) = json["browserCollapsed"].as_bool() {
            deck.browser_collapsed = browser_collapsed;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck browserCollapsed field missing or incorect",
            )));
        }

        if let Some(dynamic) = json["dyn"].as_i64() {
            deck.dynamic = dynamic;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck dyn field missing or incorect",
            )));
        }

        // Is ok if absent, defaults to 10
        if let Some(extended_new) = json["extendNew"].as_i64() {
            deck.extended_new_limit = extended_new;
        }

        if let Some(conf) = json["conf"].as_i64() {
            deck.config_id = conf;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck conf field missing or incorect",
            )));
        }

        if let Some(id) = json["id"].as_i64() {
            deck.id = id;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck id field missing or incorect",
            )));
        }

        if let Some(modification) = json["mod"].as_i64() {
            deck.modification_time = modification;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck mod field missing or incorect",
            )));
        }

        if let Some(desc) = json["desc"].as_str() {
            deck.description = String::from(desc);
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck desc field missing or incorect",
            )));
        }

        // Now, parse the tuples
        let ref new_today = json["newToday"];
        if !new_today.is_array() {
            return Err(json::JsonError::WrongType(String::from(
                "Deck newToday field missing or incorect",
            )));
        }
        let new_today: Vec<&json::JsonValue> = new_today.members().collect();
        if new_today.len() != 2 {
            return Err(json::JsonError::WrongType(String::from(
                "Deck newToday array wrong length",
            )));
        }
        if let Some(i) = new_today[0].as_i64() {
            deck.new_today.0 = i;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck newToday array element 0 not integer",
            )));
        }
        if let Some(i) = new_today[1].as_i64() {
            deck.new_today.1 = i;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck newToday array element 1 not integer",
            )));
        }

        let ref learned_today = json["lrnToday"];
        if !learned_today.is_array() {
            return Err(json::JsonError::WrongType(String::from(
                "Deck lrnToday field missing or incorect",
            )));
        }
        let learned_today: Vec<&json::JsonValue> = learned_today.members().collect();
        if learned_today.len() != 2 {
            return Err(json::JsonError::WrongType(String::from(
                "Deck lrnToday array wrong length",
            )));
        }
        if let Some(i) = learned_today[0].as_i64() {
            deck.learned_today.0 = i;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck lrnToday array element 0 not integer",
            )));
        }
        if let Some(i) = learned_today[1].as_i64() {
            deck.learned_today.1 = i;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck lrnToday array element 1 not integer",
            )));
        }

        let ref review_today = json["lrnToday"];
        if !review_today.is_array() {
            return Err(json::JsonError::WrongType(String::from(
                "Deck revToday field missing or incorect",
            )));
        }
        let review_today: Vec<&json::JsonValue> = review_today.members().collect();
        if review_today.len() != 2 {
            return Err(json::JsonError::WrongType(String::from(
                "Deck revToday array wrong length",
            )));
        }
        if let Some(i) = review_today[0].as_i64() {
            deck.reviewed_today.0 = i;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck revToday array element 0 not integer",
            )));
        }
        if let Some(i) = learned_today[1].as_i64() {
            deck.reviewed_today.1 = i;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck revToday array element 1 not integer",
            )));
        }

        Ok(deck)
    }

    // Parse the totality of the JSON into all the decks
    pub fn parse(data: &str) -> json::JsonResult<Vec<Deck>> {
        let mut decks = Vec::new();

        let parsed = json::parse(data)?;

        if !parsed.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "Decks are not an object at top level",
            )));
        }

        // Every deck will be a key in the object with the key being the epoch id
        for (deck_epoch, deck_json) in parsed.entries() {
            let deck_epoch = deck_epoch.parse::<i64>();
            if let Err(_) = deck_epoch {
                return Err(json::JsonError::WrongType(String::from(
                    "Deck does not have proper id",
                )));
            }
            let deck_epoch = deck_epoch.unwrap();

            decks.push(Deck::new(deck_epoch, deck_json)?);
        }

        Ok(decks)
    }
}

// Configuration of lasped cards in the Deck configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LapsedConfig {
    delays: Vec<f64>,
    leech_action: i64,
    leech_fails: i64,
    min_interval: i64,
    mult: f64,
}

impl LapsedConfig {
    pub fn new(json: &json::JsonValue) -> json::JsonResult<Self> {
        let mut lapsed = LapsedConfig {
            delays: Vec::new(),
            leech_action: 0,
            leech_fails: 0,
            min_interval: 0,
            mult: 0.0,
        };

        if !json.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "lapse is not an object",
            )));
        }

        // Parse the lapse configuration
        if let Some(leech_action) = json["leechAction"].as_i64() {
            lapsed.leech_action = leech_action;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "leech leechAction field missing or incorrect",
            )));
        }

        if let Some(leech_fails) = json["leechFails"].as_i64() {
            lapsed.leech_fails = leech_fails;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "leech leechFails field missing or incorrect",
            )));
        }

        if let Some(min) = json["minInt"].as_i64() {
            lapsed.min_interval = min;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "leech minInt field missing or incorrect",
            )));
        }

        if let Some(mult) = json["mult"].as_f64() {
            lapsed.mult = mult;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "leech mult field missing or incorrect",
            )));
        }

        let ref delays = json["delays"];
        if !delays.is_array() {
            return Err(json::JsonError::WrongType(String::from(
                "leech delays field missing or incorrect",
            )));
        }

        for delay in delays.members() {
            if !delay.is_number() {
                return Err(json::JsonError::WrongType(String::from(
                    "leech delays array contains non number",
                )));
            }
            lapsed.delays.push(delay.as_f64().unwrap());
        }

        Ok(lapsed)
    }
}

// Configuration of new cards in the Deck configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewConfig {
    bury: bool,
    delays: Vec<f64>,
    initial_factor: i64,
    intervals: Vec<i64>,
    order: i64,
    per_day: i64,
    separate: i64,
}

impl NewConfig {
    pub fn new(json: &json::JsonValue) -> json::JsonResult<Self> {
        let mut new = NewConfig {
            bury: false,
            delays: Vec::new(),
            initial_factor: 0,
            intervals: Vec::new(),
            order: 0,
            per_day: 0,
            separate: 0,
        };

        if !json.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "new is not object",
            )));
        }

        // Parse the object
        if let Some(bury) = json["bury"].as_bool() {
            new.bury = bury;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "new bury field missing or incorrect",
            )));
        }

        if let Some(initial) = json["initialFactor"].as_i64() {
            new.initial_factor = initial;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "new initialFactor field missing or incorrect",
            )));
        }

        if let Some(order) = json["order"].as_i64() {
            new.order = order;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "new order field missing or incorrect",
            )));
        }

        if let Some(perday) = json["perDay"].as_i64() {
            new.per_day = perday;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "new perDay field missing or incorrect",
            )));
        }

        // Parse the lists
        let ref delays = json["delays"];
        if !delays.is_array() {
            return Err(json::JsonError::WrongType(String::from(
                "new delays field missing or incorrect",
            )));
        }

        for delay in delays.members() {
            if let Some(i) = delay.as_f64() {
                new.delays.push(i);
            } else {
                return Err(json::JsonError::WrongType(String::from(
                    "new delay array contains non number",
                )));
            }
        }

        let ref ints = json["ints"];
        if !ints.is_array() {
            return Err(json::JsonError::WrongType(String::from(
                "new ints field missing or incorrect",
            )));
        }

        for int in ints.members() {
            if let Some(i) = int.as_i64() {
                new.intervals.push(i);
            } else {
                return Err(json::JsonError::WrongType(String::from(
                    "new ints array contains non number",
                )));
            }
        }

        Ok(new)
    }
}

// Configuration of review cards in the Deck configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewConfig {
    bury: bool,
    ease4: f64,
    fuzz: Option<f64>,
    interval_factor: f64,
    max_interval: f64,
    per_day: i64,
}

impl ReviewConfig {
    pub fn new(json: &json::JsonValue) -> json::JsonResult<Self> {
        let mut rev = ReviewConfig {
            bury: false,
            ease4: 0.0,
            fuzz: None,
            interval_factor: 0.0,
            max_interval: 0.0,
            per_day: 0,
        };

        if !json.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "rev is not an object",
            )));
        }

        // Parse the object
        if let Some(bury) = json["bury"].as_bool() {
            rev.bury = bury;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "rev bury field missing or incorrect",
            )));
        }

        if let Some(ease) = json["ease4"].as_f64() {
            rev.ease4 = ease;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "rev ease4 field missing or incorrect",
            )));
        }

        // Can be missing
        if let Some(fuzz) = json["fuzz"].as_f64() {
            rev.fuzz = Some(fuzz);
        }

        if let Some(ifactor) = json["ivlFct"].as_f64() {
            rev.interval_factor = ifactor;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "rev ivlFct field missing or incorrect",
            )));
        }

        if let Some(max) = json["maxIvl"].as_f64() {
            rev.max_interval = max;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "rev maxIvl field missing or incorrect",
            )));
        }

        if let Some(perday) = json["perDay"].as_i64() {
            rev.per_day = perday;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "rev perDay field missing or incorrect",
            )));
        }

        Ok(rev)
    }
}

// The deck configuration as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeckConfig {
    id: i64,
    autoplay: bool,
    dynamic: bool,
    lapse: Option<LapsedConfig>,
    max_taken: i64,
    modification_time: i64,
    name: String,
    new: Option<NewConfig>,
    replay_audio: bool,
    review: Option<ReviewConfig>,
    timer: i64,
    usn: i64,
}

impl DeckConfig {
    pub fn new(id: i64, json: &json::JsonValue) -> json::JsonResult<Self> {
        let mut conf = DeckConfig {
            id,
            autoplay: false,
            dynamic: false,
            lapse: None,
            max_taken: 0,
            modification_time: 0,
            name: String::new(),
            new: None,
            replay_audio: false,
            review: None,
            timer: 0,
            usn: 0,
        };

        if !json.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "Deck config value is not an object",
            )));
        }

        // Parse the easy stuff
        if let Some(autoplay) = json["autoplay"].as_bool() {
            conf.autoplay = autoplay;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck configuration autoplay field missing or incorrect",
            )));
        }

        if let Some(dynamic) = json["dyn"].as_bool() {
            conf.dynamic = dynamic;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck configuration dyn field missing or incorrect",
            )));
        }

        if let Some(max) = json["maxTaken"].as_i64() {
            conf.max_taken = max;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck configuration maxTaken field missing or incorrect",
            )));
        }

        if let Some(modification) = json["mod"].as_i64() {
            conf.modification_time = modification;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck configuration mod field missing or incorrect",
            )));
        }

        if let Some(name) = json["name"].as_str() {
            conf.name = String::from(name);
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck configuration name field missing or incorrect",
            )));
        }

        if let Some(replayq) = json["replayq"].as_bool() {
            conf.replay_audio = replayq;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck configuration replayq field missing or incorrect",
            )));
        }

        if let Some(timer) = json["timer"].as_i64() {
            conf.timer = timer;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck configuration timer field missing or incorrect",
            )));
        }

        if let Some(usn) = json["usn"].as_i64() {
            conf.usn = usn;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck configuration usn field missing or incorrect",
            )));
        }

        // Parse sub objects
        conf.lapse = Some(LapsedConfig::new(&json["lapse"])?);
        conf.new = Some(NewConfig::new(&json["new"])?);
        conf.review = Some(ReviewConfig::new(&json["rev"])?);

        Ok(conf)
    }

    // Parse the totality of the JSON into all the deck configs
    pub fn parse(data: &str) -> json::JsonResult<Vec<Self>> {
        let mut confs = Vec::new();

        let parsed = json::parse(data)?;

        if !parsed.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "Deck Options is not a top level object",
            )));
        }

        for (conf_id, conf_json) in parsed.entries() {
            let conf_id = conf_id.parse::<i64>();
            if let Err(_) = conf_id {
                return Err(json::JsonError::WrongType(String::from(
                    "Deck config key is not an id",
                )));
            }

            confs.push(DeckConfig::new(conf_id.unwrap(), conf_json)?);
        }

        Ok(confs)
    }
}

// Synced configuration options as represented in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    current_deck: i64,
    active_decks: Vec<i64>,
    new_spread: i64,
    collapse_time: i64,
    time_limit: i64,
    estimated_times: bool,
    due_counts: bool,
    current_model: i64,
    next_pos: i64,
    sort_type: Option<String>,
    sort_backwards: bool,
    add_to_current: bool,
    day_learn_first: bool,
    new_bury: Option<bool>,
    last_unburied: Option<i64>,
    active_cols: Vec<String>,
}

impl SyncConfig {
    pub fn new(data: &str) -> json::JsonResult<Self> {
        let mut conf = SyncConfig {
            current_deck: 0,
            active_decks: Vec::new(),
            new_spread: 0,
            collapse_time: 0,
            time_limit: 0,
            estimated_times: false,
            due_counts: false,
            current_model: 0,
            next_pos: 0,
            sort_type: None,
            sort_backwards: false,
            add_to_current: false,
            day_learn_first: false,
            new_bury: None,
            last_unburied: None,
            active_cols: Vec::new(),
        };

        let json = json::parse(data)?;

        if !json.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig is not an object",
            )));
        }

        // Get the options from the JSON
        if let Some(cur) = json["curDeck"].as_i64() {
            conf.current_deck = cur;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig curDeck field is missing or incorrect",
            )));
        }

        if let Some(spread) = json["newSpread"].as_i64() {
            conf.new_spread = spread;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig newSpread field is missing or incorrect",
            )));
        }

        if let Some(collapse) = json["collapseTime"].as_i64() {
            conf.collapse_time = collapse;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig collapseTime field is missing or incorrect",
            )));
        }

        if let Some(time) = json["timeLim"].as_i64() {
            conf.time_limit = time;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig timeLim field is missing or incorrect",
            )));
        }

        if let Some(est) = json["estTimes"].as_bool() {
            conf.estimated_times = est;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig estTimes field is missing or incorrect",
            )));
        }

        if let Some(due) = json["dueCounts"].as_bool() {
            conf.due_counts = due;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig dueCounts field is missing or incorrect",
            )));
        }

        if let Some(cur) = json["curModel"].as_i64() {
            conf.current_model = cur;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig curModel field is missing or incorrect",
            )));
        }

        if let Some(pos) = json["nextPos"].as_i64() {
            conf.next_pos = pos;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig nextPos field is missing or incorrect",
            )));
        }

        // Can be missing
        if let Some(sort) = json["sortType"].as_str() {
            conf.sort_type = Some(String::from(sort));
        }

        if let Some(sort) = json["sortBackwards"].as_bool() {
            conf.sort_backwards = sort;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig sortBackwards field is missing or incorrect",
            )));
        }

        if let Some(add) = json["addToCur"].as_bool() {
            conf.add_to_current = add;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig addToCur field is missing or incorrect",
            )));
        }

        if let Some(day) = json["dayLearnFirst"].as_bool() {
            conf.day_learn_first = day;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig dayLearnFirst field is missing or incorrect",
            )));
        }

        // Can be missing
        if let Some(newbury) = json["newBury"].as_bool() {
            conf.new_bury = Some(newbury);
        }

        // Can be missing
        if let Some(last) = json["lastUnburied"].as_i64() {
            conf.last_unburied = Some(last);
        }

        // Parse the lists
        let ref active = json["activeDecks"];
        if !active.is_array() {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig activeDecks field is missing or incorrect",
            )));
        }

        for j in active.members() {
            if let Some(i) = j.as_i64() {
                conf.active_decks.push(i);
            } else {
                return Err(json::JsonError::WrongType(String::from(
                    "SyncConfig activeDecks contains non number",
                )));
            }
        }

        // This one can be missing
        let ref active = json["activeCols"];
        if active.is_array() {
            for j in active.members() {
                if let Some(col) = j.as_str() {
                    conf.active_cols.push(String::from(col));
                } else {
                    return Err(json::JsonError::WrongType(String::from(
                        "SyncConfig activeCols contains non string",
                    )));
                }
            }
        } else if active == &json::JsonValue::Null {
            conf.active_cols.push(String::from("noteFld"));
            conf.active_cols.push(String::from("template"));
            conf.active_cols.push(String::from("cardDue"));
            conf.active_cols.push(String::from("deck"));
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "SyncConfig activeCols is incorrect",
            )));
        }

        Ok(conf)
    }
}

// The review log as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewLog {
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

// The graves as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grave {
    usn: i64,
    oid: i64,
    grave_type: i64,
}

// The collection information as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    id: i64,                       // arbritrary
    crt: i64,                      // creation date in seconds
    modification_time: i64,        // Last modified time in milliseconds
    schema_time: i64,              // schema modification time
    version: i64,                  // version
    usn: i64,                      // update sequence number
    last_sync: i64,                // last sync time
    config: SyncConfig,            // JSON, synced config options
    models: Vec<Model>,            // JSON, Note types
    decks: Vec<Deck>,              // JSON, the decks
    deck_configs: Vec<DeckConfig>, // JSON, group options for decks
    tags: String,                  // tag cache
    notes: Vec<Note>,              // Notes
    cards: Vec<Card>,              // Cards
    revlog: Vec<ReviewLog>,        // Review log
    graves: Vec<Grave>,            // Deleted things
}

impl Collection {
    // Build a connection from a .anki2 sqlite database
    pub fn new(path: &Path) -> Result<Self> {
        // Connection to the database
        let conn = Connection::open(path)?;

        // Start by loading the single row of the col table into the collection
        let mut stmt = conn.prepare(
            "SELECT id, crt, mod, scm, ver, usn, ls, conf, models, decks, dconf, tags FROM col",
        )?;
        let mut col_iter = stmt.query_map([], |row| {
            let config_txt: String = row.get(7)?;
            let model_txt: String = row.get(8)?;
            let deck_txt: String = row.get(9)?;
            let dconf_txt: String = row.get(10)?;
            Ok(Collection {
                id: row.get(0)?,
                crt: row.get(1)?,
                modification_time: row.get(2)?,
                schema_time: row.get(3)?,
                version: row.get(4)?,
                usn: row.get(5)?,
                last_sync: row.get(6)?,
                config: SyncConfig::new(&config_txt).unwrap(),
                models: Model::parse(&model_txt).unwrap(),
                decks: Deck::parse(&deck_txt).unwrap(),
                deck_configs: DeckConfig::parse(&dconf_txt).unwrap(),
                tags: row.get(11)?,
                notes: Vec::new(),
                cards: Vec::new(),
                revlog: Vec::new(),
                graves: Vec::new(),
            })
        })?;

        let mut collection = col_iter.next().unwrap()?.clone();

        // Load the cards
        let mut stmt = conn.prepare("SELECT id, nid, did, ord, mod, usn, type, queue, due, ivl, factor, reps, lapses, left, odue, odid, flags FROM cards")?;
        let card_iter = stmt.query_map([], |row| {
            Ok(Card {
                id: row.get(0)?,
                note_id: row.get(1)?,
                deck_id: row.get(2)?,
                ordinal: row.get(3)?,
                modification_time: row.get(4)?,
                usn: row.get(5)?,
                card_type: row.get(6)?,
                queue: row.get(7)?,
                due: row.get(8)?,
                interval: row.get(9)?,
                factor: row.get(10)?,
                reps: row.get(11)?,
                left: row.get(12)?,
                original_due: row.get(13)?,
                original_deck_id: row.get(14)?,
                flags: row.get(15)?,
            })
        })?;

        collection.cards = card_iter.map(|result| result.unwrap()).collect();

        // Load the notes
        let mut stmt =
            conn.prepare("SELECT id, guid, mid, mod, usn, tags, flds, sfld, csum FROM notes")?;
        let note_iter = stmt.query_map([], |row| {
            let tags: String = row.get(5)?;
            let fields: String = row.get(6)?;
            Ok(Note {
                id: row.get(0)?,
                guid: row.get(1)?,
                model_id: row.get(2)?,
                mod_time: row.get(3)?,
                usn: row.get(4)?,
                tags: tags.split(" ").map(String::from).collect(),
                fields: fields.split("\0x1f").map(String::from).collect(),
                sort_field: row.get(7)?,
                sum: row.get(8)?,
            })
        })?;

        collection.notes = note_iter.map(|result| result.unwrap()).collect();

        // Load the review log
        let mut stmt = conn
            .prepare("SELECT id, cid, usn, ease, ivl, lastIvl, factor, time, type FROM revlog")?;
        let rev_iter = stmt.query_map([], |row| {
            Ok(ReviewLog {
                id: row.get(0)?,
                card_id: row.get(1)?,
                usn: row.get(2)?,
                ease: row.get(3)?,
                interval: row.get(4)?,
                last_interval: row.get(5)?,
                factor: row.get(6)?,
                time: row.get(7)?,
                card_type: row.get(8)?,
            })
        })?;

        collection.revlog = rev_iter.map(|result| result.unwrap()).collect();

        // Load the graves
        let mut stmt = conn.prepare("SELECT usn, oid, type FROM graves")?;
        let grave_iter = stmt.query_map([], |row| {
            Ok(Grave {
                usn: row.get(0)?,
                oid: row.get(1)?,
                grave_type: row.get(2)?,
            })
        })?;

        collection.graves = grave_iter.map(|result| result.unwrap()).collect();

        Ok(collection)
    }
}
