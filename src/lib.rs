#![no_std]

//! Single-threaded lazy evaluation.
//!
//! Lazy evaluation allows you to define computations whose
//! evaluation is deferred to when they are actually needed.
//! This can be also achieved with closures; however,
//! in case of lazy evaluation, the output of computations is
//! calculated only once and stored in a cache.
//!
//! Lazy evaluation is useful if you have an expensive computation
//! of which you might need the result more than once during runtime,
//! but you do not know in advance whether you will need it at all.
//!
//! Let us consider an example, where we first use a closure to defer evaluation:
//!
//! ~~~
//! fn expensive() -> i32 {
//!     println!("I am expensive to evaluate!"); 7
//! }
//!
//! fn main() {
//!     let a = || expensive(); // Nothing is printed.
//!
//!     assert_eq!(a(), 7); // "I am expensive to evaluate!" is printed here
//!
//!     let b = [a(), a()]; // "I am expensive to evaluate!" is printed twice
//!     assert_eq!(b, [7, 7]);
//! }
//! ~~~
//!
//! Contrast this with using lazy evaluation:
//!
//! ~~~
//! # use lazy_st::lazy;
//! fn expensive() -> i32 {
//!     println!("I am expensive to evaluate!"); 7
//! }
//!
//! fn main() {
//!     let a = lazy!(expensive()); // Nothing is printed.
//!
//!     // Thunks are just smart pointers!
//!     assert_eq!(*a, 7); // "I am expensive to evaluate!" is printed here
//!
//!     let b = [*a, *a]; // Nothing is printed.
//!     assert_eq!(b, [7, 7]);
//! }
//! ~~~
//!
//! This crate is intended for use in single-threaded contexts.
//! Sharing a lazy value between multiple threads is not supported.

extern crate alloc;

use alloc::boxed::Box;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

use self::Inner::{Evaluating, Unevaluated, Value};

/// A lazily evaluated value.
pub struct Thunk<E, V>(UnsafeCell<Inner<E, V>>);

/// A lazily evaluated value produced from a closure.
pub type Lazy<T> = Thunk<Box<dyn FnOnce() -> T>, T>;

/// Construct a lazily evaluated value using a closure.
///
/// ~~~
/// # use lazy_st::lazy;
/// let val = lazy!(7);
/// assert_eq!(*val, 7);
/// ~~~
#[macro_export]
macro_rules! lazy {
    ($e:expr) => {
        $crate::Thunk::new(Box::new(move || $e))
    };
}

impl<E, V> Thunk<E, V>
where
    E: Evaluate<V>,
{
    /// Create a lazily evaluated value from
    /// a value implementing the `Evaluate` trait.
    ///
    /// The `lazy!` macro is preferred if you want to
    /// construct values from closures.
    ///
    /// ~~~
    /// # use lazy_st::Thunk;
    /// let expensive = Thunk::new(|| { println!("Evaluated!"); 7 });
    /// assert_eq!(*expensive, 7); // "Evaluated!" gets printed here.
    /// assert_eq!(*expensive, 7); // Nothing printed.
    /// ~~~
    pub fn new(e: E) -> Thunk<E, V> {
        Thunk(UnsafeCell::new(Unevaluated(e)))
    }

    /// Create a new, evaluated, thunk from a value.
    ///
    /// ~~~
    /// # use lazy_st::{Thunk, Lazy};
    /// let x: Lazy<u32> = Thunk::evaluated(10);
    /// assert_eq!(*x, 10);
    /// ~~~
    pub fn evaluated(v: V) -> Thunk<E, V> {
        Thunk(UnsafeCell::new(Value(v)))
    }

    /// Force evaluation of a thunk.
    pub fn force(&self) {
        match unsafe { &*self.0.get() } {
            Value(_) => return,
            Evaluating => panic!("Thunk::force called during evaluation."),
            Unevaluated(_) => (),
        }
        unsafe {
            match core::ptr::replace(self.0.get(), Evaluating) {
                Unevaluated(e) => *self.0.get() = Value(e.evaluate()),
                _ => unreachable!(),
            };
        }
    }

    /// Force the evaluation of a thunk and get the value, consuming the thunk.
    ///
    /// ~~~
    /// # use lazy_st::lazy;
    /// let val = lazy!(7);
    /// assert_eq!(val.unwrap(), 7);
    /// ~~~
    pub fn unwrap(self) -> V {
        self.force();
        match self.0.into_inner() {
            Value(v) => v,
            _ => unreachable!(),
        }
    }
}

/// Generalisation of lazy evaluation to other types than closures.
///
/// The main use case for implementing this trait is the following:
/// Let us suppose that you construct a large number of lazy values using
/// only one function `f` with different values `x1`, ..., `xn` of type `T`,
/// i.e. `lazy!(f(x1))`, ..., `lazy!(f(xn))`.
/// In this case, you may consider implementing `Evaluate` for `T` such that
/// `evaluate(x)` yields `f(x)`.
/// This allows you to use `Thunk::new(x)` instead of `lazy!(f(x))`,
/// saving time and space because
/// any such `Thunk` will contain only `x` instead of both `f` and `x`.
///
/// Let us look at an example:
///
/// ~~~
/// # use lazy_st::{Thunk, Evaluate};
/// struct User(usize);
///
/// impl Evaluate<String> for User {
///     fn evaluate(self) -> String {
///         format!("User no. {}", self.0)
///     }
/// }
///
/// let root = Thunk::new(User(0));
/// let mere_mortal = Thunk::evaluated(String::from("Someone else"));
/// let user = if true { root } else { mere_mortal };
/// assert_eq!(*user, "User no. 0");
/// ~~~
///
/// Note that this trait is quite similar to the `Into` trait.
/// Unfortunately, it seems that we cannot use `Into` here,
/// because we cannot implement it for instances of `FnOnce`,
/// which is necessary for `Lazy`.
pub trait Evaluate<T> {
    fn evaluate(self) -> T;
}

impl<A: FnOnce() -> B, B> Evaluate<B> for A {
    fn evaluate(self) -> B {
        self()
    }
}

enum Inner<E, V> {
    Unevaluated(E),
    Evaluating,
    Value(V),
}

impl<E, V> Deref for Thunk<E, V>
where
    E: Evaluate<V>,
{
    type Target = V;

    fn deref(&self) -> &V {
        self.force();
        match unsafe { &*self.0.get() } {
            Value(ref v) => v,
            _ => unreachable!(),
        }
    }
}

impl<E, V> DerefMut for Thunk<E, V>
where
    E: Evaluate<V>,
{
    fn deref_mut(&mut self) -> &mut V {
        self.force();
        match unsafe { &mut *self.0.get() } {
            Value(ref mut v) => v,
            _ => unreachable!(),
        }
    }
}
