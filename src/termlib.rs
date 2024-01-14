use std::{
    sync::{Arc,Mutex},
    thread::spawn,
    io::{stdout, Write},
};
use console;

#[derive(Debug,Clone,Copy)]
/// All mouse events, they all have:
/// * 'x' and 'y' telling where the event happened
/// * 'data' the payload of the event, 
/// * 'state' the raw payload
pub enum MouseEvent {
    Scroll  {x:i32,y:i32,state:i32,data:i32}, // Some values originally were u8/i8/u32
    Click   {x:i32,y:i32,state:i32,data:i32}, // but I decided to set them all to i32
    Move    {x:i32,y:i32,state:i32,data:i32}, // to unify it all, but I may revert it
    Unknown {x:i32,y:i32,state:i32,data:i32}, // back at some point
}

#[derive(Debug,Clone,Copy)]
pub enum TermEvent {
    Char{char:char},
    Mouse{event:MouseEvent},
    Arrow{x:i32,y:i32,modifiers:i32},
    Enter{},
    Backspace{},
}

#[derive(Clone)]
pub struct Term {
    events: Arc<Mutex<Vec<TermEvent>>>,
    flags: Arc<Mutex<u64>>,
}

#[allow(dead_code)]
impl Term {

    pub fn new() -> Self {
        Self{
            events: Arc::new(Mutex::new(vec![])),
            flags: Arc::new(Mutex::new(0))
        }
    }

