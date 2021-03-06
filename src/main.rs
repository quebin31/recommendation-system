// Copyright (c) 2020 White Leaf
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

pub mod parser;
pub mod utils;

use anyhow::Error;
use books::BooksController;
use clap::{App, Arg};
use config::Config;
use controller::{eid, Controller, Entity, ToTable};
use engine::{
    chunked_matrix::{ChunkedMatrix, DeviationMatrix, SimilarityMatrix},
    distances::items::Method as ItemMethod,
    Engine,
};
use movie_lens::MovieLensController;
use movie_lens_small::MovieLensSmallController;
use parser::{Database, Statement};
use rustyline::Editor;
use shelves::ShelvesController;
use simple_movie::SimpleMovieController;
use simplelog::{
    CombinedLogger, Config as LogConfig, ConfigBuilder as LogConfigBuilder, LevelFilter,
    TermLogger, TerminalMode, WriteLogger,
};
use std::{
    fmt::{Debug, Display},
    fs::File,
    hash::Hash,
    time::Instant,
};
use utils::build_prototype;

macro_rules! prompt {
    ($ed:ident) => {{
        prompt!($ed, "")
    }};

    ($ed:ident, $db:expr) => {{
        use rustyline::error::ReadlineError;

        let msg = if $db.is_empty() {
            format!("{}", $crate::PROMPT)
        } else {
            format!("({}) {}", $db, $crate::PROMPT)
        };

        match $ed.readline(&msg) {
            Ok(line) => {
                $ed.add_history_entry(line.as_str());
                Ok(line)
            }

            Err(ReadlineError::Interrupted) => {
                continue;
            }

            Err(ReadlineError::Eof) => {
                if $db.is_empty() {
                    println!("Exiting...Good bye!");
                } else {
                    println!("Disconnecting from {}", $db);
                }

                break;
            }

            Err(e) => Err(e),
        }
    }};
}

