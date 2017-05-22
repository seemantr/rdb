/*!

*/
#![crate_type= "lib"]
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![cfg_attr(feature = "lints", deny(warnings))]

#![deny(missing_docs,
        missing_debug_implementations,
        missing_copy_implementations,
        trivial_casts,
//trivial_numeric_casts, //bitflags fails this lint
//unsafe_code,
//dead_code,
        unused_extern_crates,
        unused_import_braces,
        unused_allocation,
        unused_qualifications)]

#![allow(dead_code)]

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate bitflags;

#[macro_use]
extern crate quick_error;
extern crate memmap;
extern crate fs2;

//mod node;
//mod bucket;
//mod transaction;
mod page;
mod errors;
mod constants;
mod db;
