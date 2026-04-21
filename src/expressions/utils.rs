use core::{cmp::Ordering, fmt};
use std::borrow::Cow;

use super::*;

/// Sorted list of DynamoDB reserved words used for binary-search lookups.
const SORTED_DYNAMODB_RESERVED_WORDS: [&str; 573] = [
    "ABORT",
    "ABSOLUTE",
    "ACTION",
    "ADD",
    "AFTER",
    "AGENT",
    "AGGREGATE",
    "ALL",
    "ALLOCATE",
    "ALTER",
    "ANALYZE",
    "AND",
    "ANY",
    "ARCHIVE",
    "ARE",
    "ARRAY",
    "AS",
    "ASC",
    "ASCII",
    "ASENSITIVE",
    "ASSERTION",
    "ASYMMETRIC",
    "AT",
    "ATOMIC",
    "ATTACH",
    "ATTRIBUTE",
    "AUTH",
    "AUTHORIZATION",
    "AUTHORIZE",
    "AUTO",
    "AVG",
    "BACK",
    "BACKUP",
    "BASE",
    "BATCH",
    "BEFORE",
    "BEGIN",
    "BETWEEN",
    "BIGINT",
    "BINARY",
    "BIT",
    "BLOB",
    "BLOCK",
    "BOOLEAN",
    "BOTH",
    "BREADTH",
    "BUCKET",
    "BULK",
    "BY",
    "BYTE",
    "CALL",
    "CALLED",
    "CALLING",
    "CAPACITY",
    "CASCADE",
    "CASCADED",
    "CASE",
    "CAST",
    "CATALOG",
    "CHAR",
    "CHARACTER",
    "CHECK",
    "CLASS",
    "CLOB",
    "CLOSE",
    "CLUSTER",
    "CLUSTERED",
    "CLUSTERING",
    "CLUSTERS",
    "COALESCE",
    "COLLATE",
    "COLLATION",
    "COLLECTION",
    "COLUMN",
    "COLUMNS",
    "COMBINE",
    "COMMENT",
    "COMMIT",
    "COMPACT",
    "COMPILE",
    "COMPRESS",
    "CONDITION",
    "CONFLICT",
    "CONNECT",
    "CONNECTION",
    "CONSISTENCY",
    "CONSISTENT",
    "CONSTRAINT",
    "CONSTRAINTS",
    "CONSTRUCTOR",
    "CONSUMED",
    "CONTINUE",
    "CONVERT",
    "COPY",
    "CORRESPONDING",
    "COUNT",
    "COUNTER",
    "CREATE",
    "CROSS",
    "CUBE",
    "CURRENT",
    "CURSOR",
    "CYCLE",
    "DATA",
    "DATABASE",
    "DATE",
    "DATETIME",
    "DAY",
    "DEALLOCATE",
    "DEC",
    "DECIMAL",
    "DECLARE",
    "DEFAULT",
    "DEFERRABLE",
    "DEFERRED",
    "DEFINE",
    "DEFINED",
    "DEFINITION",
    "DELETE",
    "DELIMITED",
    "DEPTH",
    "DEREF",
    "DESC",
    "DESCRIBE",
    "DESCRIPTOR",
    "DETACH",
    "DETERMINISTIC",
    "DIAGNOSTICS",
    "DIRECTORIES",
    "DISABLE",
    "DISCONNECT",
    "DISTINCT",
    "DISTRIBUTE",
    "DO",
    "DOMAIN",
    "DOUBLE",
    "DROP",
    "DUMP",
    "DURATION",
    "DYNAMIC",
    "EACH",
    "ELEMENT",
    "ELSE",
    "ELSEIF",
    "EMPTY",
    "ENABLE",
    "END",
    "EQUAL",
    "EQUALS",
    "ERROR",
    "ESCAPE",
    "ESCAPED",
    "EVAL",
    "EVALUATE",
    "EXCEEDED",
    "EXCEPT",
    "EXCEPTION",
    "EXCEPTIONS",
    "EXCLUSIVE",
    "EXEC",
    "EXECUTE",
    "EXISTS",
    "EXIT",
    "EXPLAIN",
    "EXPLODE",
    "EXPORT",
    "EXPRESSION",
    "EXTENDED",
    "EXTERNAL",
    "EXTRACT",
    "FAIL",
    "FALSE",
    "FAMILY",
    "FETCH",
    "FIELDS",
    "FILE",
    "FILTER",
    "FILTERING",
    "FINAL",
    "FINISH",
    "FIRST",
    "FIXED",
    "FLATTERN",
    "FLOAT",
    "FOR",
    "FORCE",
    "FOREIGN",
    "FORMAT",
    "FORWARD",
    "FOUND",
    "FREE",
    "FROM",
    "FULL",
    "FUNCTION",
    "FUNCTIONS",
    "GENERAL",
    "GENERATE",
    "GET",
    "GLOB",
    "GLOBAL",
    "GO",
    "GOTO",
    "GRANT",
    "GREATER",
    "GROUP",
    "GROUPING",
    "HANDLER",
    "HASH",
    "HAVE",
    "HAVING",
    "HEAP",
    "HIDDEN",
    "HOLD",
    "HOUR",
    "IDENTIFIED",
    "IDENTITY",
    "IF",
    "IGNORE",
    "IMMEDIATE",
    "IMPORT",
    "IN",
    "INCLUDING",
    "INCLUSIVE",
    "INCREMENT",
    "INCREMENTAL",
    "INDEX",
    "INDEXED",
    "INDEXES",
    "INDICATOR",
    "INFINITE",
    "INITIALLY",
    "INLINE",
    "INNER",
    "INNTER",
    "INOUT",
    "INPUT",
    "INSENSITIVE",
    "INSERT",
    "INSTEAD",
    "INT",
    "INTEGER",
    "INTERSECT",
    "INTERVAL",
    "INTO",
    "INVALIDATE",
    "IS",
    "ISOLATION",
    "ITEM",
    "ITEMS",
    "ITERATE",
    "JOIN",
    "KEY",
    "KEYS",
    "LAG",
    "LANGUAGE",
    "LARGE",
    "LAST",
    "LATERAL",
    "LEAD",
    "LEADING",
    "LEAVE",
    "LEFT",
    "LENGTH",
    "LESS",
    "LEVEL",
    "LIKE",
    "LIMIT",
    "LIMITED",
    "LINES",
    "LIST",
    "LOAD",
    "LOCAL",
    "LOCALTIME",
    "LOCALTIMESTAMP",
    "LOCATION",
    "LOCATOR",
    "LOCK",
    "LOCKS",
    "LOG",
    "LOGED",
    "LONG",
    "LOOP",
    "LOWER",
    "MAP",
    "MATCH",
    "MATERIALIZED",
    "MAX",
    "MAXLEN",
    "MEMBER",
    "MERGE",
    "METHOD",
    "METRICS",
    "MIN",
    "MINUS",
    "MINUTE",
    "MISSING",
    "MOD",
    "MODE",
    "MODIFIES",
    "MODIFY",
    "MODULE",
    "MONTH",
    "MULTI",
    "MULTISET",
    "NAME",
    "NAMES",
    "NATIONAL",
    "NATURAL",
    "NCHAR",
    "NCLOB",
    "NEW",
    "NEXT",
    "NO",
    "NONE",
    "NOT",
    "NULL",
    "NULLIF",
    "NUMBER",
    "NUMERIC",
    "OBJECT",
    "OF",
    "OFFLINE",
    "OFFSET",
    "OLD",
    "ON",
    "ONLINE",
    "ONLY",
    "OPAQUE",
    "OPEN",
    "OPERATOR",
    "OPTION",
    "OR",
    "ORDER",
    "ORDINALITY",
    "OTHER",
    "OTHERS",
    "OUT",
    "OUTER",
    "OUTPUT",
    "OVER",
    "OVERLAPS",
    "OVERRIDE",
    "OWNER",
    "PAD",
    "PARALLEL",
    "PARAMETER",
    "PARAMETERS",
    "PARTIAL",
    "PARTITION",
    "PARTITIONED",
    "PARTITIONS",
    "PATH",
    "PERCENT",
    "PERCENTILE",
    "PERMISSION",
    "PERMISSIONS",
    "PIPE",
    "PIPELINED",
    "PLAN",
    "POOL",
    "POSITION",
    "PRECISION",
    "PREPARE",
    "PRESERVE",
    "PRIMARY",
    "PRIOR",
    "PRIVATE",
    "PRIVILEGES",
    "PROCEDURE",
    "PROCESSED",
    "PROJECT",
    "PROJECTION",
    "PROPERTY",
    "PROVISIONING",
    "PUBLIC",
    "PUT",
    "QUERY",
    "QUIT",
    "QUORUM",
    "RAISE",
    "RANDOM",
    "RANGE",
    "RANK",
    "RAW",
    "READ",
    "READS",
    "REAL",
    "REBUILD",
    "RECORD",
    "RECURSIVE",
    "REDUCE",
    "REF",
    "REFERENCE",
    "REFERENCES",
    "REFERENCING",
    "REGEXP",
    "REGION",
    "REINDEX",
    "RELATIVE",
    "RELEASE",
    "REMAINDER",
    "RENAME",
    "REPEAT",
    "REPLACE",
    "REQUEST",
    "RESET",
    "RESIGNAL",
    "RESOURCE",
    "RESPONSE",
    "RESTORE",
    "RESTRICT",
    "RESULT",
    "RETURN",
    "RETURNING",
    "RETURNS",
    "REVERSE",
    "REVOKE",
    "RIGHT",
    "ROLE",
    "ROLES",
    "ROLLBACK",
    "ROLLUP",
    "ROUTINE",
    "ROW",
    "ROWS",
    "RULE",
    "RULES",
    "SAMPLE",
    "SATISFIES",
    "SAVE",
    "SAVEPOINT",
    "SCAN",
    "SCHEMA",
    "SCOPE",
    "SCROLL",
    "SEARCH",
    "SECOND",
    "SECTION",
    "SEGMENT",
    "SEGMENTS",
    "SELECT",
    "SELF",
    "SEMI",
    "SENSITIVE",
    "SEPARATE",
    "SEQUENCE",
    "SERIALIZABLE",
    "SESSION",
    "SET",
    "SETS",
    "SHARD",
    "SHARE",
    "SHARED",
    "SHORT",
    "SHOW",
    "SIGNAL",
    "SIMILAR",
    "SIZE",
    "SKEWED",
    "SMALLINT",
    "SNAPSHOT",
    "SOME",
    "SOURCE",
    "SPACE",
    "SPACES",
    "SPARSE",
    "SPECIFIC",
    "SPECIFICTYPE",
    "SPLIT",
    "SQL",
    "SQLCODE",
    "SQLERROR",
    "SQLEXCEPTION",
    "SQLSTATE",
    "SQLWARNING",
    "START",
    "STATE",
    "STATIC",
    "STATUS",
    "STORAGE",
    "STORE",
    "STORED",
    "STREAM",
    "STRING",
    "STRUCT",
    "STYLE",
    "SUB",
    "SUBMULTISET",
    "SUBPARTITION",
    "SUBSTRING",
    "SUBTYPE",
    "SUM",
    "SUPER",
    "SYMMETRIC",
    "SYNONYM",
    "SYSTEM",
    "TABLE",
    "TABLESAMPLE",
    "TEMP",
    "TEMPORARY",
    "TERMINATED",
    "TEXT",
    "THAN",
    "THEN",
    "THROUGHPUT",
    "TIME",
    "TIMESTAMP",
    "TIMEZONE",
    "TINYINT",
    "TO",
    "TOKEN",
    "TOTAL",
    "TOUCH",
    "TRAILING",
    "TRANSACTION",
    "TRANSFORM",
    "TRANSLATE",
    "TRANSLATION",
    "TREAT",
    "TRIGGER",
    "TRIM",
    "TRUE",
    "TRUNCATE",
    "TTL",
    "TUPLE",
    "TYPE",
    "UNDER",
    "UNDO",
    "UNION",
    "UNIQUE",
    "UNIT",
    "UNKNOWN",
    "UNLOGGED",
    "UNNEST",
    "UNPROCESSED",
    "UNSIGNED",
    "UNTIL",
    "UPDATE",
    "UPPER",
    "URL",
    "USAGE",
    "USE",
    "USER",
    "USERS",
    "USING",
    "UUID",
    "VACUUM",
    "VALUE",
    "VALUED",
    "VALUES",
    "VARCHAR",
    "VARIABLE",
    "VARIANCE",
    "VARINT",
    "VARYING",
    "VIEW",
    "VIEWS",
    "VIRTUAL",
    "VOID",
    "WAIT",
    "WHEN",
    "WHENEVER",
    "WHERE",
    "WHILE",
    "WINDOW",
    "WITH",
    "WITHIN",
    "WITHOUT",
    "WORK",
    "WRAPPED",
    "WRITE",
    "YEAR",
    "ZONE",
];

