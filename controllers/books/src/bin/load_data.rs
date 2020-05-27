use anyhow::Error;
use books::establish_connection;
use books::models::{books::NewBook, ratings::NewRating, users::NewUser};
use books::schema::{books as books_sc, ratings, users};
use books::BooksController;
use controller::Controller;
use diesel::pg::PgConnection;
use diesel::{insert_into, prelude::*};
use indicatif::ProgressIterator;

fn insert_users(conn: &PgConnection) -> Result<(), Error> {
    let mut csv = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b';')
        .from_path("data/users.csv")?;

    let mut users = Vec::new();
    println!("Collecting records for users...");
    let records: Vec<_> = csv.records().collect();

    for record in records.iter().progress() {
        if let Ok(record) = record {
            let id: i32 = record[0].parse()?;
            let location = &record[1];
            let age: Option<i16> = if &record[2] == "\\N" {
                None
            } else {
                Some(record[2].parse()?)
            };

            users.push(NewUser { id, location, age });
        }
    }

    println!("Pushing users by chunks");
    for chunk in users.chunks(10_000).progress() {
        insert_into(users::table).values(chunk).execute(conn)?;
    }

    Ok(())
}

fn insert_books(conn: &PgConnection) -> Result<(), Error> {
    let mut csv = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b';')
        .from_path("data/books.csv")?;

    let mut books = Vec::new();
    println!("Collecting records for books...");
    let records: Vec<_> = csv.records().collect();

    for record in records.iter().progress() {
        if let Ok(record) = record {
            let id = &record[0];
            let title = &record[1];
            let author = &record[2];
            let year: i16 = record[3].parse()?;
            let publisher = &record[4];

            books.push(NewBook {
                id,
                title,
                author,
                year,
                publisher,
            });
        }
    }

    println!("Pushing books by chunks");
    for chunk in books.chunks(10_000).progress() {
        insert_into(books_sc::table).values(chunk).execute(conn)?;
    }

    Ok(())
}

fn insert_ratings(conn: &PgConnection) -> Result<(), Error> {
    let mut csv = csv::ReaderBuilder::new()
        .has_headers(false)
        .delimiter(b',')
        .from_path("data/ratings.csv")?;

    let mut ratings = Vec::new();
    println!("Collecting records for ratings...");
    let records: Vec<_> = csv.records().collect();

    let controller = BooksController::new()?;
    for record in records.iter().progress() {
        if let Ok(record) = record {
            let user_id: i32 = record[0].parse()?;
            let book_id = &record[1];
            let score: f64 = record[2].parse()?;

            let book_res = controller.item_by_id(&book_id.into());
            if book_res.is_err() {
                continue;
            }

            ratings.push(NewRating {
                score,
                user_id,
                book_id,
            });
        }
    }

    println!("Pushing ratings by chunks");
    for chunk in ratings.chunks(10_000).progress() {
        insert_into(ratings::table).values(chunk).execute(conn)?;
    }

    Ok(())
}

fn main() -> Result<(), Error> {
    let url = "postgres://postgres:@localhost/books";
    let conn = establish_connection(url)?;

    insert_users(&conn)?;
    insert_books(&conn)?;
    insert_ratings(&conn)?;
    Ok(())
}