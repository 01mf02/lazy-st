# lazy-st

This crate provides single-threaded lazy evaluation for Rust.
It is an adaptation of the "[lazy]<https://github.com/reem/rust-lazy>" crate,
removing support for multi-threaded operation and
making it compatible with newer Rust versions.

## Example

~~~ rust
fn expensive() -> i32 {
    println!("I am only evaluated once!"); 7
}

fn main() {
    let a = lazy!(expensive());

    // Thunks are just smart pointers!
    assert_eq!(*a, 7); // "I am only evaluated once." is printed here

    let b = [*a, *a]; // Nothing is printed.
    assert_eq!(b, [7, 7]);
}
~~~
