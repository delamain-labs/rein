# Plan: Issue #14 — Replace f64 with u64 cents for monetary amounts

## Problem
`Budget.amount` and `Constraint::MonetaryCap.amount` use `f64`, which has
floating-point precision issues (e.g. `0.03 * 100 == 2.9999...`).

## Fix: Store cents as `u64`

`$0.03` → 3 cents, `$50` → 5000 cents. No floats anywhere in the value chain.

---

## Types That Change

| Location | Field | Before | After |
|---|---|---|---|
| `ast.rs` | `Constraint::MonetaryCap.amount` | `f64` | `u64` |
| `ast.rs` | `Budget.amount` | `f64` | `u64` |
| `lexer.rs` | `TokenKind::Dollar(…)` | `f64` | `u64` |
| `parser.rs` | `expect_dollar` return | `(f64, Span)` | `(u64, Span)` |

---

## Conversion Logic (lexer `read_dollar`)

Parse the raw digit string without using `f64`:
1. Split on `.`
2. Parse whole part (dollars) → multiply by 100
3. Parse fractional part: take at most 2 digits, right-pad with `0` to 2 digits
4. Return `whole * 100 + cents`

Examples:
- `"50"`   → whole=50, frac="" → 50*100 + 0 = 5000
- `"0.03"` → whole=0,  frac="03" → 0*100 + 3 = 3
- `"1.5"`  → whole=1,  frac="5" → padded "50" → 1*100 + 50 = 150

---

## Validator Changes

`check_budget_positive`: `budget.amount <= 0.0` → `budget.amount == 0`
(u64 can never be negative, so only zero is invalid)

The `negative_budget_detected` test constructs AST directly with `amount: -5.0`.
Since u64 can't be negative, replace that test with a comment-only change
noting u64 guarantees non-negative. Remove the test.

---

## Tests to Update

### `lexer.rs` tests
- `tokenize_dollar_amount`: `Dollar(0.03)` → `Dollar(3)`
- `tokenize_dollar_integer`: `Dollar(50.0)` → `Dollar(5000)`
- `tokenize_up_to_constraint`: `Dollar(50.0)` → `Dollar(5000)`
- `tokenize_full_agent_snippet`: `Dollar(0.03)` → `Dollar(3)`

### `ast.rs` tests
- `constraint_monetary_cap_serializes`: `amount: 50.0` → `amount: 5000`, JSON 50.0 → 5000
- `capability_with_constraint_serializes`: `amount: 50.0` → `5000`, JSON check 5000
- `budget_serializes`: `amount: 0.03` → `amount: 3`, JSON 0.03 → 3
- `agent_def_full_serializes`: budget `amount: 0.03` → `amount: 3`

### `parser.rs` tests
- `parse_up_to_constraint`: `assert_eq!(*amount, 50.0)` → `5000u64`
- `parse_budget`: `assert_eq!(b.amount, 0.03)` → `3u64`

### `validator.rs` tests
- `zero_budget_detected`: `amount: 0.0` → `amount: 0`
- Remove `negative_budget_detected` (u64 can't represent negative)

---

## Edge Cases
- Sub-cent amounts (e.g. `$0.005`) → truncate to 2 decimal digits → 0 cents
- Integer dollar amounts (e.g. `$50`) → treated as `$50.00` = 5000 cents
- E003 validator still catches zero budget

---

## Order of Execution (TDD)
1. Update tests first in each file to the new `u64` expected values
2. Update `TokenKind::Dollar` to `u64` and `read_dollar` conversion in `lexer.rs`
3. Update `ast.rs` type fields
4. Update `parser.rs` `expect_dollar` signature
5. Update `validator.rs` check and remove `negative_budget_detected` test
6. `cargo test` — all green
7. `cargo clippy` — no warnings
8. `cargo fmt` — clean
9. Commit
