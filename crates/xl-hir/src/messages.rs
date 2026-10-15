//! Stable lowering-diagnostic message builders.
//!
//! Every message produced by structural lowering is built here. The texts
//! are part of this stage's observable behavior and are pinned exactly by
//! tests. They are new at this stage (`hir-contract.md` §7.2): lexical and
//! syntactic errors were already reported by the parser and are never
//! repeated.

/// `HIR nesting limit ({limit}) exceeded`.
pub(crate) fn depth_limit(limit: u32) -> String {
    format!("HIR nesting limit ({limit}) exceeded")
}

/// `HIR node limit ({limit}) exceeded`.
pub(crate) fn node_limit(limit: u32) -> String {
    format!("HIR node limit ({limit}) exceeded")
}

/// `HIR definition limit ({limit}) exceeded`.
pub(crate) fn def_limit(limit: u32) -> String {
    format!("HIR definition limit ({limit}) exceeded")
}

/// `HIR use limit ({limit}) exceeded`.
pub(crate) fn use_limit(limit: u32) -> String {
    format!("HIR use limit ({limit}) exceeded")
}

/// `'break' outside of a loop`.
pub(crate) fn break_outside_loop() -> String {
    "'break' outside of a loop".to_string()
}

/// `'continue' outside of a loop`.
pub(crate) fn continue_outside_loop() -> String {
    "'continue' outside of a loop".to_string()
}

/// `string interpolation is not allowed in patterns`.
pub(crate) fn interpolation_in_pattern() -> String {
    "string interpolation is not allowed in patterns".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_texts_are_exact() {
        assert_eq!(depth_limit(256), "HIR nesting limit (256) exceeded");
        assert_eq!(node_limit(1_000_000), "HIR node limit (1000000) exceeded");
        assert_eq!(def_limit(100_000), "HIR definition limit (100000) exceeded");
        assert_eq!(use_limit(1_000_000), "HIR use limit (1000000) exceeded");
        assert_eq!(break_outside_loop(), "'break' outside of a loop");
        assert_eq!(continue_outside_loop(), "'continue' outside of a loop");
        assert_eq!(
            interpolation_in_pattern(),
            "string interpolation is not allowed in patterns"
        );
    }
}
