use crate::ast::SecretSource;
use crate::parser::parse;

#[test]
fn secrets_block_vault() {
    let f = parse(
        r#"
        secrets {
            api_key: vault("secret/rein/key")
        }
    "#,
    )
    .unwrap();
    assert_eq!(f.secrets.len(), 1);
    let binding = &f.secrets[0].bindings[0];
    assert_eq!(binding.name, "api_key");
    assert!(matches!(
        &binding.source,
        SecretSource::Vault { path } if path == "secret/rein/key"
    ));
}

#[test]
fn secrets_block_multiple_bindings() {
    let f = parse(
        r#"
        secrets {
            api_key: vault("secret/rein/key")
            db_password: vault("secret/rein/db")
        }
    "#,
    )
    .unwrap();
    assert_eq!(f.secrets[0].bindings.len(), 2);
}
