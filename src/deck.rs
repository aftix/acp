/* This file is part of acp.
 * Copyright (c) 2021 Wyatt Campbell
 *
 * See repository LICENSE for information.
 */

use json;
use serde::{Deserialize, Serialize};

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
            if let json::JsonValue::String(val) = s {
                req.string = val.clone();
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
        if let json::JsonValue::String(ref afmt) = json["afmt"] {
            template.answer_format = afmt.clone();
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Template afmt is missing or incorrect",
            )));
        }

        if let json::JsonValue::String(ref bafmt) = json["bafmt"] {
            template.back_format = bafmt.clone();
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Template bafmt is missing or incorrect",
            )));
        }

        if let json::JsonValue::String(ref bqfmt) = json["bqfmt"] {
            template.browser_format = bqfmt.clone();
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Template bqfmt is missing or incorrect",
            )));
        }

        if let json::JsonValue::String(ref qfmt) = json["qfmt"] {
            template.question_format = qfmt.clone();
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Template qfmt is missing or incorrect",
            )));
        }

        if let json::JsonValue::String(ref name) = json["name"] {
            template.name = name.clone();
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
    // Parse a model from the JSON string inside the database
    pub fn new(input: &str) -> json::Result<Self> {
        let mut model = Model {
            epoch: 0,
            id: 0,
            css: String::from(""),
            deck_id: 0,
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
        let parsed = json::parse(input)?;

        // The model is an object at root level
        if !parsed.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "Model is not an object",
            )));
        }

        // Should have an object with key of epoch time
        if parsed.len() != 1 {
            return Err(json::JsonError::WrongType(String::from(
                "Model is wrong length",
            )));
        }

        // Get the key, value pair for the first (only) entry in the root object
        let (model_key, json_model) = parsed.entries().next().unwrap();

        // Epoch is the key for the first entry
        let epoch = model_key.parse::<i64>();
        if let Err(_) = epoch {
            return Err(json::JsonError::WrongType(String::from(
                "Model not named with epoch",
            )));
        }
        model.epoch = epoch.unwrap();

        // Make sure that we're working with an object
        if !json_model.is_object() {
            return Err(json::JsonError::WrongType(String::from(
                "Nested model is not an object",
            )));
        }

        // Get the easy fields from the JSONValue
        // tags, vers ignored

        if let json::JsonValue::String(ref css) = json_model["css"] {
            model.css = css.clone();
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "CSS field missing or incorrect",
            )));
        }

        if let Some(deck_id) = json_model["did"].as_i64() {
            model.deck_id = deck_id;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "Deck ID field missing or incorrect",
            )));
        }

        if let Some(id) = json_model["id"].as_i64() {
            model.id = id;
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "ID field missing or incorrect",
            )));
        }

        if let json::JsonValue::String(ref pre) = json_model["latexPre"] {
            model.latex_pre = pre.clone();
        } else {
            return Err(json::JsonError::WrongType(String::from(
                "latexPre field missing or incorrect",
            )));
        }

        if let json::JsonValue::String(ref post) = json_model["latexPost"] {
            model.latex_post = post.clone();
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

        if let json::JsonValue::String(ref name) = json_model["name"] {
            model.name = name.clone();
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
}

// The note as stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
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
pub struct Collection {
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
pub struct Revlog {
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