fn chunked_matrix_prompt<'a, M, C, U, I>(
    controller: &C,
    mut matrix: M,
    name: &str,
    rl: &mut Editor<()>,
) -> Result<(), Error>
where
    C: Controller<User = U, Item = I>,
    M: ChunkedMatrix<'a, C, I>,
    U: Entity,
    I: Entity,
    eid!(U): Hash + Eq + Display + Clone + Debug + Default,
    eid!(I): Hash + Eq + Display + Clone + Debug,
{
    let mut curr_i = 0;
    let mut curr_j = 0;

    let now = Instant::now();
    match matrix.calculate_chunk(curr_i, curr_j) {
        Ok(chunk) => chunk,
        Err(e) => {
            log::error!("{}", e);
            return Ok(());
        }
    }

    println!("Operation took {:.4} seconds", now.elapsed().as_secs_f64());

    loop {
        let formatted = format!("{}:matrix({}, {})", name, curr_i, curr_j);
        let opt: String = prompt!(rl, formatted)?;

        match opt.trim() {
            "e" | "exit" => {
                println!("Exiting the matrix");
                break;
            }

            "v" | "version" => {
                println!("version: {}", VERSION);
            }

            empty if empty.is_empty() => {}

            comment if comment.starts_with('#') => {}

            line => match parser::parse_line(line) {
                Some(stmt) => match stmt {
                    Statement::MatrixGet(searchby_a, searchby_b) => {
                        let item_id_a = match controller.items_by(&searchby_a) {
                            Ok(items) => items[0].get_id(),
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let item_id_b = match controller.items_by(&searchby_b) {
                            Ok(items) => items[0].get_id(),
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let val = matrix.get_value(&item_id_a, &item_id_b);

                        if let Some(val) = val {
                            println!("Value for ({}, {}) is {}", item_id_a, item_id_b, val);
                        } else {
                            log::error!("No value found for ({}, {})", item_id_a, item_id_b);
                        }
                    }

                    Statement::MatrixMoveTo(i, j) => {
                        curr_i = i;
                        curr_j = j;

                        let now = Instant::now();
                        match matrix.calculate_chunk(curr_i, curr_j) {
                            Ok(chunk) => chunk,
                            Err(e) => {
                                log::error!("{}", e);
                                return Ok(());
                            }
                        }
                        println!("Operation took {:.4} seconds", now.elapsed().as_secs_f64());
                    }

                    _ => {
                        log::error!("Invalid statement in this context.");
                        log::error!("Exit the matrix first!");
                    }
                },

                None => log::error!("Invalid syntax!"),
            },
        }
    }

    Ok(())
}

fn database_connected_prompt<C, U, I, R>(
    config: &Config,
    controller: C,
    name: &str,
    rl: &mut Editor<()>,
) -> Result<(), Error>
where
    C: Controller<User = U, Item = I, Rating = R>,
    U: Entity,
    I: Entity,
    R: Entity,
    eid!(U): Hash + Eq + Display + Clone + Debug + Default,
    eid!(I): Hash + Eq + Display + Clone + Debug,
    eid!(R): Display,
{
    let mut engine = Engine::with_controller(&controller, config);

    loop {
        let opt: String = prompt!(rl, name)?;

        match opt.trim() {
            "d" | "disconnect" => {
                println!("Disconnecting from database {}", name);
                break;
            }

            "v" | "version" => {
                println!("version: {}", VERSION);
            }

            empty if empty.is_empty() => {}

            comment if comment.starts_with('#') => {}

            line => match parser::parse_line(line) {
                Some(stmt) => match stmt {
                    Statement::Connect(_) => {
                        log::error!("Invalid statement in this context.");
                        log::error!("Disconnect from current database first!");
                    }

                    Statement::MatrixGet(_, _) | Statement::MatrixMoveTo(_, _) => {
                        log::error!("Invalid statement in this context.");
                        log::error!("Enter the matrix first!");
                    }

                    Statement::QueryUser(searchby) => match controller.users_by(&searchby) {
                        Ok(users) => {
                            for user in users {
                                println!("{}", user.to_table());
                            }
                        }
                        Err(e) => log::error!("{}", e),
                    },

                    Statement::QueryItem(searchby) => match controller.items_by(&searchby) {
                        Ok(items) => {
                            for item in items {
                                println!("{}", item.to_table());
                            }
                        }
                        Err(e) => log::error!("{}", e),
                    },

                    Statement::QueryRatings(searchby) => match controller.users_by(&searchby) {
                        Ok(users) => {
                            for user in users {
                                if let Ok(ratings) = controller.user_ratings(&user) {
                                    if !ratings.is_empty() {
                                        println!("{}", ratings.to_table());
                                    } else {
                                        log::error!(
                                            "No ratings found for user with id({})",
                                            user.get_id()
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => log::error!("{}", e),
                    },

                    Statement::InsertUser => {
                        let fields = controller.fields_for_users();
                        let prototype = build_prototype(rl, fields);

                        let prototype = match prototype {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("Error creating prototype");
                                log::error!("Reason: {}", e);
                                continue;
                            }
                        };

                        match controller.insert_user(prototype) {
                            Ok(user) => {
                                println!("Successfully inserted! Yay!");
                                println!("{}", user.to_table());
                            }

                            Err(e) => {
                                log::error!("Failed to insert user!");
                                log::error!("Reason: {}", e);
                            }
                        }
                    }

                    Statement::InsertItem => {
                        let fields = controller.fields_for_items();
                        let prototype = build_prototype(rl, fields);

                        let prototype = match prototype {
                            Ok(p) => p,
                            Err(e) => {
                                log::error!("Error creating prototype");
                                log::error!("Reason: {}", e);
                                continue;
                            }
                        };

                        match controller.insert_item(prototype) {
                            Ok(item) => {
                                println!("Successfully inserted! Yay!");
                                println!("{}", item.to_table());
                            }

                            Err(e) => {
                                log::error!("Failed to insert item!");
                                log::error!("Reason: {}", e);
                            }
                        }
                    }

                    Statement::InsertRating(searchby_user, searchby_item, score) => {
                        let (lower_limit, upper_limit) = controller.score_range();

                        if score < lower_limit || score > upper_limit {
                            log::error!("Given score is not between database bounds");
                            continue;
                        }

                        let user = match controller
                            .users_by(&searchby_user)
                            .map(|mut users| users.drain(..1).next().unwrap())
                        {
                            Ok(user) => user,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let item = match controller
                            .items_by(&searchby_item)
                            .map(|mut items| items.drain(..1).next().unwrap())
                        {
                            Ok(item) => item,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let user_id = user.get_id();
                        let item_id = item.get_id();

                        match controller.insert_rating(&user_id, &item_id, score) {
                            Ok(rating) => {
                                println!("Successfully inserted! Yay!");
                                println!("{}", rating.to_table());

                                match controller.users_means(&[user]) {
                                    Ok(means) => {
                                        if let Some(mean) = means.get(&user_id) {
                                            engine.maybe_update_mean_for(&user_id, *mean);
                                        } else {
                                            log::error!(
                                                "Expected a calculated mean value for user({})",
                                                user_id
                                            );
                                            log::error!("Got nothing instead");
                                            panic!(
                                                "No calculated mean for user({}), broken invariant on insertion of rating",
                                                user_id
                                            );
                                        }
                                    }

                                    Err(e) => {
                                        log::error!("Couldn't get mean for user({})", user_id);
                                        log::error!("Got error: {}", e);
                                        log::error!("Maybe check the database");
                                        log::error!("Deleting cached mean on adj_cosine");
                                        engine.maybe_delete_mean_for(&user_id);
                                    }
                                }
                            }

                            Err(e) => {
                                log::error!("Failed to insert rating!");
                                log::error!("Reason: {}", e);
                            }
                        }
                    }

                    Statement::UpdateRating(searchby_user, searchby_item, score) => {
                        let (lower_limit, upper_limit) = controller.score_range();
                        if score < lower_limit || score > upper_limit {
                            log::error!("Given score is not between database bounds");
                            continue;
                        }

                        let user = match controller
                            .users_by(&searchby_user)
                            .map(|mut users| users.drain(..1).next().unwrap())
                        {
                            Ok(user) => user,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let item = match controller
                            .items_by(&searchby_item)
                            .map(|mut items| items.drain(..1).next().unwrap())
                        {
                            Ok(item) => item,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let user_id = user.get_id();
                        let item_id = item.get_id();

                        match controller.update_rating(&user_id, &item_id, score) {
                            Ok(rating) => {
                                println!("Successfully updated! Yay!");
                                println!("{}", rating.to_table());

                                match controller.users_means(&[user]) {
                                    Ok(means) => {
                                        if let Some(mean) = means.get(&user_id) {
                                            engine.maybe_update_mean_for(&user_id, *mean);
                                        } else {
                                            log::error!(
                                                "Expected a calculated mean value for user({})",
                                                user_id
                                            );
                                            log::error!("Got nothing instead");
                                            panic!(
                                                "No calculated mean for user({}), broken invariant on update of rating",
                                                user_id
                                            );
                                        }
                                    }

                                    Err(e) => {
                                        log::error!("Couldn't get mean for user({})", user_id);
                                        log::error!("Got error: {}", e);
                                        log::error!("Maybe check the database");
                                        log::error!("Deleting cached mean on adj_cosine");
                                        engine.maybe_delete_mean_for(&user_id);
                                    }
                                }
                            }

                            Err(e) => {
                                log::error!("Failed to update rating!");
                                log::error!("Reason: {}", e);
                            }
                        }
                    }

                    Statement::RemoveRating(searchby_user, searchby_item) => {
                        let user = match controller
                            .users_by(&searchby_user)
                            .map(|mut users| users.drain(..1).next().unwrap())
                        {
                            Ok(user) => user,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let item = match controller
                            .items_by(&searchby_item)
                            .map(|mut items| items.drain(..1).next().unwrap())
                        {
                            Ok(item) => item,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let user_id = user.get_id();
                        let item_id = item.get_id();

                        match controller.remove_rating(&user_id, &item_id) {
                            Ok(rating) => {
                                println!("Successfully removed! Yay?");
                                println!("{}", rating.to_table());

                                match controller.users_means(&[user]) {
                                    Ok(means) => {
                                        if let Some(mean) = means.get(&user_id) {
                                            engine.maybe_update_mean_for(&user_id, *mean);
                                        } else {
                                            engine.maybe_delete_mean_for(&user_id);
                                        }
                                    }

                                    Err(e) => {
                                        log::error!("Couldn't get mean for user({})", user_id);
                                        log::error!("Got error: {}", e);
                                        log::error!("Maybe check the database");
                                        log::error!("Deleting cached mean on adj_cosine");
                                        engine.maybe_delete_mean_for(&user_id);
                                    }
                                }
                            }

                            Err(e) => {
                                log::error!("Failed to remove rating!");
                                log::error!("Reason: {}", e);
                            }
                        }
                    }

                    Statement::ItemDistance(searchby_a, searchby_b, method) => {
                        let item_a = match controller
                            .items_by(&searchby_a)
                            .map(|mut items| items.drain(..1).next().unwrap())
                        {
                            Ok(item) => item,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let item_b = match controller
                            .items_by(&searchby_b)
                            .map(|mut items| items.drain(..1).next().unwrap())
                        {
                            Ok(item) => item,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let now = Instant::now();
                        let dist = engine.item_distance(item_a, item_b, method);
                        match dist {
                            Ok(dist) => println!("Distance is {}", dist),
                            Err(e) => {
                                log::error!("Distance couldn't be calculated");
                                log::error!("Reason: {}", e);
                            }
                        }

                        println!("Operation took {:.4} seconds", now.elapsed().as_secs_f64());
                    }

                    Statement::UserDistance(searchby_a, searchby_b, method) => {
                        let user_a = match controller
                            .users_by(&searchby_a)
                            .map(|mut users| users.drain(..1).next().unwrap())
                        {
                            Ok(user) => user,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let user_b = match controller
                            .users_by(&searchby_b)
                            .map(|mut users| users.drain(..1).next().unwrap())
                        {
                            Ok(user) => user,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let now = Instant::now();
                        let dist = engine.user_distance(user_a, user_b, method);
                        match dist {
                            Ok(dist) => println!("Distance is {}", dist),
                            Err(e) => {
                                log::error!("Distance couldn't be calculated");
                                log::error!("Reason: {}", e);
                            }
                        }

                        println!("Operation took {:.4} seconds", now.elapsed().as_secs_f64());
                    }

                    Statement::UserKnn(k, searchby, method, chunks_opt) => {
                        let user = match controller
                            .users_by(&searchby)
                            .map(|mut users| users.drain(..1).next().unwrap())
                        {
                            Ok(user) => user,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let now = Instant::now();
                        let knn = engine.user_knn(k, user, method, chunks_opt);

                        let elapsed = now.elapsed().as_secs_f64();

                        match knn {
                            Ok(knn) => {
                                for (nn_id, dist) in knn {
                                    println!("Distance with user with id({}) is {}", nn_id, dist);
                                }
                            }

                            Err(e) => {
                                log::error!("Failed to find the {} nearest neighbors", k);
                                log::error!("Reason: {}", e);
                            }
                        }

                        println!("Operation took {:.4} seconds", elapsed);
                    }

                    Statement::UserBasedPredict(
                        k,
                        searchby_user,
                        searchby_item,
                        method,
                        chunks_opt,
                    ) => {
                        let user = match controller
                            .users_by(&searchby_user)
                            .map(|mut users| users.drain(..1).next().unwrap())
                        {
                            Ok(user) => user,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let item = match controller
                            .items_by(&searchby_item)
                            .map(|mut items| items.drain(..1).next().unwrap())
                        {
                            Ok(item) => item,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let item_id = item.get_id();

                        let now = Instant::now();
                        let prediction =
                            engine.user_based_predict(k, user, item, method, chunks_opt);

                        match prediction {
                            Ok(predicted) => println!(
                                "Predicted score for item with id({}) is {}",
                                item_id, predicted
                            ),

                            Err(e) => {
                                log::error!("Failed to predict the score");
                                log::error!("Reason: {}", e);
                            }
                        }

                        println!("Operation took {:.4} seconds", now.elapsed().as_secs_f64());
                    }

                    Statement::ItemBasedPredict(
                        searchby_user,
                        searchby_item,
                        method,
                        chunk_size,
                    ) => {
                        let user = match controller
                            .users_by(&searchby_user)
                            .map(|mut users| users.drain(..1).next().unwrap())
                        {
                            Ok(user) => user,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let item = match controller
                            .items_by(&searchby_item)
                            .map(|mut items| items.drain(..1).next().unwrap())
                        {
                            Ok(item) => item,
                            Err(e) => {
                                log::error!("{}", e);
                                continue;
                            }
                        };

                        let item_id = item.get_id();

                        let now = Instant::now();
                        let prediction = engine.item_based_predict(user, item, method, chunk_size);

                        match prediction {
                            Ok(predicted) => println!(
                                "Predicted score for item with id({}) is {}",
                                item_id, predicted
                            ),

                            Err(e) => {
                                log::error!("Failed to predict the score");
                                log::error!("Reason: {}", e);
                            }
                        }

                        println!("Operation took {:.4} seconds", now.elapsed().as_secs_f64());
                    }

                    Statement::EnterMatrix(m, n, method) => match method {
                        ItemMethod::AdjCosine => {
                            let adj_cosine = engine.clone_rc_adj_cosine();
                            let matrix = SimilarityMatrix::with_cache(
                                &controller,
                                &config,
                                adj_cosine,
                                m,
                                n,
                            );
                            chunked_matrix_prompt(&controller, matrix, name, rl)?;
                        }

                        ItemMethod::SlopeOne => {
                            let matrix = DeviationMatrix::new(&controller, &config, m, n);
                            chunked_matrix_prompt(&controller, matrix, name, rl)?;
                        }
                    },
                },

                None => log::error!("Invalid syntax!"),
            },
        }
    }

    Ok(())
}

const NAME: &str = env!("CARGO_PKG_NAME");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
const ABOUT: &str = env!("CARGO_PKG_DESCRIPTION");
const PROMPT: &str = ">> ";

fn to_level_filter(level: usize) -> LevelFilter {
    match level {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 => LevelFilter::Info,
        _ => LevelFilter::Debug,
    }
}

fn main() -> Result<(), Error> {
    let matches = App::new(NAME)
        .version(VERSION)
        .author(AUTHORS)
        .about(ABOUT)
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("PATH")
                .default_value("config.toml")
                .help("Set custom config file path"),
        )
        .get_matches();

    let config_path = matches.value_of("config").unwrap();
    let config = Config::load(config_path)?;
    let term_level = to_level_filter(config.system.term_verbosity_level);
    let file_log_path = config
        .system
        .log_output
        .clone()
        .unwrap_or_else(|| "rsys.log".to_string());
    let file_log = File::create(&file_log_path)?;
    let file_level = to_level_filter(config.system.file_verbosity_level);

    CombinedLogger::init(vec![
        TermLogger::new(
            term_level,
            LogConfigBuilder::new()
                .set_time_level(LevelFilter::Off)
                .build(),
            TerminalMode::Mixed,
        ),
        WriteLogger::new(file_level, LogConfig::default(), file_log),
    ])?;

    println!("Welcome to recommendation-system {}", VERSION);
    let mut rl = rustyline::Editor::<()>::new();

    loop {
        let opt: String = prompt!(rl)?;

        match opt.trim() {
            "q" | "quit" => {
                println!("Bye!");
                break;
            }

            "v" | "version" => {
                println!("version: {}", VERSION);
            }

            empty if empty.is_empty() => {}

            comment if comment.starts_with('#') => {}

            line => match parser::parse_line(line) {
                Some(stmt) => {
                    if let Statement::Connect(db) = stmt {
                        let name = db.to_string();

                        match db {
                            Database::Books => database_connected_prompt(
                                &config,
                                BooksController::from_config(&config, &name)?,
                                &name,
                                &mut rl,
                            )?,

                            Database::Shelves => database_connected_prompt(
                                &config,
                                ShelvesController::from_config(&config, &name)?,
                                &name,
                                &mut rl,
                            )?,

                            Database::SimpleMovie => database_connected_prompt(
                                &config,
                                SimpleMovieController::from_config(&config, &name)?,
                                &name,
                                &mut rl,
                            )?,

                            Database::MovieLens => database_connected_prompt(
                                &config,
                                MovieLensController::from_config(&config, &name)?,
                                &name,
                                &mut rl,
                            )?,

                            Database::MovieLensSmall => database_connected_prompt(
                                &config,
                                MovieLensSmallController::from_config(&config, &name)?,
                                &name,
                                &mut rl,
                            )?,
                        }
                    } else {
                        log::error!("Invalid statement in this context.");
                        log::error!("Connect to a database first!");
                    }
                }

                None => log::error!("Invalid syntax!"),
            },
        }
    }

    Ok(())
}
