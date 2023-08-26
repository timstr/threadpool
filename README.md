# threadpool

A basic threadpool that uses work-stealing and allows closures with non-static references.

Currently only `foreach` is implemented. This function takes a mutable slice and closure which mutates an element of the slice, and dispatches this to each thread in the pool on a first-come-first-serve basis. `foreach` returns once all elements in the slice have been visited.

Example usage:

```rust
let mut data: Vec<usize> = (0..65536).collect();

let mut threadpool = ThreadPool::new(1);

let offset: usize = 10;

threadpool.foreach(&mut data, |x| {
    *x = *x + offset; // <---------- 'offset' is borrowed from multiple threads here!
});

assert!(data.iter().enumerate().all(|(i, x)| *x == i + 10));
```

## Is this library safe and sound?

Um, probably? Thread-safe atomics, channels, and barriers are used to handle all synchronization and the included tests run and pass. However, I don't have a rigorous proof that it is guaranteed to be memory safe, and part of the implementation relies on `std::mem::transmute` to cast away the lifetime bound on the closure being passed to other threads. This is a known-workaround for writing threading primitives as mentioned in item 3 of https://github.com/rust-lang/rust/pull/55043. The use of a shared one-off barrier inside of `foreach` serves to ensure that all threads have finished using and dropped the closure before `foreach` returns.
