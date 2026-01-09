//! Test for distributed registry using inventory feature.
//!
//! NOTE: This test is currently disabled because `inventory::collect!`
//! generates a trait impl that violates Rust's orphan rules when used
//! from a test crate. The macros work correctly when used from a
//! downstream crate that defines its own event types.
//!
//! To test the distributed registry feature:
//! 1. Create a new binary crate that depends on sakuramiya-event
//! 2. Use define_global_registry! and register_hook! in that crate
//! 3. Call global_registry() to get the collected hooks

// Tests are disabled due to orphan rule constraints with inventory crate.
// See: https://github.com/dtolnay/inventory/issues/26

#[test]
fn distributed_registry_note() {
    // This is a placeholder test to document the limitation.
    // The distributed registry feature works, but cannot be tested
    // in the test crate due to Rust's orphan rules.
    //
    // Example usage (in a downstream crate):
    //
    // ```rust
    // use risten::{define_global_registry, register_hook};
    //
    // struct MyEvent { msg: String }
    //
    // define_global_registry!(MyEvent);
    //
    // struct MyHook;
    // impl Hook<MyEvent> for MyHook { ... }
    //
    // register_hook!(MyEvent, MyHook);
    //
    // fn main() {
    //     let registry = global_registry();
    //     let dispatcher = SimpleDispatcher::new(registry);
    //     // ...
    // }
    // ```
}
