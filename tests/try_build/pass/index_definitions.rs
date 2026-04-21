use dynamodb_facade::test_fixtures::*;

dynamodb_facade::index_definitions! {

    /// With comments
    #[derive(Debug)]
    #[table = PlatformTable]
    SimpleIndex1 {
        type PartitionKey = PK;
        fn index_name() -> String { "simple".to_owned() }
    }

    #[table = PlatformTable]
    /// With comments
    #[derive(Debug)]
    SimpleIndex2 {
        fn index_name() -> String { "simple".to_owned() }
        type PartitionKey = PK;
    }

    #[table = PlatformTable]
    /// With comments
    CompositeIndex1 {
        type PartitionKey = PK;
        type SortKey = SK;
        fn index_name() -> String { "composite".to_owned() }
    }
    #[table = PlatformTable]
    #[derive(Debug)]
    CompositeIndex2 {
        type SortKey = SK;
        type PartitionKey = PK;
        fn index_name() -> String { "composite".to_owned() }
    }
    #[derive(Debug)]
    #[table = PlatformTable]
    CompositeIndex3 {
        type PartitionKey = PK;
        fn index_name() -> String { "composite".to_owned() }
        type SortKey = SK;
    }
    /// With comments
    #[table = PlatformTable]
    #[derive(Debug)]
    CompositeIndex4 {
        type SortKey = SK;
        fn index_name() -> String { "composite".to_owned() }
        type PartitionKey = PK;
    }
    #[derive(Debug)]
    #[table = PlatformTable]
    /// With comments
    CompositeIndex5 {
        fn index_name() -> String { "composite".to_owned() }
        type PartitionKey = PK;
        type SortKey = SK;
    }
    #[table = PlatformTable]
    CompositeIndex6 {
        fn index_name() -> String { "composite".to_owned() }
        type SortKey = SK;
        type PartitionKey = PK;
    }
}

fn main() {}
