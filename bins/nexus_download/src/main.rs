use miette::Result;
use tracing::trace_span;
use tracing_subscriber::{EnvFilter, prelude::*};

fn main() -> Result<()> {
    libc77::init!();

    let main_span = trace_span!("main").entered();

    main_span.exit();
    Ok(())
}
