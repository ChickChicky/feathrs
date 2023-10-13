#![allow(unused_braces)]

use std::time::Duration;

use crossterm::{
    event,
    terminal,
    tty
};

fn main() {

    'main: loop {

        while event::poll(Duration::from_millis(10)).unwrap_or(false) {

            let ev = event::read().unwrap();

            
            
        }

    }

}
