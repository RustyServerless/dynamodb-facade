use dynamodb_facade::test_fixtures::*;

dynamodb_facade::key_schema! {
    SimpleSchema {
        type PartitionKey = PK;
    }
}

dynamodb_facade::key_schema! {
    CompositeSchema1 {
        type PartitionKey = PK;
        type SortKey = SK;
    }
}

dynamodb_facade::key_schema! {
    CompositeSchema2 {
        type SortKey = SK;
        type PartitionKey = PK;
    }
}

fn main() {}
