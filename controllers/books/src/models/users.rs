// Copyright (c) 2020 White Leaf
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use crate::schema::{means, users};
use common_macros::hash_map;
use controller::Entity;
use std::collections::HashMap;

// To query data from the database
#[derive(Debug, Clone, Identifiable, Queryable, Default)]
pub struct User {
    pub id: i32,
    pub location: String,
    pub age: Option<i16>,
}

impl Entity for User {
    type Id = i32;
    fn get_id(&self) -> Self::Id {
        self.id
    }

    fn get_data(&self) -> HashMap<String, String> {
        let mut map = hash_map! {
            "location".into() => self.location.clone()
        };

        if let Some(age) = &self.age {
            map.insert("age".into(), age.to_string());
        }

        map
    }
}

// To insert a new user into the database
#[derive(Debug, Clone, Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub id: i32,
    pub location: &'a str,
    pub age: Option<i16>,
}

#[derive(Debug, Clone, Insertable)]
#[table_name = "users"]
pub struct NewUnseenUser<'a> {
    pub location: &'a str,
    pub age: Option<i16>,
}

#[derive(Debug, Clone, Identifiable, Queryable, Associations)]
#[primary_key(user_id)]
#[belongs_to(User)]
pub struct Mean {
    pub user_id: i32,
    pub val: f64,
    pub score_number: i32,
}

#[derive(Debug, Clone, Insertable)]
#[table_name = "means"]
pub struct NewMean {
    pub user_id: i32,
    pub val: f64,
    pub score_number: i32,
}
