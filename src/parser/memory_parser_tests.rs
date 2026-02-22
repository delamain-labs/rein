use crate::ast::MemoryTier;
use crate::parser::parse;

#[test]
fn memory_block_three_tiers() {
    let f = parse(
        r#"
        memory {
            working {
                ttl: "30m"
                max_entries: 100
            }
            session {
                ttl: "24h"
                backend: "redis"
            }
            knowledge {
                backend: "sqlite"
            }
        }
    "#,
    )
    .unwrap();
    assert_eq!(f.memories.len(), 1);
    let mem = &f.memories[0];
    assert_eq!(mem.tiers.len(), 3);

    assert_eq!(mem.tiers[0].tier, MemoryTier::Working);
    assert_eq!(mem.tiers[0].ttl.as_deref(), Some("30m"));
    assert_eq!(mem.tiers[0].max_entries, Some(100));

    assert_eq!(mem.tiers[1].tier, MemoryTier::Session);
    assert_eq!(mem.tiers[1].ttl.as_deref(), Some("24h"));
    assert_eq!(mem.tiers[1].backend.as_deref(), Some("redis"));

    assert_eq!(mem.tiers[2].tier, MemoryTier::Knowledge);
    assert!(mem.tiers[2].ttl.is_none());
    assert_eq!(mem.tiers[2].backend.as_deref(), Some("sqlite"));
}

#[test]
fn memory_block_with_name() {
    let f = parse(
        r#"
        memory agent_memory {
            working {
                ttl: "10m"
            }
        }
    "#,
    )
    .unwrap();
    assert_eq!(f.memories[0].name.as_deref(), Some("agent_memory"));
}

#[test]
fn memory_block_empty_tier() {
    let f = parse(
        r#"
        memory {
            session {}
        }
    "#,
    )
    .unwrap();
    let tier = &f.memories[0].tiers[0];
    assert_eq!(tier.tier, MemoryTier::Session);
    assert!(tier.ttl.is_none());
    assert!(tier.max_entries.is_none());
    assert!(tier.backend.is_none());
}
