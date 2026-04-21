use serde::{Deserialize, Serialize};

use dynamodb_facade::{dynamodb_item, test_fixtures::*};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
}

dynamodb_item! {
    #[table = PlatformTable]
    User {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}

fn main() {}
