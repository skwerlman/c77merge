pub mod c77mod;
pub mod error;
pub mod hash_mod;
pub mod modlist;
pub mod time;

#[macro_export]
macro_rules! header {
    () => {
        let pkg_name = env!("CARGO_PKG_NAME");
        let bin_name = option_env!("CARGO_BIN_NAME").unwrap_or("");
        let v_maj = env!("CARGO_PKG_VERSION_MAJOR");
        let v_min = env!("CARGO_PKG_VERSION_MINOR");

        tracing::info!("{pkg_name} {bin_name} v{v_maj}.{v_min}");
    };
}

#[macro_export]
macro_rules! init {
    () => {
        human_panic::setup_panic!();
        let format = tracing_subscriber::fmt::format::format().compact();
        let filter = tracing_subscriber::EnvFilter::try_from_env("C77_LOG")
            .unwrap_or_else(|_| EnvFilter::new("debug"));

        fmt()
            .event_format(format)
            .with_env_filter(filter)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .finish()
            .init();

        c77merge::header!();
    };
}
