use crate::bind::*;
use crate::exec::*;
use crate::meta::*;
use crate::bitcode::*;

use std::convert::*;

///
/// Function that takes a monad parameter and returns the assembled bitcode
/// 
/// Normally monad parameters to functions are remapped so that the monad's content is used instead of the
/// monad itself, so this requres assistance from syntax (the assemble keyword in this case) to work.
///
fn assemble_fn() -> impl FrameMonad<Binding=RuntimeResult> {
    FnMonad::from(|(monad, ): (CellRef, )| {

        if let Some(monad) = BitCodeMonad::from_cell(&monad) {
            let (result, bitcode)   = assemble(&monad)?;
            let bitcode             = CellRef::new(SafasCell::BitCode(bitcode));

            Ok(SafasCell::list_with_cells(vec![bitcode, result]))
        } else {
            // Only works on bitcode monads
            Err(RuntimeError::NotBitCode(monad))
        }

    })
}

///
/// Keyword that takes a monad and returns a bitcode cell
///
pub fn assemble_keyword() -> impl BindingMonad<Binding=SyntaxCompiler> {
    // Create a copy of the assemble function to use when compiling
    let assemble_fn = CellRef::new(SafasCell::FrameMonad(Box::new(assemble_fn())));

    get_expression_arguments().and_then(|ListTuple((assemble_monad, )): ListTuple<(CellRef, )>| {

        BindingFn::from_binding_fn(move |bindings| {
            // Bind the monad 
            let assemble_monad              = bind_statement(assemble_monad.clone(), bindings);
            let (bindings, assemble_monad)  = match assemble_monad { Ok((monad, bindings)) => (bindings, monad), Err((err, bindings)) => return (bindings, Err(err)) };

            // Monad is the result
            (bindings, Ok(assemble_monad))
        })

    }).map(move |bound_assemble_monad| {

        // Need our own copy of assemble_fn
        let assemble_fn = assemble_fn.clone();

        let compile = move |bound_assemble_monad| {

            // Generate the actions
            let mut assemble_monad = CompiledActions::empty();

            // Function to call to do the assembly
            assemble_monad.push(Action::PushValue(assemble_fn.clone()));

            // Generate the monad value
            assemble_monad.extend(compile_statement(bound_assemble_monad)?);

            // Call the function
            assemble_monad.push(Action::Push);
            assemble_monad.push(Action::PopCall(1));

            Ok(assemble_monad)

        };

        SyntaxCompiler::with_compiler(compile, bound_assemble_monad)
    })
}

#[cfg(test)]
mod test {
    use crate::interactive::*;

    #[test]
    fn assemble_simple() {
        let val = eval("
            (car (assemble (d $02u8)))
        ").unwrap().to_string();
        assert!(val == "00000000: 02                                  | .".to_string());
    }
}
