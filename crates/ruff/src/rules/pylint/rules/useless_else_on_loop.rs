use rustpython_parser::ast::{self, Excepthandler, MatchCase, Stmt};

use ruff_diagnostics::{Diagnostic, Violation};
use ruff_macros::{derive_message_formats, violation};
use ruff_python_ast::helpers;

use crate::checkers::ast::Checker;

/// ## What it does
/// Checks for `else` clauses on loops without a `break` statement.
///
/// ## Why is this bad?
/// When a loop includes an `else` statement, the code inside the `else` clause
/// will be executed if the loop terminates "normally" (i.e., without a
/// `break`).
///
/// If a loop _always_ terminates "normally" (i.e., does _not_ contain a
/// `break`), then the `else` clause is redundant, as the code inside the
/// `else` clause will always be executed.
///
/// In such cases, the code inside the `else` clause can be moved outside the
/// loop entirely, and the `else` clause can be removed.
///
/// ## Example
/// ```python
/// for item in items:
///     print(item)
/// else:
///     print("All items printed")
/// ```
///
/// Use instead:
/// ```python
/// for item in items:
///     print(item)
/// print("All items printed")
/// ```
///
/// ## References
/// - [Python documentation](https://docs.python.org/3/tutorial/controlflow.html#break-and-continue-statements-and-else-clauses-on-loops)
#[violation]
pub struct UselessElseOnLoop;

impl Violation for UselessElseOnLoop {
    #[derive_message_formats]
    fn message(&self) -> String {
        format!(
            "`else` clause on loop without a `break` statement; remove the `else` and de-indent all the \
             code inside it"
        )
    }
}

fn loop_exits_early(body: &[Stmt]) -> bool {
    body.iter().any(|stmt| match stmt {
        Stmt::If(ast::StmtIf { body, orelse, .. }) => {
            loop_exits_early(body) || loop_exits_early(orelse)
        }
        Stmt::With(ast::StmtWith { body, .. })
        | Stmt::AsyncWith(ast::StmtAsyncWith { body, .. }) => loop_exits_early(body),
        Stmt::Match(ast::StmtMatch { cases, .. }) => cases
            .iter()
            .any(|MatchCase { body, .. }| loop_exits_early(body)),
        Stmt::Try(ast::StmtTry {
            body,
            handlers,
            orelse,
            finalbody,
            ..
        })
        | Stmt::TryStar(ast::StmtTryStar {
            body,
            handlers,
            orelse,
            finalbody,
            ..
        }) => {
            loop_exits_early(body)
                || loop_exits_early(orelse)
                || loop_exits_early(finalbody)
                || handlers.iter().any(|handler| match handler {
                    Excepthandler::ExceptHandler(ast::ExcepthandlerExceptHandler {
                        body, ..
                    }) => loop_exits_early(body),
                })
        }
        Stmt::For(ast::StmtFor { orelse, .. })
        | Stmt::AsyncFor(ast::StmtAsyncFor { orelse, .. })
        | Stmt::While(ast::StmtWhile { orelse, .. }) => loop_exits_early(orelse),
        Stmt::Break(_) => true,
        _ => false,
    })
}

/// PLW0120
pub(crate) fn useless_else_on_loop(
    checker: &mut Checker,
    stmt: &Stmt,
    body: &[Stmt],
    orelse: &[Stmt],
) {
    if !orelse.is_empty() && !loop_exits_early(body) {
        checker.diagnostics.push(Diagnostic::new(
            UselessElseOnLoop,
            helpers::else_range(stmt, checker.locator).unwrap(),
        ));
    }
}
