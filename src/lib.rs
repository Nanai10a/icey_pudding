#![feature(try_blocks)]
#![feature(drain_filter)]
#![feature(box_syntax)]
#![feature(fn_traits)]
#![feature(label_break_value)]

pub mod conductors;
pub mod entities;
pub mod handlers;
pub mod repositories;

// FIXME: Resultの多用が酷いのでpanic!を検討しましょう
