#![allow(unused)]

use std::{
    thread::{
        spawn,
        Thread
    },
    sync::{
        Arc,
        Mutex
    }, 
    net::{
        TcpListener, 
        TcpStream
    },
    process::{
        exit
    }
};

use console::{
    Term,
    Key::{ self,
        Char,
        UnknownEscSeq
    }
};

use ctrlc;

fn handle_conn( ctx: Context, stream: TcpStream ) {



}

#[derive(Clone)]
struct Context {
    //interface: Arc<Mutex<Vec<Panel>>>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            // interface: Arc::new(Mutex::new(vec![]))
        }
    }

    /*pub fn get_interface(&self) -> Vec<Panel> {
        return self.interface.lock().unwrap().clone();
    }
    pub fn set_interface(&self, setter: Box<dyn Fn(&mut Vec<Panel>)->()>) {
        setter(&mut *self.interface.lock().unwrap());
    }*/
}

fn main() {

    ctrlc::set_handler(||{
        print!("\x1b[?1003l");
        exit(0);
    });

    let mut ctx: Context = Context::new();

    /*ctx.set_interface(Box::new(|interfaces|{
        interfaces.push(Panel::new());
        interfaces.push(Panel::new());
    }));*/

    // Language server thread

    let server_ctx: Context = ctx.clone();
    spawn(move||{
        let mut ctx: Context = server_ctx;

        let server: TcpListener = TcpListener::bind("0.0.0.0:1234").expect("Could not init language server.");

        for stream in server.incoming() {
            let listener_ctx = ctx.clone();
            spawn(move||{ handle_conn(listener_ctx, stream.unwrap()); });
        }
    });

    // Input thread

    let input_ctx: Context = ctx.clone();
    spawn(move||{

        let mut ctx: Context = input_ctx;

        let term: Term = Term::stdout();

        let mut buf_escseq: String = String::new();

        fn handle_mouse(v:String) {
            let parts: Vec<u32> = v[1..].split(";").map(|v|v.parse::<u32>().unwrap()).collect::<Vec<u32>>();
            if parts.len() == 3 {
                let ev: u32 = parts[0]; let ev_type: u32 = (ev>>5)&2;
                // Modifiers:
                //  1 : alt
                //  2 : ctrl
                // Button states:
                //  0 : left click
                //  1 : middle-click
                //  2 : right click
                //  3 : none
                if ev_type == 0 { // ???

                } else if ev_type == 1 { // Click
                    let button_state: u32 = ev&2;
                    let modifier: u32 = (ev>>3)&2;
                } else if ev_type == 2 { // Move
                    let button_state: u32 = ev&2;
                    let modifier: u32 = (ev>>3)&2;
                } else if ev_type == 3 { // Scroll
                    let dir: u32 = ev&1; // 0 -> up / 1 -> down
                }
            }
            println!("MOUSE EV `{:032b}`",parts[0]);
        }

        println!("\x1b[?1003h\x1b[?1015h");

        'input_loop: loop {

            if let Ok(key) = term.read_key() {
                match key {
                    Char(x) => {
                        if buf_escseq.len() > 0 {
                            if x == 'M' {
                                handle_mouse(buf_escseq.clone());
                                buf_escseq.clear();
                            } else {
                                buf_escseq.push(x);
                            }
                        } else {
                            println!("KEY {}",x);
                        }
                    },

                    UnknownEscSeq(chars) => {
                        let seq = String::from_utf8(chars.iter().map(|v|*v as u8).collect()).unwrap();
                        if seq[1..].parse::<u32>().is_ok() {
                            buf_escseq = seq;
                        } else {
                            println!("ESCSEQ {:?}",seq);
                        }
                    },

                    _ => {
                        println!("Unknown {:?}",key);
                    }
                };
            } else {
                break 'input_loop;
            }

        }
    });

    loop {}

}
