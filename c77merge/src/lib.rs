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
