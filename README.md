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
