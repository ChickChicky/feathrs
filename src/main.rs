use std::{
    thread,
    sync::{
        Arc,
        Mutex,
        mpsc::{Sender,Receiver,channel}
    }, 
    net::{
        TcpListener, 
        TcpStream
    },
    ops::{Deref, DerefMut},
    process::exit,
    time,
    io::{
        stdout, 
        Write
    },
};

use termsize;

use ctrlc;

use termlib::{
    Term,
    TermEvent
};

mod termlib;

enum WindowEvent {
    TermEvent{evt:TermEvent},
}

trait IWindow {
    fn init(&mut self, chan:Receiver<WindowEvent>);
    fn render(&mut self) -> String;
    fn close(&mut self);
}

struct BufferWindow {
    cursor: (u32,u32),
    buff: String,
    events: Receiver<WindowEvent>,
}

impl IWindow for BufferWindow {
    fn init(&mut self, chan:Receiver<WindowEvent>) {
        self.events = chan;
    }

    fn render(&mut self) -> String {
        let mut render_buff: String = String::new();

        for line in self.buff.lines().map(String::from).collect::<Vec<String>>() {
            render_buff += &(line + "\n");
        }

        render_buff += &format!("\x1b[{};{}H",1,1);

        return render_buff;
    }

    fn close(&mut self) {
        todo!()
    }
}

struct ContextWindow {
    window: Box<dyn IWindow>
}

struct Context {
    windows: Vec<ContextWindow>,
    current: i32
}

impl Context {
    pub fn new() -> Context {
        Context {
            windows: vec![],
            current: 0
        }
    }
    pub fn get_active_window(&mut self) -> Option<Box<dyn IWindow>> {
        if self.current > 0 && self.current as usize > self.windows.len() {
            self.current = (self.windows.len() as i32)-1;
        }
        if self.current < 0 {
            return None;
        }
        return Some( self.windows.get(self.current as usize).unwrap().window );
    }
}

fn handle_conn( _ctx: Context, _stream: TcpStream ) {



}

fn sleep(ms:u64) {
    thread::sleep(time::Duration::from_millis(ms));
}

fn main() {

    ctrlc::set_handler(||{
        print!("\x1b[?1003l\x1b[?1049l");
        exit(0);
    }).unwrap();

    let mut ctx: Context = Context::new();
    
    let mut term: Term = Term::new();
    term.enable_mouse();
    term.enable_alternate_buffer();
    term.init();

    // Language server thread

    // let server_ctx: Context = ctx.clone();
    // thread::spawn(move||{
    //     let ctx: Context = server_ctx;

    //     let server: TcpListener = TcpListener::bind("0.0.0.0:1234").expect("Could not init language server.");

    //     for stream in server.incoming() {
    //         let listener_ctx = ctx.clone();
    //         thread::spawn(move||{ handle_conn(listener_ctx, stream.unwrap()); });
    //     }
    // });

    // Render thread

    // let render_ctx: Context = ctx.clone();
    // let render_term: Term   = term.clone();
    // thread::spawn(move||{
    //     let ctx: Context = render_ctx;
    //     let _term: Term  = render_term;

    //     loop { sleep(50);

    //         let mut render_buff: String = String::new();

    //         render_buff += "\x1b[H";

    //         if let Some(mut window) = ctx.get_active_window() {
    //             render_buff += window.render();
    //         } else {
    //             let msg: &str = "No active window";
    //             let sz:termsize::Size = termsize::get().unwrap();
    //             render_buff += &format!("\x1b[{};{}H\x1b[37m",sz.cols/2-msg.len()/2,sz.rows/2);
    //             render_buff += msg;
    //             render_buff += "\x1b[39m";
    //         }

    //         stdout().write_all(render_buff.as_bytes()).unwrap(); stdout().flush().unwrap();
            
    //     }
    // });

    // Input thread

    // let input_ctx: Context = ctx.clone();
    // let input_term: Term   = term.clone();
    // thread::spawn(move||{
    //     let ctx: Context = input_ctx;
    //     let mut term: Term   = input_term;

    //     loop { sleep(10);

    //         while let Some(event) = term.consume_event() {

    //             match event {
    //                 TermEvent::Char{char} => {
    //                     ctx.set_active_buffer(Box::new(move|buffer: &mut Buffer|{
    //                         let ci: usize = buffer.get_cursor_idx() as usize;
    //                         buffer.get_text_mut().insert(ci,char);
    //                         buffer.get_cursor_mut().1 += 1;
    //                     }));
    //                 },
    //                 /*TermEvent::Mouse{event} => {

    //                 },*/
    //                 TermEvent::Arrow{x, y, modifiers: _} => {
    //                     ctx.set_active_buffer(Box::new(move|buffer: &mut Buffer|{
    //                         let text = buffer.get_text();
    //                         let cursor: &mut (i32,i32) = buffer.get_cursor_mut();
    //                         let lines: Vec<&str> = text.lines().collect::<Vec<&str>>();
    //                         cursor.0 = (cursor.0+y).max(0).min(lines.len() as i32-1);
    //                         cursor.1 = cursor.1+x;
    //                         if x != 0 {
    //                             if cursor.1 < 0 {
    //                                 cursor.1 = if cursor.0 > 0 { lines.get((cursor.0-1).max(0) as usize).unwrap_or(&"").len() as i32 } else { 0 };
    //                                 cursor.0 = (cursor.0-1).max(0);
    //                             }
    //                             else if cursor.1 > lines.get(cursor.0 as usize).unwrap_or(&"").len() as i32 {
    //                                 if cursor.0+1 < lines.len() as i32 {
    //                                     cursor.1 = 0;
    //                                     cursor.0 = (cursor.0+1).min(lines.len() as i32);
    //                                 } else {
    //                                     cursor.1 = lines.get(cursor.0 as usize).unwrap_or(&"").len() as i32;
    //                                 }
    //                             }
    //                             cursor.1 = cursor.1.min(lines.get(cursor.0 as usize).unwrap_or(&"").len() as i32);
    //                         }
    //                     }));
    //                 },
    //                 TermEvent::Enter{} => {
    //                     ctx.set_active_buffer(Box::new(move|buffer: &mut Buffer|{
    //                         let ci: usize = buffer.get_cursor_idx() as usize;
    //                         buffer.get_text_mut().insert(ci,'\n');
    //                         let cursor = buffer.get_cursor_mut();
    //                         cursor.0 += 1;
    //                         cursor.1 = 0;
    //                     }));
    //                 },
    //                 TermEvent::Backspace{} => {
    //                     ctx.set_active_buffer(Box::new(move|buffer: &mut Buffer|{
    //                         let i = buffer.get_cursor_idx();
    //                         if i != 0 {
    //                             buffer.get_text_mut().remove(i as usize-1);
    //                             buffer.set_cursor_idx(i as i32-1);
    //                         }
    //                     }));
    //                 },
    //                 _ => {},
    //             };
    
    //         }

    //     }
    // });

    loop {

        sleep(50);

        let mut render_buff: String = String::new();

        render_buff += "\x1b[H";

        if let Some(mut window) = ctx.get_active_window() {
            render_buff += &window.render();
        } else {
            let msg: &str = "No active window";
            let sz:termsize::Size = termsize::get().unwrap();
            render_buff += &format!("\x1b[{};{}H\x1b[90m",sz.rows/2+1,sz.cols/2-(msg.len() as u16)/2+1);
            render_buff += msg;
            render_buff += "\x1b[39m";
        }

        stdout().write_all(render_buff.as_bytes()).unwrap(); stdout().flush().unwrap();
        
    }

}
