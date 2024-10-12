mod vector_type;

::pgrx::pg_module_magic!();

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::pg_test;
    use pgrx::Spi;

    #[pg_test]
    fn extension_exists() {
        Spi::get_one::<bool>("SELECT count(*) = 1 FROM pg_extension WHERE extname = 'pg_ivfflat'")
            .unwrap()
            .unwrap();
    }
}
