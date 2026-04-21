use dynamodb_facade::test_fixtures::*;

dynamodb_facade::table_definitions! {

    /// With comments
    SimpleTable1 {
        type PartitionKey = PK;
        fn table_name() -> String { "simple".to_owned() }
    }

    /// With comments
    #[derive(Debug)]
    SimpleTable2 {
        fn table_name() -> String { "simple".to_owned() }
        type PartitionKey = PK;
    }

    CompositeTable1 {
        type PartitionKey = PK;
        type SortKey = SK;
        fn table_name() -> String { "composite".to_owned() }
    }
    /// With comments
    #[derive(Debug)]
    CompositeTable2 {
        type SortKey = SK;
        type PartitionKey = PK;
        fn table_name() -> String { "composite".to_owned() }
    }
    CompositeTable3 {
        type PartitionKey = PK;
        fn table_name() -> String { "composite".to_owned() }
        type SortKey = SK;
    }
    CompositeTable4 {
        type SortKey = SK;
        fn table_name() -> String { "composite".to_owned() }
        type PartitionKey = PK;
    }
    CompositeTable5 {
        fn table_name() -> String { "composite".to_owned() }
        type PartitionKey = PK;
        type SortKey = SK;
    }
    CompositeTable6 {
        fn table_name() -> String { "composite".to_owned() }
        type SortKey = SK;
        type PartitionKey = PK;
    }
}

fn main() {}
