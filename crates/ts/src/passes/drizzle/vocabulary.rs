//! The Drizzle names the query pass recognises.

/// Identifiers conventionally bound to a Drizzle database or transaction.
const RECEIVERS: [&str; 2] = ["db", "tx"];

/// Builder methods that continue a query chain. No rule reads them yet.
#[cfg(test)]
const CHAIN_METHODS: [&str; 31] = [
    "$count",
    "$dynamic",
    "$with",
    "as",
    "crossJoin",
    "delete",
    "except",
    "from",
    "fullJoin",
    "groupBy",
    "having",
    "innerJoin",
    "insert",
    "intersect",
    "leftJoin",
    "limit",
    "offset",
    "onConflictDoNothing",
    "onConflictDoUpdate",
    "orderBy",
    "prepare",
    "returning",
    "rightJoin",
    "select",
    "set",
    "union",
    "unionAll",
    "update",
    "values",
    "where",
    "with",
];

/// Builder methods whose arguments may be laid out one per line.
const FORMAT_METHODS: [&str; 22] = [
    "$count",
    "as",
    "crossJoin",
    "except",
    "findFirst",
    "findMany",
    "fullJoin",
    "groupBy",
    "having",
    "innerJoin",
    "intersect",
    "leftJoin",
    "onConflictDoNothing",
    "onConflictDoUpdate",
    "orderBy",
    "returning",
    "rightJoin",
    "set",
    "union",
    "unionAll",
    "values",
    "where",
];

/// Operator helpers imported from `drizzle-orm`.
const HELPERS: [&str; 27] = [
    "and",
    "arrayContained",
    "arrayContains",
    "arrayOverlaps",
    "asc",
    "between",
    "desc",
    "eq",
    "exists",
    "gt",
    "gte",
    "ilike",
    "inArray",
    "isNotNull",
    "isNull",
    "like",
    "lt",
    "lte",
    "ne",
    "not",
    "notBetween",
    "notExists",
    "notIlike",
    "notInArray",
    "notLike",
    "or",
    "sql",
];

/// Helpers whose arguments are laid out one per line.
const MULTILINE_HELPERS: [&str; 5] = ["and", "or", "not", "exists", "notExists"];

/// Option keys whose object, array, or call values are laid out.
const OBJECT_KEYS: [&str; 11] = ["columns", "extras", "limit", "offset", "onUpdate", "orderBy", "set", "target", "targetWhere", "where", "with"];

/// Set operations imported from `drizzle-orm`.
const SET_OPERATIONS: [&str; 4] = ["except", "intersect", "union", "unionAll"];

pub(crate) fn is_receiver(name: &str) -> bool {
    RECEIVERS.contains(&name)
}

#[cfg(test)]
pub(crate) fn is_chain_method(name: &str) -> bool {
    CHAIN_METHODS.contains(&name)
}

pub(crate) fn is_format_method(name: &str) -> bool {
    FORMAT_METHODS.contains(&name)
}

pub(crate) fn is_helper(name: &str) -> bool {
    HELPERS.contains(&name)
}

pub(crate) fn is_multiline_helper(name: &str) -> bool {
    MULTILINE_HELPERS.contains(&name)
}

pub(crate) fn formats_object_key(name: &str) -> bool {
    OBJECT_KEYS.contains(&name)
}

pub(crate) fn is_set_operation(name: &str) -> bool {
    SET_OPERATIONS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_recognised_drizzle_names() {
        assert!(is_receiver("db"));
        assert!(is_receiver("tx"));
        assert!(!is_receiver("builder"));

        assert!(is_chain_method("select"));
        assert!(is_chain_method("from"));
        assert!(!is_chain_method("findMany"));

        assert!(is_format_method("where"));
        assert!(is_format_method("findMany"));
        assert!(!is_format_method("from"));

        assert!(is_helper("eq"));
        assert!(is_helper("and"));
        assert!(!is_helper("coalesce"));

        assert!(is_multiline_helper("and"));
        assert!(is_multiline_helper("exists"));
        assert!(!is_multiline_helper("eq"));

        assert!(is_set_operation("union"));
        assert!(is_set_operation("unionAll"));
        assert!(!is_set_operation("where"));

        assert!(formats_object_key("with"));
        assert!(formats_object_key("target"));
        assert!(!formats_object_key("id"));
    }
}
