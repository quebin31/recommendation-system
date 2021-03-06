// Copyright (c) 2020 White Leaf
//
// This software is released under the MIT License.
// https://opensource.org/licenses/MIT

use crate::schema::movies;
use common_macros::hash_map;
use controller::Entity;
use std::collections::HashMap;

// To query data from the database
#[derive(Debug, Clone, Identifiable, Queryable, Default)]
pub struct Movie {
    pub id: i32,
    pub name: String,
}

impl Entity for Movie {
    type Id = i32;

    fn get_id(&self) -> Self::Id {
        self.id
    }

    fn get_data(&self) -> HashMap<String, String> {
        hash_map! {
            "name".into() => self.name.clone()
        }
    }
}

// To insert a new movie into the database
#[derive(Debug, Clone, Insertable)]
#[table_name = "movies"]
pub struct NewMovie<'a> {
    pub name: &'a str,
}
