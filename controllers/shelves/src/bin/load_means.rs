// Copyright (c) 2020 White Leaf
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use anyhow::Error;
use config::Config;
use controller::Controller;
use diesel::pg::PgConnection;
use diesel::{insert_into, prelude::*};
use shelves::establish_connection;
use shelves::models::users::NewMean;
use shelves::schema::means;
use shelves::ShelvesController;
use std::collections::HashMap;
use std::time::Instant;

fn insert_means(conn: &PgConnection, new_means: &[NewMean]) -> Result<(), Error> {
    insert_into(means::table).values(new_means).execute(conn)?;

    Ok(())
}

fn compute_mean(ratings: &HashMap<i32, f64>) -> Option<f64> {
    if ratings.is_empty() {
        return None;
    }

    let mut mean = 0.0;
    for rating in ratings.values() {
        mean += rating;
    }
    Some(mean / ratings.len() as f64)
}

fn main() -> Result<(), Error> {
    let vars: HashMap<String, String> = dotenv::vars().collect();
    let mut config = Config::default();

    let db = config.databases.get_mut("shelves").unwrap();
    db.psql_url = vars["DATABASE_URL"].clone();
    db.mongo_url = vars["MONGO_URL"].clone();
    db.mongo_db = vars["MONGO_DB"].clone();

    let conn = establish_connection(&db.psql_url)?;
    let controller = ShelvesController::from_config(&config, "shelves")?;

    let users_iterator = controller.users_by_chunks(10000);
    for user_chunk in users_iterator {
        println!("Inserting new chunk");
        let now = Instant::now();
        let mut mean_chunk = Vec::new();
        let maped_ratings = controller.users_ratings(&user_chunk)?;
        for (user_id, ratings) in maped_ratings {
            let mean = compute_mean(&ratings);
            if let Some(mean) = mean {
                mean_chunk.push(NewMean {
                    user_id,
                    val: mean,
                    score_number: ratings.len() as i32,
                });
            }
        }

        insert_means(&conn, &mean_chunk)?;
        println!("Elapsed per iteration: {}", now.elapsed().as_secs_f64());
    }

    Ok(())
}
