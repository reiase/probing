#![feature(thread_local)]
#![feature(iter_map_windows)]

#[macro_use]
extern crate ctor;

mod pkg;

pub mod extensions;
pub mod features;
pub mod pycode;
pub mod python;
pub mod repl;

mod setup;
