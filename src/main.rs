use std::{
    thread,
    sync::{
        Arc,
        Mutex
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

use ctrlc;

use termlib::{
    Term,
    TermEvent
};

mod termlib;


fn handle_conn( _ctx: Context, _stream: TcpStream ) {



}

fn sleep(ms:u64) {
    thread::sleep(time::Duration::from_millis(ms));
}

trait IBuffer: Send + Sync {
    /// Retrieves the text content of the buffer
    fn get_text(&mut self) -> String;
    /// Retreives a reference to the text content of the buffer
    fn get_text_mut(&mut self) -> &mut String;
    
    /// Retreives the position of the cursor as (colum,row)
    fn get_cursor_pos(&mut self) -> (i32,i32);
    /// Retreives the position of the cursor as (colum,row) fixed
    fn get_cursor_fix(&mut self) -> (i32,i32);
    /// Retreives a reference to the cursor as (column,row)
    fn get_cursor_mut(&mut self) -> &mut (i32,i32);
    /*/// Sets the position of the cursor as (colum,row)
    fn set_cursor_pos(&mut self, pos: (i32,i32));*/
    /*/// Retreives the position of the cursor given the index of the cursor in the text
    fn get_cursor_pos(&mut self, pos: i32) -> (i32,i32);*/
    
    /// Retreives the index of the cursor in the text
    fn get_cursor_idx(&mut self) -> i32;
    /// Sets the position of the cursor as its index in the text
    fn set_cursor_idx(&mut self, pos: i32);
    /*/// Retreives the index of the cursor in the text given the cursor as (column,row)
    fn get_cursor_idx(&mut self, pos: (i32,i32)) -> i32;*/

    fn clone_box(&mut self) -> Box<dyn IBuffer + Send + Sync>;
}

#[derive(Clone)]
struct TextBuffer {
    text: String,
    cursor: (i32,i32),
}

impl IBuffer for TextBuffer {
    fn get_cursor_pos(&mut self) -> (i32,i32) {
        self.cursor.clone()
    }
    fn get_cursor_fix(&mut self) -> (i32,i32) {
        (self.cursor.0,self.cursor.1.min(self.text.lines().nth(self.cursor.0 as usize).unwrap_or(&"").len() as i32))
    }
    fn get_cursor_idx(&mut self) -> i32 {
        // TODO: Simplify this
        (self.text.lines().collect::<Vec<&str>>()[..self.cursor.0.max(0) as usize].iter().fold(0,|acc,l|acc+l.len()+1)+self.text.lines().nth(self.cursor.0.max(0) as usize).unwrap_or(&"")[..self.cursor.1.max(0) as usize].len()) as i32
    }
    fn get_cursor_mut(&mut self) -> &mut (i32,i32) {
        &mut self.cursor
    }
    fn get_text(&mut self) -> String {
        self.text.clone()
    }
    fn get_text_mut(&mut self) -> &mut String {
        &mut self.text
    }
    fn set_cursor_idx(&mut self, pos: i32) {
        let prev: String = self.text[..pos as usize].to_string();
        self.cursor = (prev.lines().fold(0,|acc:i32,_:&str|acc+1)-1,(prev.lines().last().unwrap_or(&"").len() as i32));
    }
    fn clone_box(&mut self) -> Box<dyn IBuffer + Send + Sync> {
        Box::new(TextBuffer{
            text: self.text.clone(),
            cursor: self.cursor.clone()
        })
    }
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor: (0,0),
        }
    }
}

// #[derive(Clone)]
struct Buffer(Box<dyn IBuffer + Send + Sync + 'static>);

impl Buffer {
    pub fn new<T: IBuffer + Send + Sync + 'static>(buffer: T) -> Self {
        Buffer(Box::from(buffer))
    }
}

impl Deref for Buffer {
    type Target = Box<dyn IBuffer + Sync + Send>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone)]
struct Context {
    buffers: Arc<Mutex<Vec<Buffer>>>,
    selected_buffer: usize,
}

impl Context {
    pub fn new() -> Self {
        Self {
            buffers: Arc::from(Mutex::from(vec![Buffer::new(TextBuffer::new())])),
            selected_buffer: 0
        }
    }
    
    pub fn get_active_buffer(&self) -> Option<Buffer> {
        let mut buffers = self.buffers.lock().unwrap();
        if self.selected_buffer < buffers.len() {
            Some(Buffer(buffers.get_mut(self.selected_buffer).unwrap().clone_box()))
        } else { None }
    }

