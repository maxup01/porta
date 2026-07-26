use super::*;

/// `Display` must render the payload, not the format string.
///
/// Regression: the variant was annotated `#[error("0")]` rather than
/// `#[error("{0}")]`, so every error rendered as the literal digit and the
/// message was discarded.
#[test]
fn invalid_data_display_renders_the_payload() {
    let error = Error::InvalidData("Passed string that doesn't match any http method");

    assert_eq!(
        error.to_string(),
        "Passed string that doesn't match any http method"
    );
}

/// Errors cross `tokio::spawn` boundaries in the dispatch path, so the type has
/// to stay thread-safe as variants are added. Fails at compile time, not run time.
#[test]
fn error_is_send_sync_and_static() {
    fn assert_send_sync_static<T: Send + Sync + 'static>() {}

    assert_send_sync_static::<Error>();
}