    pub fn init(&mut self) {

        let t = self.clone();

        spawn(move||{
    
            let term: console::Term = console::Term::stdout();
    
            let mut buf_escseq: String = String::new();
    
            'input_loop: loop {
    
                if let Ok(key) = term.read_key() {
                    match key {
                        console::Key::Char(x) => {
                            if buf_escseq.len() > 0 {
                                buf_escseq.push(x);
                            } else {
                                t.events.lock().unwrap().push(
                                    TermEvent::Char{char:x}
                                );
                            }
                        },
    
                        console::Key::UnknownEscSeq(chars) => {
                            let seq = String::from_utf8(chars.iter().map(|v|*v as u8).collect()).unwrap();
                            buf_escseq = seq;
                        },

                        console::Key::Enter => {
                            t.events.lock().unwrap().push(
                                TermEvent::Enter{}
                            );
                        },

                        console::Key::ArrowUp => {
                            t.events.lock().unwrap().push(
                                TermEvent::Arrow { 
                                    x: 0, y: -1, 
                                    modifiers: 0
                                }
                            );
                        },

                        console::Key::ArrowDown => {
                            t.events.lock().unwrap().push(
                                TermEvent::Arrow { 
                                    x: 0, y: 1, 
                                    modifiers: 0
                                }
                            );
                        },

                        console::Key::ArrowLeft => {
                            t.events.lock().unwrap().push(
                                TermEvent::Arrow { 
                                    x: -1, y: 0, 
                                    modifiers: 0
                                }
                            );
                        },

                        console::Key::ArrowRight => {
                            t.events.lock().unwrap().push(
                                TermEvent::Arrow { 
                                    x: 1, y: 0, 
                                    modifiers: 0
                                }
                            );
                        },

                        console::Key::Backspace => {
                            t.events.lock().unwrap().push(
                                TermEvent::Backspace {

                                }
                            );
                        }
                        
                        _ => {
                            println!("Unknown {:?}",key);
                        }
                    };
                } else {
                    break 'input_loop;
                }
    
                if buf_escseq.len() > 1 && match buf_escseq.chars().nth(buf_escseq.len()-1).unwrap() {'A'..='Z'=>true,_=>false} {
                    let l: char = buf_escseq.chars().nth(buf_escseq.len()-1).unwrap();
                    if l == 'M' {
                        let parts: Vec<u32> = buf_escseq[..buf_escseq.len()-1].to_string()[1..].split(";").map(|v|v.parse::<u32>().unwrap()).collect::<Vec<u32>>();
                        if parts.len() == 3 {
                            let ev: u32 = parts[0]; let ev_type: u32 = (ev>>5)&3;
                            // Modifiers:
                            //  1 : alt
                            //  2 : ctrl
                            // Button states:
                            //  0 : left click
                            //  1 : middle-click
                            //  2 : right click
                            //  3 : none
                            if ev_type == 0 { // ???
                                t.events.lock().unwrap().push(
                                    TermEvent::Mouse{
                                        event: MouseEvent::Unknown{ 
                                            x: parts[1] as i32, y: parts[2] as i32, state: parts[0] as i32, 
                                            data: 0
                                        }
                                    }
                                );
                            } else if ev_type == 1 { // Click
                                let button_state: u32 = ev&3;
                                // let modifier: u32 = (ev>>3)&3;
                                t.events.lock().unwrap().push(
                                    TermEvent::Mouse{
                                        event: MouseEvent::Click{ 
                                            x: parts[1] as i32, y: parts[2] as i32, state: parts[0] as i32, 
                                            data: 
                                                     if button_state == 3 {0} 
                                                else if button_state == 0 {1}
                                                else if button_state == 1 {2}
                                                else if button_state == 2 {4}
                                                else                      {0},
                                        }
                                    }
                                );
                            } else if ev_type == 2 { // Move
                                let button_state: u32 = ev&3;
                                // let modifier: u32 = (ev>>3)&3;
                                t.events.lock().unwrap().push(
                                    TermEvent::Mouse{
                                        event: MouseEvent::Move{ 
                                            x: parts[1] as i32, y: parts[2] as i32, state: parts[0] as i32, 
                                            data: 
                                                     if button_state == 3 {0} 
                                                else if button_state == 0 {1}
                                                else if button_state == 1 {2}
                                                else if button_state == 2 {4}
                                                else                      {0},
                                        }
                                    }
                                );
                            } else if ev_type == 3 { // Scroll
                                let dir: u32 = ev&1; // 0 -> up / 1 -> down
                                t.events.lock().unwrap().push(
                                    TermEvent::Mouse{
                                        event: MouseEvent::Scroll{ 
                                            x: parts[1] as i32, y: parts[2] as i32, state: parts[0] as i32, 
                                            data: if dir==1 {1} else {-1},
                                        }
                                    }
                                );
                            }
                        }
                    } else if l == 'A' || l == 'B' || l == 'C' || l == 'D' {
                        // shift          => 010
                        // ctrl           => 101
                        // alt            => 011
                        // ctrl+shift     => 110
                        // ctrl+alt       => 111
                        // shift+alt      => 100
                        // ctrl+shift+alt => 1000
                        // A : up
                        // B : down
                        // C : right
                        // D : left
                        t.events.lock().unwrap().push(
                            TermEvent::Arrow { 
                                x: if l == 'C' {-1} else if l == 'D' {1} else {0}, 
                                y: if l == 'A' {-1} else if l == 'B' {1} else {0}, 
                                modifiers: 0
                            }
                        );
                    }
                    buf_escseq.clear();
                }
    
            }
        });

    }

    pub fn consume_event(&mut self) -> Option<TermEvent> {
        let mut events = self.events.lock().unwrap();
        if events.len() == 0 {
            None
        } else {
            Some(events.remove(0))
        }
    }

    pub fn enable_mouse(&mut self) {
        print!("\x1b[?1003h\x1b[?1015h"); stdout().flush().unwrap();
        let mut flags = self.flags.lock().unwrap();
        *flags |= 1;
    }

    pub fn disable_mouse(&mut self) {
        print!("\x1b[?1015l\x1b[?1003l"); stdout().flush().unwrap();
        let mut flags = self.flags.lock().unwrap();
        *flags &= !1;
    }

    pub fn enable_alternate_buffer(&mut self) {
        print!("\x1b[?1049h"); stdout().flush().unwrap();
        let mut flags = self.flags.lock().unwrap();
        *flags |= 2;
    }

    pub fn disable_alternate_buffer(&mut self) {
        print!("\x1b[?1049l"); stdout().flush().unwrap();
        let mut flags = self.flags.lock().unwrap();
        *flags &= !2;
    }

}
