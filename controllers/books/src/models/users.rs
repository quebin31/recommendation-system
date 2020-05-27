use crate::schema::users;
use common_macros::hash_map;
use controller::{Entity, Id};
use std::collections::HashMap;

// To query data from the database
#[derive(Debug, Clone, Identifiable, Queryable)]
pub struct User {
    pub id: i32,
    pub location: String,
    pub age: Option<i16>,
}

impl Entity for User {
    fn get_id(&self) -> Id {
        self.id.into()
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