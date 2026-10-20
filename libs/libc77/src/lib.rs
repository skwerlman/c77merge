pub mod c77mod;
pub mod error;
pub mod hash_mod;
pub mod modlist;
pub mod time;

#[macro_export]
macro_rules! print_header {
    () => {
        let pkg_name: &'static str = env!("CARGO_PKG_NAME");
        let bin_name: &'static str = option_env!("CARGO_BIN_NAME").unwrap_or("");
        let v_maj: &'static str = env!("CARGO_PKG_VERSION_MAJOR");
        let v_min: &'static str = env!("CARGO_PKG_VERSION_MINOR");
        tracing::info!("{pkg_name} {bin_name} v{v_maj}.{v_min}");
    };
}

#[macro_export]
macro_rules! init {
    // inner curly braces stop us polluting the caller
    () => {{
        miette::set_panic_hook();

        let filter = tracing_subscriber::EnvFilter::try_from_env("C77_LOG").unwrap_or_else(|_| {
            EnvFilter::new(
                "debug,h2=info,hyper_util=info,reqwest=info,rustls_platform_verifier=info",
            )
        });

        let subscriber = tracing_subscriber::Registry::default()
            .with(
                tracing_tree::HierarchicalLayer::new(2)
                    .with_indent_lines(true)
                    .with_indent_amount(1)
                    .with_bracketed_fields(true)
                    .with_span_retrace(true)
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_targets(true)
                    .with_writer(std::io::stdout)
                    .with_timer(tracing_tree::time::Uptime::default()),
            )
            .with(filter);
        tracing::subscriber::set_global_default(subscriber).unwrap();

        libc77::print_header!();
    }};
}
