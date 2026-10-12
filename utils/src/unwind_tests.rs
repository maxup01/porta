use super::*;
use std::sync::Arc;

#[tokio::test]
async fn a_future_that_finishes_returns_its_value() {
    let answered = catch(Box::pin(async { 7u32 })).await;

    assert_eq!(answered.unwrap(), 7);
}

#[tokio::test]
async fn a_panic_before_the_first_await_is_caught() {
    let panicked = catch(Box::pin(async { panic!("boom") })).await;

    assert!(panicked.is_err());
}

#[tokio::test]
async fn a_panic_after_an_await_is_caught() {
    let panicked = catch(Box::pin(async {
        tokio::task::yield_now().await;
        panic!("boom");
    }))
    .await;

    assert!(panicked.is_err());
}

#[tokio::test]
async fn the_payload_carries_the_panic_argument() {
    let panicked = catch(Box::pin(async { panic!("no such ticket") })).await;
    let payload = panicked.err().expect("should have panicked");

    assert_eq!(payload.downcast_ref::<&str>(), Some(&"no such ticket"));
}

#[tokio::test]
async fn a_formatted_panic_arrives_as_a_string() {
    let id = 42;
    let panicked = catch(Box::pin(async move { panic!("{id} is gone") })).await;
    let payload = panicked.err().expect("should have panicked");

    assert!(payload.downcast_ref::<&str>().is_none());
    assert_eq!(
        payload.downcast_ref::<String>().map(String::as_str),
        Some("42 is gone")
    );
}
