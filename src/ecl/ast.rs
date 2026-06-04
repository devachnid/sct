//! Abstract syntax tree for the supported subset of SNOMED CT Expression
//! Constraint Language (ECL). See `specs/ecl.md` §5.

/// A focus operator applied to a sub-expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// `<` - descendants (not self)
    DescendantOf,
    /// `<<` - descendants or self
    DescendantOrSelfOf,
    /// `>` - ancestors (not self)
    AncestorOf,
    /// `>>` - ancestors or self
    AncestorOrSelfOf,
    /// `<!` - direct children
    ChildOf,
    /// `>!` - direct parents
    ParentOf,
    /// `^` - members of the refset
    MemberOf,
}

/// Boolean combination of two expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    /// `AND` - intersection
    And,
    /// `OR` - union
    Or,
    /// `MINUS` - set difference
    Minus,
}

/// An ECL expression constraint.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// `*` - any concept.
    Wildcard,
    /// A concept reference by SCTID. Any `|term|` annotation is parsed and
    /// dropped (it is a human label, not part of the semantics).
    Concept(String),
    /// A focus operator applied to a sub-expression.
    Op(Op, Box<Expr>),
    /// A boolean combination.
    Bool(BoolOp, Box<Expr>, Box<Expr>),
    /// A focus expression refined by attribute constraints (`focus : refinement`).
    Refined(Box<Expr>, Refinement),
}

/// The attribute-constraint portion after a `:`.
#[derive(Debug, Clone, PartialEq)]
pub enum Refinement {
    /// A single attribute constraint: `attr = value` (or `!=` when `negate`).
    Attr {
        /// Attribute *type* expression (usually a concept, e.g. `363698007`).
        attr: Box<Expr>,
        /// `true` for `!=`, `false` for `=`.
        negate: bool,
        /// Attribute *value* expression (e.g. `<<80891009`, `*`).
        value: Box<Expr>,
    },
    /// Conjunction (comma or `AND`).
    And(Box<Refinement>, Box<Refinement>),
    /// Disjunction (`OR`).
    Or(Box<Refinement>, Box<Refinement>),
    /// An attribute group `{ … }`. Evaluated as a flat conjunction in v1
    /// (group cardinality is deferred - see `specs/ecl.md` §5).
    Group(Box<Refinement>),
}
