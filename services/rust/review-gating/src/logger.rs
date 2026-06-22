pub fn init_logger() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .log_internal_errors(false)
        .with_level(true)
        .init();
}
