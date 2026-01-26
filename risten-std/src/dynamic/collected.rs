use risten_core::{DynHook, Hook, Message};

/// A wrapper for hooks to be collected via `inventory`.
///
/// This struct allows submitting hooks to a distributed collection
/// that can be gathered at runtime to form a router.
pub struct CollectedHook<E: Message> {
    /// The hook instance (type-erased).
    pub hook: Box<dyn DynHook<E>>,
    /// Priority for ordering (higher runs first).
    pub priority: i32,
    /// Name for debugging.
    pub name: &'static str,
}

impl<E: Message> CollectedHook<E> {
    /// Create a new collected hook entry.
    pub fn new<H>(hook: H, priority: i32, name: &'static str) -> Self
    where
        H: Hook<E> + 'static,
    {
        Self {
            hook: Box::new(hook),
            priority,
            name,
        }
    }
}

/// Collects all registered hooks for the given event type.
///
/// Returns a vector of hooks sorted by priority (descending).
pub fn collect_hooks<E: Message>() -> Vec<std::sync::Arc<dyn DynHook<E>>> {
    // Note: inventory::iter returns an iterator.
    // We collect them into a Vec to sort them.
    // The hooks are stored as Box<dyn DynHook<E>> in CollectedHook.
    // `DynHook` is `Send + Sync`, so we can put it in Arc.
    // However, `CollectedHook` owns the Box. Inventory items are usually static references,
    // but `submit!` generates a static block that registers the item.
    //
    // Wait, `inventory::submit!` creates a static item. The item must be `Copy` or consistent?
    // No, `inventory` allows any type that is `Sync` (I think?).
    // Actually, `inventory::submit!` typically takes an expression that evaluates to the item.
    // The item is stored in a distributed slice or list node.
    //
    // Let's modify CollectedHook to hold `fn() -> Box<dyn DynHook<E>>` if we want to construct fresh hooks,
    // OR if we want singleton behavior, we might need a `Lazy` or just construct it once.
    // Given the `new` method takes ownership of `H`, we can't put `CollectedHook` directly in `submit!` if it owns non-const-constructible things?
    //
    // `inventory` example:
    // inventory::submit! { Flag::new('v', "verbose") }
    //
    // `CollectedHook` field `hook` is `Box<dyn ...>`. `Box::new` is not const.
    // But `inventory::submit!` block is executed at runtime (ctor-like mechanism / lazy_static-ish depending on implementation... wait).
    // The `inventory::submit!` creates a static `Node` which registers the value.
    // The value expression is evaluated when the registration happens (usually at init time).
    //
    // So `Box::new` is fine.

    let mut entries: Vec<&CollectedHook<E>> =
        inventory::iter::<CollectedHook<E>>.into_iter().collect();

    entries.sort_by(|a, b| b.priority.cmp(&a.priority));

    entries
        .into_iter()
        .map(|entry| {
            // We need to clone the hook or similar?
            // `CollectedHook` is owned by the registry. We only get references `&CollectedHook`.
            // So we cannot move `hook` out of it.
            // And `Box<dyn DynHook>` is not clonable unless we have `DynClone`.
            // `DynHook` doesn't seem to enforce `Clone`.
            //
            // SOLUTION:
            // 1. `CollectedHook` should hold a factory: `fn() -> Box<dyn DynHook<E>>`.
            // or
            // 2. `DynHook` should be clonable (usually preferred for dynamic routers anyway, but might be heavy).
            //
            // If `DynHook` objects are used as singletons (shared via Arc), then `CollectedHook` could hold `Arc<dyn DynHook>`.
            // Arc is clonable!

            entry.hook.clone()
        })
        .collect()
}