/// Compares `reserved` (already uppercase) against `word` (any case) byte-by-byte.
///
/// Used as the comparator for binary search in [`SORTED_DYNAMODB_RESERVED_WORDS`].
fn cmp_upper_case_ascii(reserved: &str, word: &str) -> Ordering {
    reserved
        .bytes()
        .zip(word.bytes())
        .map(|(r, w)| r.cmp(&w.to_ascii_uppercase()))
        .find(|&ordering| ordering != Ordering::Equal)
        .unwrap_or(reserved.len().cmp(&word.len()))
}

/// Returns `true` if `word` is a DynamoDB reserved word (case-insensitive).
fn is_reserved_word(word: &str) -> bool {
    debug_assert!(word.is_ascii());
    SORTED_DYNAMODB_RESERVED_WORDS
        .binary_search_by(|reserved| cmp_upper_case_ascii(reserved, word))
        .is_ok()
}

/// Returns `true` if `word` must be replaced with a `#name` placeholder in an expression.
///
/// A word must be aliased if it starts with a non-alpha character, contains `#` or `:`,
/// or is a DynamoDB reserved word.
fn must_be_aliased(word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let mut char_iter = word.chars();
    if !char_iter.next().expect("not empty").is_ascii_alphabetic() {
        return true;
    }
    let second_char = char_iter.next();
    if second_char.is_none() {
        return false;
    }
    if second_char.is_some_and(|c| !c.is_ascii_alphanumeric()) {
        return true;
    }

    if char_iter.any(|c| c == '#' || c == ':') {
        return true;
    }

    is_reserved_word(word)
}

