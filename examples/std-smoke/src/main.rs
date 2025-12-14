#![cfg_attr(target_os = "none", no_std)]
#![no_main]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

cfg_if::cfg_if! {
    if #[cfg(target_os = "none")] {
        use platform::println;
    } else {
        use std::println;
    }
}

fn alloc_smoke() -> bool {
    // Minimal deterministic heap smoke: Vec + Box + String.
    let v: Vec<u8> = vec![1, 2, 3];

    let b = Box::new(0xdead_beef_u32);

    let mut s = String::new();
    s.push_str("zeroos");

    v == [1, 2, 3] && *b == 0xdead_beef_u32 && s == "zeroos"
}

fn parallel_sum_of_squares(pool: &rayon::ThreadPool, n: u32) -> u64 {
    use rayon::iter::{IntoParallelIterator, ParallelIterator};
    pool.install(|| {
        (1..=n)
            .into_par_iter()
            .map(|x| (x as u64) * (x as u64))
            .sum()
    })
}

fn thread_smoke() -> bool {
    let pool = match rayon::ThreadPoolBuilder::new().build() {
        Ok(p) => p,
        Err(_) => return false,
    };

    let n = 101;
    let result = parallel_sum_of_squares(&pool, n);
    println!("smoke:thread: result={}", result);
    result == 348551
}

#[no_mangle]
fn main() -> ! {
    if !alloc_smoke() {
        println!("smoke:alloc: failed");
        platform::exit(1)
    }
    println!("smoke:alloc: ok");

    if !thread_smoke() {
        println!("smoke:thread: failed");
        platform::exit(1)
    }
    println!("smoke:thread: ok");

    platform::exit(0)
}