    pub fn set_active_buffer(&self, setter: Box<dyn Fn(&mut Buffer)->()>) -> bool {
        if let Some(buffer) = self.buffers.lock().unwrap().get_mut(self.selected_buffer) {
            setter(buffer);
            true
        } else { false }
    }
}

fn main() {

    ctrlc::set_handler(||{
        print!("\x1b[?1003l\x1b[?1049l");
        exit(0);
    }).unwrap();

    let ctx: Context = Context::new();
    
    let mut term: Term = Term::new();
    term.enable_mouse();
    term.enable_alternate_buffer();
    term.init();

    // Language server thread

    let server_ctx: Context = ctx.clone();
    thread::spawn(move||{
        let ctx: Context = server_ctx;

        let server: TcpListener = TcpListener::bind("0.0.0.0:1234").expect("Could not init language server.");

        for stream in server.incoming() {
            let listener_ctx = ctx.clone();
            thread::spawn(move||{ handle_conn(listener_ctx, stream.unwrap()); });
        }
    });

    // Render thread

    let render_ctx: Context = ctx.clone();
    let render_term: Term   = term.clone();
    thread::spawn(move||{
        let ctx: Context = render_ctx;
        let _term: Term  = render_term;

        loop { sleep(50);

            if let Some(mut buffer) = ctx.get_active_buffer() {

                let mut render_buff: String = String::new();

                render_buff += "\x1b[H\x1b[J";

                for line in buffer.get_text().lines().map(String::from).collect::<Vec<String>>() {
                    render_buff += &(line + "\n");
                }

                render_buff += &format!("\x1b[{};{}H",buffer.get_cursor_fix().0+1,buffer.get_cursor_fix().1+1);

                stdout().write_all(render_buff.as_bytes()).unwrap(); stdout().flush().unwrap();

            }
            
        }
    });

    // Input thread

    let input_ctx: Context = ctx.clone();
    let input_term: Term   = term.clone();
    thread::spawn(move||{
        let ctx: Context = input_ctx;
        let mut term: Term   = input_term;

        loop { sleep(10);

            while let Some(event) = term.consume_event() {

                match event {
                    TermEvent::Char{char} => {
                        ctx.set_active_buffer(Box::new(move|buffer: &mut Buffer|{
                            let ci: usize = buffer.get_cursor_idx() as usize;
                            buffer.get_text_mut().insert(ci,char);
                            buffer.get_cursor_mut().1 += 1;
                        }));
                    },
                    /*TermEvent::Mouse{event} => {

                    },*/
                    TermEvent::Arrow{x, y, modifiers: _} => {
                        ctx.set_active_buffer(Box::new(move|buffer: &mut Buffer|{
                            let text = buffer.get_text();
                            let cursor: &mut (i32,i32) = buffer.get_cursor_mut();
                            let lines: Vec<&str> = text.lines().collect::<Vec<&str>>();
                            cursor.0 = (cursor.0+y).max(0).min(lines.len() as i32-1);
                            cursor.1 = cursor.1+x;
                            if x != 0 {
                                if cursor.1 < 0 {
                                    cursor.1 = if cursor.0 > 0 { lines.get((cursor.0-1).max(0) as usize).unwrap_or(&"").len() as i32 } else { 0 };
                                    cursor.0 = (cursor.0-1).max(0);
                                }
                                else if cursor.1 > lines.get(cursor.0 as usize).unwrap_or(&"").len() as i32 {
                                    if cursor.0+1 < lines.len() as i32 {
                                        cursor.1 = 0;
                                        cursor.0 = (cursor.0+1).min(lines.len() as i32);
                                    } else {
                                        cursor.1 = lines.get(cursor.0 as usize).unwrap_or(&"").len() as i32;
                                    }
                                }
                                cursor.1 = cursor.1.min(lines.get(cursor.0 as usize).unwrap_or(&"").len() as i32);
                            }
                        }));
                    },
                    TermEvent::Enter{} => {
                        ctx.set_active_buffer(Box::new(move|buffer: &mut Buffer|{
                            let ci: usize = buffer.get_cursor_idx() as usize;
                            buffer.get_text_mut().insert(ci,'\n');
                            let cursor = buffer.get_cursor_mut();
                            cursor.0 += 1;
                            cursor.1 = 0;
                        }));
                    },
                    TermEvent::Backspace{} => {
                        ctx.set_active_buffer(Box::new(move|buffer: &mut Buffer|{
                            let i = buffer.get_cursor_idx();
                            if i != 0 {
                                buffer.get_text_mut().remove(i as usize-1);
                                buffer.set_cursor_idx(i as i32-1);
                            }
                        }));
                    },
                    _ => {},
                };
    
            }

        }
    });

    loop {}

}
