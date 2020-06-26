// Copyright (c) 2020 White Leaf
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use anyhow::Error;
use controller::{Controller, Entity};
use diesel::pg::PgConnection;
use diesel::{insert_into, prelude::*};
use simple_movie::establish_connection;
use simple_movie::models::users::NewMean;
use simple_movie::schema::means;
use simple_movie::SimpleMovieController;
use std::collections::HashMap;

fn create_mean(
    conn: &PgConnection,
    user_id: i32,
    mean: f64,
    score_number: usize,
) -> Result<(), Error> {
    let new_mean = NewMean {
        user_id,
        val: mean,
        score_number: score_number as i32,
    };

    insert_into(means::table).values(&new_mean).execute(conn)?;

    Ok(())
}

fn compute_mean(ratings: &HashMap<i32, f64>) -> f64 {
    if ratings.is_empty() {
        return 0.0;
    }

    let mut mean = 0.0;
    for rating in ratings.values() {
        mean += rating;
    }
    mean / ratings.len() as f64
}

fn main() -> Result<(), Error> {
    let vars: HashMap<String, String> = dotenv::vars().collect();

    let psql_url = &vars["DATABASE_URL"];
    let mongo_url = &vars["MONGO_URL"];
    let mongo_db = &vars["MONGO_DB"];
    let conn = establish_connection(psql_url)?;

    let controller = SimpleMovieController::with_url(psql_url, mongo_url, mongo_db)?;

    let users_iterator = controller.users_by_chunks(10000);
    for user_chunk in users_iterator {
        let maped_ratings = controller.maped_ratings_by(&user_chunk)?;
        for user in user_chunk {
            let user_id = user.get_id();
            if maped_ratings.contains_key(&user_id) {
                let mean = compute_mean(&maped_ratings[&user_id]);
                create_mean(&conn, user_id, mean, maped_ratings[&user_id].len())?;
            } else {
                create_mean(&conn, user_id, 0.0, 0)?;
            }
        }
    }

    Ok(())
}
