use super::*;
use crate::ast::{ChannelDef, Span};

fn span() -> Span {
    Span { start: 0, end: 0 }
}

fn make_def() -> ChannelDef {
    ChannelDef {
        name: "price_updates".to_string(),
        message_type: Some("PriceChange".to_string()),
        retention: Some("7 days".to_string()),
        span: span(),
    }
}

#[test]
fn creates_from_def() {
    let ch = Channel::from_def(&make_def());
    assert_eq!(ch.name(), "price_updates");
    assert_eq!(ch.message_type(), Some("PriceChange"));
    assert_eq!(ch.retention(), Some("7 days"));
    assert!(ch.is_empty());
}

#[test]
fn publish_and_consume() {
    let mut ch = Channel::from_def(&make_def());
    ch.publish("agent_a", "BTC: $50000");
    ch.publish("agent_b", "ETH: $3000");
    assert_eq!(ch.len(), 2);

    let msg = ch.consume().unwrap();
    assert_eq!(msg.sender, "agent_a");
    assert_eq!(msg.payload, "BTC: $50000");

    let msg = ch.consume().unwrap();
    assert_eq!(msg.sender, "agent_b");
    assert_eq!(ch.len(), 0);
}

#[test]
fn consume_empty_returns_none() {
    let mut ch = Channel::from_def(&make_def());
    assert!(ch.consume().is_none());
}

#[test]
fn peek_does_not_consume() {
    let mut ch = Channel::from_def(&make_def());
    ch.publish("agent", "msg");
    assert!(ch.peek().is_some());
    assert_eq!(ch.len(), 1);
}

#[test]
fn fifo_ordering() {
    let mut ch = Channel::from_def(&make_def());
    ch.publish("a", "first");
    ch.publish("b", "second");
    ch.publish("c", "third");
    assert_eq!(ch.consume().unwrap().payload, "first");
    assert_eq!(ch.consume().unwrap().payload, "second");
    assert_eq!(ch.consume().unwrap().payload, "third");
}
