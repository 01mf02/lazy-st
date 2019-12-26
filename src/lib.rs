//! Single-threaded lazy evaluation.
//!
//! ~~~
//! # use lazy_st::lazy;
//! fn expensive() -> i32 {
//!     println!("I am only evaluated once!"); 7
//! }
//!
//! fn main() {
//!     let a = lazy!(expensive());
//!
//!     // Thunks are just smart pointers!
//!     assert_eq!(*a, 7); // "I am only evaluated once." is printed here
//!
//!     let b = [*a, *a]; // Nothing is printed.
//!     assert_eq!(b, [7, 7]);
//! }
//! ~~~

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};

use self::Inner::{Evaluating, Function, Value};

/// Construct a lazily evaluated value.
///
/// ~~~
/// # use lazy_st::lazy;
/// let val = lazy!(7);
/// assert_eq!(*val, 7);
/// ~~~
#[macro_export]
macro_rules! lazy {
    ($e:expr) => {
        $crate::Thunk::new(move || $e)
    };
}

/// A lazily evaluated value.
pub struct Thunk<'a, T>(UnsafeCell<Inner<'a, T>>);

/// An alternative name for a lazily evaluated value.
pub type Lazy<'a, T> = Thunk<'a, T>;

impl<'a, T> Thunk<'a, T> {
    /// Create a lazily evaluated value from a function that returns that value.
    ///
    /// You can construct `Thunk`s manually using this,
    /// but the `lazy!` macro is preferred.
    ///
    /// ~~~
    /// # use lazy_st::Thunk;
    /// let expensive = Thunk::new(|| { println!("Evaluated!"); 7 });
    /// assert_eq!(*expensive, 7); // "Evaluated!" gets printed here.
    /// assert_eq!(*expensive, 7); // Nothing printed.
    /// ~~~
    pub fn new<F>(f: F) -> Thunk<'a, T>
    where
        F: 'a + FnOnce() -> T,
    {
        Thunk(UnsafeCell::new(Function(Box::new(f))))
    }

    /// Create a new, evaluated, thunk from a value.
    ///
    /// ~~~
    /// # use lazy_st::Thunk;
    /// let x = Thunk::evaluated(10);
    /// assert_eq!(*x, 10);
    /// ~~~
    pub fn evaluated<'b>(v: T) -> Thunk<'b, T> {
        Thunk(UnsafeCell::new(Value(v)))
    }

    /// Force evaluation of a thunk.
    pub fn force(&self) {
        match unsafe { &*self.0.get() } {
            Value(_) => return,
            Evaluating => {
                panic!("Thunk::force called recursively. (A Thunk tried to force itself while trying to force itself).")
            },
            Function(_) => ()
        }
        unsafe {
            match std::ptr::replace(self.0.get(), Evaluating) {
                Function(f) => *self.0.get() = Value(f()),
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
    pub fn unwrap(self) -> T {
        self.force();
        match self.0.into_inner() {
            Value(v) => v,
            _ => unreachable!(),
        }
    }
}

enum Inner<'a, T> {
    Value(T),
    Evaluating,
    Function(Box<dyn FnOnce() -> T + 'a>),
}

impl<'x, T> Deref for Thunk<'x, T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.force();
        match unsafe { &*self.0.get() } {
            Value(ref v) => v,
            _ => unreachable!(),
        }
    }
}

impl<'x, T> DerefMut for Thunk<'x, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.force();
        match unsafe { &mut *self.0.get() } {
            Value(ref mut v) => v,
            _ => unreachable!(),
        }
    }
}
