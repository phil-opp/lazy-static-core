/*!
A macro for declaring lazily evaluated statics that doesn't depend on the standard library.

Using this macro, it is possible to have `static`s that require code to be
executed at runtime in order to be initialized.
This includes anything requiring heap allocations, like vectors or hash maps,
as well as anything that requires function calls to be computed.

# Syntax

```ignore
lazy_static_core! {
    [pub] static ref NAME_1: TYPE_1 = EXPR_1;
    [pub] static ref NAME_2: TYPE_2 = EXPR_2;
    ...
    [pub] static ref NAME_N: TYPE_N = EXPR_N;
}
```

# Semantic

For a given `static ref NAME: TYPE = EXPR;`, the macro generates a
unique type that implements `Deref<TYPE>` and stores it in a static with name `NAME`.

On first deref, `EXPR` gets evaluated and stored internally, such that all further derefs
can return a reference to the same object.

Like regular `static mut`s, this macro only works for types that fulfill the `Sync`
trait.

# Example

Using the macro:

```rust
#![feature(phase)]

extern crate core; //required
#[phase(plugin)]
extern crate lazy_static_core;

use std::collections::HashMap;

lazy_static_core! {
    static ref HASHMAP: HashMap<uint, &'static str> = {
        let mut m = HashMap::new();
        m.insert(0u, "foo");
        m.insert(1u, "bar");
        m.insert(2u, "baz");
        m
    };
    static ref COUNT: uint = HASHMAP.len();
    static ref NUMBER: uint = times_two(21);
}

fn times_two(n: uint) -> uint { n * 2 }

fn main() {
    println!("The map has {} entries.", *COUNT);
    println!("The entry for `0` is \"{}\".", HASHMAP.get(&0).unwrap());
    println!("A expensive calculation on a static results in: {}.", *NUMBER);
}
```

# Implementation details

The `Deref` implementation uses a hidden `static mut` that is guarded by a atomic check
using `core::atomic::AtomicBool`. All lazily evaluated values are currently
put in a heap allocated box, due to the Rust language currently not providing any way to
define uninitialized `static mut` values.

*/

#![no_std]

#![crate_type = "dylib"]

#![feature(macro_rules)]

#[cfg(test)] extern crate std;

#[macro_export]
macro_rules! lazy_static_core {
    (static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        lazy_static_core!(PRIV static ref $N : $T = $e; $($t)*);
    };
    (pub static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        lazy_static_core!(PUB static ref $N : $T = $e; $($t)*);
    };
    ($VIS:ident static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        lazy_static_core!(MAKE TY $VIS $N);
        impl ::core::ops::Deref<$T> for $N {
            fn deref<'a>(&'a self) -> &'a $T {
                use core::mem::transmute;
                use core::atomic::{AtomicBool, INIT_ATOMIC_BOOL, Ordering};
                use core::kinds::Sync;

                #[inline(always)]
                fn require_sync<T: Sync>(_: &T) { }
                
                static mut data: *const $T = 0 as *const $T;
                static INITIALIZED: AtomicBool = INIT_ATOMIC_BOOL;

                if INITIALIZED.compare_and_swap(false, true, Ordering::SeqCst) == false {
                    unsafe{data = transmute::<Box<$T>, *const $T>(box() ($e))};
                }

                let static_ref = unsafe {&*data};
                require_sync(static_ref);
                static_ref                
            }
        }
        lazy_static_core!($($t)*);
    };
    (MAKE TY PUB $N:ident) => {
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        pub struct $N {__private_field: ()}
        #[allow(dead_code)]
        pub static $N: $N = $N {__private_field: ()};
    };
    (MAKE TY PRIV $N:ident) => {
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        struct $N {__private_field: ()}
        #[allow(dead_code)]
        static $N: $N = $N {__private_field: ()};
    };
    () => ()
}
