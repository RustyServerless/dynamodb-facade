use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use dynamodb_facade::{
    DynamoDBItem, attr_list, dynamodb_item, table_definitions, test_fixtures::*,
};

table_definitions! {
    SimpleTable {
        type PartitionKey = PK;
        fn table_name() -> String {
            "simple".to_owned()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User<T> {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    _marker: PhantomData<T>,
}

// Simple Table tests
// Minimal
struct SimpleTest11;
dynamodb_item! {
    #[table = SimpleTable]
    User<SimpleTest11> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}
struct SimpleTest21;
// With additional attr only
dynamodb_item! {
    #[table = SimpleTable]
    User<SimpleTest21> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        ItemType { const VALUE: &'static str = "USER"; }
    }
}
struct SimpleTest22;
dynamodb_item! {
    #[table = SimpleTable]
    User<SimpleTest22> {
        ItemType { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}

// With marker only
struct SimpleTest31;
dynamodb_item! {
    #[table = SimpleTable]
    User<SimpleTest31> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
    }
}

struct SimpleTest32;
dynamodb_item! {
    #[table = SimpleTable]
    User<SimpleTest32> {
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}

// With additional attr and marker
struct SimpleTest41;
dynamodb_item! {
    #[table = SimpleTable]
    User<SimpleTest41> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        ItemType { const VALUE: &'static str = "USER"; }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
    }
}
struct SimpleTest42;
dynamodb_item! {
    #[table = SimpleTable]
    User<SimpleTest42> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        ItemType { const VALUE: &'static str = "USER"; }
    }
}
struct SimpleTest43;
dynamodb_item! {
    #[table = SimpleTable]
    User<SimpleTest43> {
        ItemType { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
    }
}
struct SimpleTest44;
dynamodb_item! {
    #[table = SimpleTable]
    User<SimpleTest44> {
        ItemType { const VALUE: &'static str = "USER"; }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}
struct SimpleTest45;
dynamodb_item! {
    #[table = SimpleTable]
    User<SimpleTest45> {
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        ItemType { const VALUE: &'static str = "USER"; }
    }
}
struct SimpleTest46;
dynamodb_item! {
    #[table = SimpleTable]
    User<SimpleTest46> {
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        ItemType { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}

// Composite Table tests
// Minimal
struct CompositeTest11;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest11> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
    }
}
struct CompositeTest12;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest12> {
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}

// With additional attr only
struct CompositeTest21;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest21> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        ItemType { const VALUE: &'static str = "USER"; }
    }
}

struct CompositeTest22;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest22> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        ItemType { const VALUE: &'static str = "USER"; }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
    }
}

struct CompositeTest23;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest23> {
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        ItemType { const VALUE: &'static str = "USER"; }
    }
}

struct CompositeTest24;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest24> {
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        ItemType { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}

struct CompositeTest25;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest25> {
        ItemType { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
    }
}

struct CompositeTest26;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest26> {
        ItemType { const VALUE: &'static str = "USER"; }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}

// With marker only
struct CompositeTest31;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest31> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
    }
}

struct CompositeTest32;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest32> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
    }
}

struct CompositeTest33;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest33> {
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
    }
}

struct CompositeTest34;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest34> {
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}

struct CompositeTest35;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest35> {
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
    }
}

struct CompositeTest36;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest36> {
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}

// With additional attr and marker (just a few, not all 24 combinations)

struct CompositeTest41;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest41> {
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        ItemType { const VALUE: &'static str = "USER"; }
    }
}

struct CompositeTest42;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest42> {
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        ItemType { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}

struct CompositeTest43;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest43> {
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        ItemType { const VALUE: &'static str = "USER"; }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}

struct CompositeTest44;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest44> {
        ItemType { const VALUE: &'static str = "USER"; }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
    }
}

struct CompositeTest45;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest45> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        ItemType { const VALUE: &'static str = "USER"; }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
    }
}

struct CompositeTest46;
dynamodb_item! {
    #[table = PlatformTable]
    User<CompositeTest46> {
        #[partition_key]
        PK {
            fn attribute_id(&self) -> &'id str { &self.id }
            fn attribute_value(id) -> String { format!("USER#{id}") }
        }
        #[sort_key]
        SK { const VALUE: &'static str = "USER"; }
        #[marker_only]
        Email {
            fn attribute_id(&self) -> &'id str { &self.email }
            fn attribute_value(id) -> String { id.to_owned() }
        }
        ItemType { const VALUE: &'static str = "USER"; }
    }
}

fn _assert_empty_additional(_: attr_list![]) {}
fn _assert_type_additional(_: attr_list![ItemType]) {}
type AddAttr<U, TD> = <U as DynamoDBItem<TD>>::AdditionalAttributes;

fn main() {
    const _: fn(AddAttr<User<SimpleTest11>, SimpleTable>) = _assert_empty_additional;
    const _: fn(AddAttr<User<SimpleTest21>, SimpleTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<SimpleTest22>, SimpleTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<SimpleTest31>, SimpleTable>) = _assert_empty_additional;
    const _: fn(AddAttr<User<SimpleTest32>, SimpleTable>) = _assert_empty_additional;
    const _: fn(AddAttr<User<SimpleTest41>, SimpleTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<SimpleTest42>, SimpleTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<SimpleTest43>, SimpleTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<SimpleTest44>, SimpleTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<SimpleTest45>, SimpleTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<SimpleTest46>, SimpleTable>) = _assert_type_additional;

    const _: fn(AddAttr<User<CompositeTest11>, PlatformTable>) = _assert_empty_additional;
    const _: fn(AddAttr<User<CompositeTest12>, PlatformTable>) = _assert_empty_additional;

    const _: fn(AddAttr<User<CompositeTest21>, PlatformTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<CompositeTest22>, PlatformTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<CompositeTest23>, PlatformTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<CompositeTest24>, PlatformTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<CompositeTest25>, PlatformTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<CompositeTest26>, PlatformTable>) = _assert_type_additional;

    const _: fn(AddAttr<User<CompositeTest31>, PlatformTable>) = _assert_empty_additional;
    const _: fn(AddAttr<User<CompositeTest32>, PlatformTable>) = _assert_empty_additional;
    const _: fn(AddAttr<User<CompositeTest33>, PlatformTable>) = _assert_empty_additional;
    const _: fn(AddAttr<User<CompositeTest34>, PlatformTable>) = _assert_empty_additional;
    const _: fn(AddAttr<User<CompositeTest35>, PlatformTable>) = _assert_empty_additional;
    const _: fn(AddAttr<User<CompositeTest36>, PlatformTable>) = _assert_empty_additional;

    const _: fn(AddAttr<User<CompositeTest41>, PlatformTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<CompositeTest42>, PlatformTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<CompositeTest43>, PlatformTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<CompositeTest44>, PlatformTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<CompositeTest45>, PlatformTable>) = _assert_type_additional;
    const _: fn(AddAttr<User<CompositeTest46>, PlatformTable>) = _assert_type_additional;
}
