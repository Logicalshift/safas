use crate::bind::*;

///
/// The `label` keyword creates a bitcode monad that specifies a label
/// 
/// Label values are available everywhere in the same context (and may be passed outside 
/// of that context as separate values if necessary): note that 'forward declaration' of
/// labels are specifically allowed via the pre-binding mechanism.
///
pub fn label_keyword() -> SyntaxCompiler {
    unimplemented!()
}
