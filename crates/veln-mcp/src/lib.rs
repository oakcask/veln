//! Model Context Protocol access to saved Veln workspaces.

mod check_project;
mod definition;
mod schema;
mod server;
mod workspace;

use std::env;
use std::io;

/// Runs the MCP server over standard input and standard output.
pub fn run_stdio() -> io::Result<()> {
    let base = env::current_dir()?.canonicalize()?;
    server::run(base, io::stdin().lock(), io::stdout().lock())
}