/// Resolves a dotted attribute path into an expression string and name placeholders.
///
/// Segments that must be aliased are replaced with `#{prefix}{counter}` placeholders,
/// and the corresponding `(placeholder, real_name)` pairs are returned. Bracket
/// index suffixes (e.g. `[0]`) are preserved verbatim.
pub(super) fn resolve_attr_path<'a>(
    attr: &'a str,
    prefix: &str,
    counter: &mut usize,
) -> (Cow<'a, str>, AttrNames) {
    let placeholder_needed = attr.split('.').any(|segment| {
        if let Some((ident, _)) = segment.split_once('[') {
            must_be_aliased(ident)
        } else {
            must_be_aliased(segment)
        }
    });

    if placeholder_needed {
        let mut names: AttrNames = Vec::new();
        let mut result = String::new();

        for (i, segment) in attr.split('.').enumerate() {
            if i > 0 {
                result.push('.');
            }

            // A segment may contain a bracket index, e.g. "items[0]" or "items[0][1]".
            // Split at the first '[' to isolate the identifier part.
            let (ident, bracket_suffix) = match segment.find('[') {
                Some(split) => (&segment[..split], &segment[split..]),
                None => (segment, ""),
            };

            if must_be_aliased(ident) {
                let ph = format!("#{prefix}{}", *counter);
                *counter += 1;
                result.push_str(&ph);
                names.push((ph, ident.to_owned()));
            } else {
                result.push_str(ident);
            }

            result.push_str(bracket_suffix);
        }

        (result.into(), names)
    } else {
        (attr.into(), vec![])
    }
}

