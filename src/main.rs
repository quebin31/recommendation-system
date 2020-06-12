pub mod parser;

use anyhow::Error;
use books::BooksController;
use controller::{Controller, Entity, ToTable};
use engine::Engine;
use movie_lens::MovieLensController;
use movie_lens_small::MovieLensSmallController;
use parser::{Database, Statement};
use simple_movie::SimpleMovieController;
use std::{fmt::Display, hash::Hash, time::Instant};

macro_rules! prompt {
    ($ed:ident) => {{
        prompt!($ed, "")
    }};

    ($ed:ident, $db:expr) => {{
        use rustyline::error::ReadlineError;

        let msg = if $db.is_empty() {
            format!("{}", PROMPT)
        } else {
            format!("({}) {}", $db, PROMPT)
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

fn similarity_matrix_shell() {
    todo!()
}

fn database_connected_prompt<C, User, UserId, Item, ItemId>(
    controller: C,
    name: &str,
) -> Result<(), Error>
where
    C: Controller<User, UserId, Item, ItemId>,
    User: Entity<Id = UserId> + ToTable,
    Item: Entity<Id = ItemId> + ToTable,
    UserId: Hash + Eq + Display,
    ItemId: Hash + Eq + Display,
{
    let engine = Engine::with_controller(&controller);
    let mut rl = rustyline::Editor::<()>::new();

    loop {
        let opt: String = prompt!(rl, name)?;

        match opt.trim() {
            "q" | "quit" => {
                println!("Bye!");
                break;
            }

            "d" | "disconnect" => {
                println!("Disconnecting from database {}", name);
                break;
            }

            "v" | "version" => {
                println!("version: {}", VERSION);
            }

            line => match parser::parse_line(line) {
                Some(stmt) => match stmt {
                    Statement::Connect(_) => {
                        println!("Invalid statement in this context.");
                        println!("Disconnect from current database first!");
                    }

                    Statement::QueryUser(searchby) => match controller.users_by(&searchby) {
                        Ok(users) => {
                            for user in users {
                                println!("{}", user.to_table());
                            }
                        }
                        Err(e) => println!("{}", e),
                    },

                    Statement::QueryItem(searchby) => match controller.items_by(&searchby) {
                        Ok(items) => {
                            for item in items {
                                println!("{}", item.to_table());
                            }
                        }
                        Err(e) => println!("{}", e),
                    },

                    Statement::QueryRatings(searchby) => match controller.users_by(&searchby) {
                        Ok(users) => {
                            for user in users {
                                if let Ok(ratings) = controller.ratings_by(&user) {
                                    if !ratings.is_empty() {
                                        println!("{}", ratings.to_table());
                                    } else {
                                        println!(
                                            "No ratings found for user with id({})",
                                            user.get_id()
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => println!("{}", e),
                    },

                    Statement::UserDistance(searchby_a, searchby_b, method) => {
                        let users_a = match controller.users_by(&searchby_a) {
                            Ok(users) => users,
                            Err(e) => {
                                println!("{}", e);
                                continue;
                            }
                        };

                        let users_b = match controller.users_by(&searchby_b) {
                            Ok(users) => users,
                            Err(e) => {
                                println!("{}", e);
                                continue;
                            }
                        };

                        let now = Instant::now();
                        let dist = engine.user_distance(&users_a[0], &users_b[0], method);
                        match dist {
                            Some(dist) => println!("Distance is {}", dist),
                            None => println!("Distance couldn't be calculated or gave NaN/∞/-∞"),
                        }

                        println!("Operation took {:.4} seconds", now.elapsed().as_secs_f64());
                    }

                    Statement::UserKnn(k, searchby, method, chunks_opt) => {
                        let users = controller.users_by(&searchby);
                        let now = Instant::now();
                        let knn = match users {
                            Ok(users) => engine.user_knn(k, &users[0], method, chunks_opt),
                            Err(e) => {
                                println!("{}", e);
                                continue;
                            }
                        };

                        let elapsed = now.elapsed().as_secs_f64();

                        match knn {
                            Some(knn) => {
                                if knn.is_empty() {
                                    println!("Couldn't found the {} nearest neighbours", k);
                                    println!("Try using a different metric");
                                    continue;
                                }

                                for (nn_id, dist) in knn {
                                    println!("Distance with user with id({}) is {}", nn_id, dist);
                                }
                            }

                            None => println!("Failed to calculate the {} nearest neighbors", k),
                        }

                        println!("Operation took {:.4} seconds", elapsed);
                    }

                    Statement::KnnPredict(k, searchby_user, searchby_item, method, chunks_opt) => {
                        let users = match controller.users_by(&searchby_user) {
                            Ok(users) => users,
                            Err(e) => {
                                println!("{}", e);
                                continue;
                            }
                        };

                        let items = match controller.items_by(&searchby_item) {
                            Ok(items) => items,
                            Err(e) => {
                                println!("{}", e);
                                continue;
                            }
                        };

                        let now = Instant::now();
                        let prediction =
                            engine.knn_predict(k, &users[0], &items[0], method, chunks_opt);
                        match prediction {
                            Some(predicted) => println!(
                                "Predicted score for item with id({}) is {}",
                                items[0].get_id(),
                                predicted
                            ),
                            None => {
                                println!("Failed to predict the score");
                                println!(
                                    "Try increasing 'k' or using a different metric for inner knn"
                                );
                            }
                        }

                        println!("Operation took {:.4} seconds", now.elapsed().as_secs_f64());
                    }
                },

                None => println!("Invalid syntax!"),
            },
        }
    }

    Ok(())
}

const VERSION: &str = env!("CARGO_PKG_VERSION");
const PROMPT: &str = ">> ";

fn main() -> Result<(), Error> {
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

            line => match parser::parse_line(line) {
                Some(stmt) => {
                    if let Statement::Connect(db) = stmt {
                        match db {
                            Database::Books => {
                                database_connected_prompt(BooksController::new()?, "books")?
                            }
                            Database::SimpleMovie => database_connected_prompt(
                                SimpleMovieController::new()?,
                                "simple-movie",
                            )?,
                            Database::MovieLens => database_connected_prompt(
                                MovieLensController::new()?,
                                "movie-lens",
                            )?,
                            Database::MovieLensSmall => database_connected_prompt(
                                MovieLensSmallController::new()?,
                                "movie-lens-small",
                            )?,
                        }
                    } else {
                        println!("Invalid statement in this context.");
                        println!("Connect to a database first!");
                    }
                }
                None => println!("Invalid syntax!"),
            },
        }
    }

    Ok(())
}