// -- Display helpers ----------------------------------------------------------

/// Substitutes all name and value placeholders in `expression` with their real values.
///
/// Longer placeholders are replaced first to avoid partial matches.
pub(super) fn resolve_expression(
    expression: &str,
    names: &AttrNames,
    values: &AttrValues,
) -> String {
    // Collect all replacements (placeholder → replacement string).
    let mut replacements: Vec<(&str, String)> = Vec::with_capacity(names.len() + values.len());

    for (placeholder, real_name) in names {
        replacements.push((placeholder.as_str(), real_name.clone()));
    }
    for (placeholder, attr_value) in values {
        replacements.push((placeholder.as_str(), format!("{attr_value:?}")));
    }

    // Sort by placeholder length descending so longer placeholders are
    // replaced first, preventing partial matches.
    replacements.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    let mut result = expression.to_owned();
    for (placeholder, replacement) in &replacements {
        result = result.replace(placeholder, replacement);
    }
    result
}

/// Writes the name and value placeholder maps to a formatter (used by `Display` alternate mode).
pub(super) fn fmt_attr_maps(
    f: &mut fmt::Formatter<'_>,
    names: &AttrNames,
    values: &AttrValues,
) -> fmt::Result {
    if !names.is_empty() {
        f.write_str("\n  names: { ")?;
        for (i, (placeholder, real_name)) in names.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{placeholder} = {real_name}")?;
        }
        f.write_str(" }")?;
    }

    if !values.is_empty() {
        f.write_str("\n  values: { ")?;
        for (i, (placeholder, attr_value)) in values.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{placeholder} = {attr_value:?}")?;
        }
        f.write_str(" }")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    // Tests for cmp_upper_case_ascii

    #[test]
    fn test_cmp_upper_case_ascii_equal() {
        // Same case
        assert_eq!(cmp_upper_case_ascii("ABC", "ABC"), Ordering::Equal);

        // Reserved uppercase, word lowercase
        assert_eq!(cmp_upper_case_ascii("ABC", "abc"), Ordering::Equal);

        // Reserved uppercase, word mixed case
        assert_eq!(cmp_upper_case_ascii("ABC", "aBc"), Ordering::Equal);

        // Both empty
        assert_eq!(cmp_upper_case_ascii("", ""), Ordering::Equal);

        // Single char, different case
        assert_eq!(cmp_upper_case_ascii("A", "a"), Ordering::Equal);
    }

    #[test]
    fn test_cmp_upper_case_ascii_not_equal() {
        // First is less (alphabetically earlier)
        assert_eq!(cmp_upper_case_ascii("ABC", "DEF"), Ordering::Less);

        // First is greater (alphabetically later)
        assert_eq!(cmp_upper_case_ascii("DEF", "ABC"), Ordering::Greater);

        // Reserved is prefix of word (shorter → Less)
        assert_eq!(cmp_upper_case_ascii("AB", "ABC"), Ordering::Less);

        // Word is prefix of reserved (longer → Greater)
        assert_eq!(cmp_upper_case_ascii("ABC", "AB"), Ordering::Greater);

        // Empty vs non-empty
        assert_eq!(cmp_upper_case_ascii("", "A"), Ordering::Less);

        // Non-empty vs empty
        assert_eq!(cmp_upper_case_ascii("A", ""), Ordering::Greater);

        // Differ at second character
        assert_eq!(cmp_upper_case_ascii("AB", "AC"), Ordering::Less);
    }

    #[test]
    fn test_cmp_upper_case_ascii_variable_lengths() {
        // Short vs long, differ at first byte (Z > A)
        assert_eq!(cmp_upper_case_ascii("Z", "ABCDEF"), Ordering::Greater);

        // Long vs short, differ at first byte (A < Z)
        assert_eq!(cmp_upper_case_ascii("ABCDEF", "Z"), Ordering::Less);

        // Short is prefix of long (shorter length → Less)
        assert_eq!(cmp_upper_case_ascii("AB", "ABCDEF"), Ordering::Less);

        // Long vs short prefix (longer length → Greater)
        assert_eq!(cmp_upper_case_ascii("ABCDEF", "AB"), Ordering::Greater);

        // Single char vs multi-char, equal start (shorter → Less)
        assert_eq!(cmp_upper_case_ascii("A", "ABC"), Ordering::Less);

        // Multi-char vs single char, equal start (longer → Greater)
        assert_eq!(cmp_upper_case_ascii("ABC", "A"), Ordering::Greater);

        // Same prefix bytes, very different lengths (shorter → Less)
        assert_eq!(cmp_upper_case_ascii("TEST", "TESTING123"), Ordering::Less);

        // Same prefix bytes reversed, very different lengths (longer → Greater)
        assert_eq!(
            cmp_upper_case_ascii("TESTING123", "TEST"),
            Ordering::Greater
        );
    }

    // Tests for is_reserved_word

    #[test]
    fn test_is_reserved_word_returns_true_for_reserved_words() {
        // Uppercase reserved words → found → true
        assert!(is_reserved_word("SELECT"));
        assert!(is_reserved_word("FROM"));
        assert!(is_reserved_word("WHERE"));
        assert!(is_reserved_word("TABLE"));

        // First and last entries in the list
        assert!(is_reserved_word("ABORT"));
        assert!(is_reserved_word("ZONE"));

        // Lowercase reserved words → case-insensitive match → found → true
        assert!(is_reserved_word("select"));
        assert!(is_reserved_word("from"));
        assert!(is_reserved_word("where"));

        // Mixed case reserved words → found → true
        assert!(is_reserved_word("Select"));
        assert!(is_reserved_word("fRoM"));
    }

    #[test]
    fn test_is_reserved_word_returns_false_for_non_reserved_words() {
        // Arbitrary non-reserved words → not found → false
        assert!(!is_reserved_word("foo"));
        assert!(!is_reserved_word("bar"));
        assert!(!is_reserved_word("mycolumn"));
        assert!(!is_reserved_word("NOTARESERVEDWORD"));

        // Empty string → not in list → false
        assert!(!is_reserved_word(""));
    }

    // Tests for resolve_attr_path

    #[test]
    fn test_resolve_attr_path_simple_non_reserved() {
        let mut counter = 0;
        let (expr, names) = resolve_attr_path("balance", "u", &mut counter);
        assert_eq!(expr, "balance");
        assert!(names.is_empty());
        assert_eq!(counter, 0);
    }

    #[test]
    fn test_resolve_attr_path_simple_reserved() {
        let mut counter = 0;
        let (expr, names) = resolve_attr_path("Status", "u", &mut counter);
        assert_eq!(expr, "#u0");
        assert_eq!(names, vec![("#u0".to_string(), "Status".to_string())]);
        assert_eq!(counter, 1);
    }

    #[test]
    fn test_resolve_attr_path_dotted_no_reserved() {
        let mut counter = 0;
        let (expr, names) = resolve_attr_path("nested.attr", "c", &mut counter);
        assert_eq!(expr, "nested.attr");
        assert!(names.is_empty());
        assert_eq!(counter, 0);
    }

    #[test]
    fn test_resolve_attr_path_dotted_with_reserved_segment() {
        let mut counter = 0;
        let (expr, names) = resolve_attr_path("attr1.Status.attr2", "u", &mut counter);
        assert_eq!(expr, "attr1.#u0.attr2");
        assert_eq!(names, vec![("#u0".to_string(), "Status".to_string())]);
        assert_eq!(counter, 1);
    }

    #[test]
    fn test_resolve_attr_path_indexed_non_reserved() {
        let mut counter = 0;
        let (expr, names) = resolve_attr_path("list_attr[0]", "u", &mut counter);
        assert_eq!(expr, "list_attr[0]");
        assert!(names.is_empty());
    }

    #[test]
    fn test_resolve_attr_path_indexed_reserved() {
        let mut counter = 5;
        let (expr, names) = resolve_attr_path("Zone[3]", "c", &mut counter);
        assert_eq!(expr, "#c5[3]");
        assert_eq!(names, vec![("#c5".to_string(), "Zone".to_string())]);
        assert_eq!(counter, 6);
    }

    #[test]
    fn test_resolve_attr_path_mixed_complex() {
        let mut counter = 0;
        let (expr, names) = resolve_attr_path("attr.Status.list_attr[0].Name", "u", &mut counter);
        assert_eq!(expr, "attr.#u0.list_attr[0].#u1");
        assert_eq!(
            names,
            vec![
                ("#u0".to_string(), "Status".to_string()),
                ("#u1".to_string(), "Name".to_string()),
            ]
        );
        assert_eq!(counter, 2);
    }

    #[test]
    fn test_resolve_attr_path_multiple_brackets() {
        let mut counter = 0;
        let (expr, names) = resolve_attr_path("matrix[0][1]", "u", &mut counter);
        assert_eq!(expr, "matrix[0][1]");
        assert!(names.is_empty());
    }

    #[test]
    fn test_resolve_attr_path_counter_continues() {
        let mut counter = 3;
        let (expr, names) = resolve_attr_path("Select", "u", &mut counter);
        assert_eq!(expr, "#u3");
        assert_eq!(names, vec![("#u3".to_string(), "Select".to_string())]);
        assert_eq!(counter, 4);
    }
}
