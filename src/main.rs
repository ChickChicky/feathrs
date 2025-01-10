#![allow(unused_mut,dead_code)]

use core::panic;
use std::{collections::VecDeque, env::{args, current_dir}, fs, io::{stdin, stdout, Write}, path::{self, Path}, sync::{Arc, Mutex}, thread::{sleep, spawn}, time::{Duration, Instant}};

use libc::{self, termios};
use renderer::{Color, Renderer, Style};
use serde::{Deserialize, Serialize};
use termion::{event::{Event, Key}, input::TermReadEventsAndRaw};

mod renderer;

const BACKGROUND : Color = Color::RGB(40, 42, 54);
const HEAD : Color = Color::RGB(68, 71, 90);
const CURRENT : Color = Color::RGB(50, 52, 64);
const FOREGROUND : Color = Color::RGB(248, 248, 242);
const COMMENT : Color = Color::RGB(98, 114, 164);
const RED : Color = Color::RGB(255, 85, 85);
const YELLOW : Color = Color::RGB(241, 250, 140);

const BLINK_HOLD : Duration = Duration::from_millis(200);

#[derive(Serialize, Deserialize, Debug)]
struct ColorSettings {
    background: Color,
    head: Color,
    current: Color,
    foreground: Color,
    comment: Color,
    /// black: Color,
    red: Color,
    // green: Color,
    yellow: Color,
    // blue: Color,
    // magenta: Color,
    // cyan: Color,
    // white: Color,
}

#[derive(Serialize, Deserialize, Debug)]
struct ControlSettings {
    smart_home: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct Settings {
    color: ColorSettings,
    controls: ControlSettings,
}

struct Clock {
    next: Instant,
    delay: Duration,
}

impl Clock {
    fn new(delay: Duration) -> Self {
        Self {
            delay,
            next: Instant::now() + delay
        }
    }
    fn tick(&mut self) -> bool {
        let now = Instant::now();
        if now >= self.next {
            self.next = now + self.delay;
            return true;
        }
        return false;
    }
}

trait Window {
    fn render(&mut self, env: &mut Env, renderer: &mut Renderer) -> ();
    fn key_pressed(&mut self, env: &mut Env, ev: Event) -> ();
}

#[derive(Clone)]
enum BufferMenuState {
    None,
    Open(String),
    Command(String),
    SaveFailed,
    Save(String),
    Overwrite(String),
    Load(String),
    ForceLoad,
}

impl BufferMenuState {
    
}

struct Buffer {
    body: String,
    cursor: (i32, i32),
    scroll: (i32, i32),
    menu: Option<BufferMenuState>,
    saved: bool,
    path: Option<String>,
    hold_blink: Instant,
}

impl Buffer {
    fn new() -> Self {
        Self {
            body: String::new(),
            cursor: (0, 0),
            scroll: (0, 0),
            menu: None,
            saved: true,
            path: None,
            hold_blink: Instant::now(),
        }
    }

    fn from_file(path: &String) -> Self {
        let body = fs::read_to_string(path);
        Self {
            saved: body.is_ok(),
            body: body.unwrap_or(String::new()),
            cursor: (0, 0),
            scroll: (0, 0),
            menu: None,
            path: Some(path.clone()),
            hold_blink: Instant::now(),
        }
    }

    fn cur(&self, c: (i32, i32)) -> usize {
        if self.body.len() == 0 {
            return 0
        }

        let lines = self.body.split('\n').collect::<Vec<&str>>();
        
        let cy = (c.1.max(0) as usize).min(lines.len()-1);

        let mut j = 0usize;

        for y in 0..lines.len() {
            let l = lines[y].len() + 1;
            if cy == y {
                let cx = (c.0.max(0) as usize).min(l-1);
                return j + cx;
            }
            j += l;
        }

        return self.body.len();
    }

    fn fix(&self, c: (i32, i32)) -> (i32, i32) {
        let lines = self.body.split('\n').collect::<Vec<&str>>();
        
        let cy = (c.1.max(0) as usize).min(lines.len()-1) as i32;
        let cx = (c.0.max(0) as usize).min(lines[cy as usize].len()) as i32;

        return (cx, cy);
    }

    fn ipos(&self, i: usize) -> (i32,i32) {
        if self.body.len() == 0 {
            return (0, 0)
        }

        let lines = self.body.split('\n').collect::<Vec<&str>>();

        let mut j = 0usize;

        for y in 0..lines.len() {
            let nj = j + lines[y].len() + 1;
            if i >= j && i < nj {
                return ((i-j) as i32, y as i32)
            }
            j = nj;
        }

        return (lines.last().unwrap().len() as i32, (lines.len()-1) as i32);
    }

    pub fn write(&mut self) -> bool {
        if let Some(path) = self.path.clone() {
            fs::write(path, self.body.clone())
                .and_then(
                    |_r| Ok(self.saved=true)
                )
                .is_ok()
        } else {
            false
        }
    }

    pub fn read(&mut self) -> bool {
        if let Some(path) = self.path.clone() {
            fs::read_to_string(path)
                .and_then(
                    |r| {
                        self.body=r;
                        Ok(self.saved=true)
                    }
                )
                .is_ok()
        } else {
            false
        }
    }
}

fn leading_whitespaces(str: &str) -> usize {
    str
        .chars()
        .take_while(|char| char.is_whitespace())
        .map(|char| char.len_utf8())
        .sum()
}

impl Window for Buffer {
    fn render(&mut self, _env: &mut Env, renderer: &mut Renderer) {
        renderer.clear();

        let w = renderer.buffer.width;
        let h = renderer.buffer.height;

        if w < 10 || h < 3 {
            renderer.fill(0, 0, w, h, renderer::Cell { c: '!', s: Style::default() });
            renderer.render();
            renderer.flip();
            stdout().flush().unwrap();
            return;
        }

        let tw = w - 5u32;
        let th = h - 2u32;

        let cur = self.fix(self.cursor);
        let (cx, cy) = (cur.0 - self.scroll.0, cur.1 - self.scroll.1);
        
        renderer.paint(0, h as u32 -1, w as u32, 1, Style::default().fg(FOREGROUND).bg(HEAD).clone());
        renderer.paint(5, 1, tw, th, Style::default().bg(BACKGROUND).fg(FOREGROUND).clone());

        let cursor: Option<(i32,i32)> =
            if let Some(menu) = &self.menu {
                match menu {
                    BufferMenuState::None => {
                        Some((0i32, h as i32 -1))
                    },
                    BufferMenuState::Open(message) => {
                        renderer.get_mut(0, h-1).c = '🍵';
                        renderer.put_text(w - message.len() as u32 -1, h-1, message.clone());
                        Some((2, h as i32 - 1))
                    }
                    BufferMenuState::Command(cmd) => {
                        renderer.get_mut(0, h-1).c = '📜';
                        renderer.put_text(2, h-1, cmd.clone());
                        Some(((cmd.len()) as i32+2, h as i32 - 1))
                    }
                    BufferMenuState::SaveFailed => {
                        let message = "Abandon unsaved? ".to_string(); // TODO: Inline below :p
                        renderer.get_mut(0, h-1).c = '❗';
                        renderer.put_text(2, h-1, message.clone());
                        Some(((message.len()) as i32+2, h as i32 - 1))
                    }
                    BufferMenuState::Save(path) => {
                        renderer.get_mut(0, h-1).c = '💾';
                        renderer.put_text(2, h-1, path.clone());
                        Some((path.len() as i32+2, h as i32-1))
                    }
                    BufferMenuState::Overwrite(_path) => {
                        let message = "Overwrite? ".to_string();
                        renderer.get_mut(0, h-1).c = '💾';
                        renderer.put_text(2, h-1, message.clone());
                        Some((message.len() as i32+2, h as i32-1))
                    }
                    BufferMenuState::Load(path) => {
                        renderer.get_mut(0, h-1).c = '📂';
                        renderer.put_text(2, h-1, path.clone());
                        Some((path.len() as i32+2, h as i32-1))
                    }
                    BufferMenuState::ForceLoad => {
                        let message = "Abandon unsaved? ".to_string();
                        renderer.get_mut(0, h-1).c = '📂';
                        renderer.put_text(2, h-1, message.clone());
                        Some((message.len() as i32+2, h as i32-1))
                    }
                }
            }
            else {
                let fmt = format!("{}:{}",cy+1,cx+1);
                renderer.put_text(w-1-fmt.len() as u32, h-1, fmt);
                renderer.get_mut(0, h-1).c = '🪶';
                None
            }
        ;

        // renderer.paint(0, 1, 4, th, Style::default().bg(if cursor.is_none() {COMMENT} else {HEAD}).clone());
        renderer.paint(0, 1, 4, th, Style::default().bg(COMMENT).clone());
        renderer.paint(4, 1, 1, th, Style::default().fg(HEAD).clone());

        {
            renderer.paint(0, 0, w as u32, 1, Style::default().fg(FOREGROUND).bg(HEAD).clone());
            let path = self
                .path.clone()
                .and_then(|p|
                    Path::new(&p.clone())
                        .file_name()
                        .and_then(|q|
                            Some(q.to_string_lossy().to_string())
                        )
                )
                .unwrap_or("<new>".to_string())
            ;
            let off = w/2-(path.len() as u32)/2;
            renderer.put_text(off, 0, path);
            if !self.saved {
                renderer.put_text(w-1, 0, "M".to_string());
            }
        }

        let body = self.body.clone();
        let lines = body.split('\n').collect::<Vec<&str>>();

        if cy >= 0 && (cy as u32) <= th {
            renderer.paint(5, (cy+1) as u32, (w-5) as u32, 1, Style::default().fg(FOREGROUND).bg(CURRENT).clone());
            renderer.paint(0, (cy+1) as u32, 4, 1, Style::default().fg(COMMENT).bg(FOREGROUND).clone());
        }

        let mut bi = 0;

        for y in 0 .. th {
            renderer.put_text(3, y+1, "~┃".to_string());
        }

        for j in 0 .. th {
            let ii = j as i32 + self.scroll.1;
            if ii < 0 || ii > (th as i32) {
                continue
            }
            let i = ii as u32;
            // if let Some(line) = lines.get((i as i32+self.scroll.) as usize) {
            if let Some(line) = lines.get(i as usize) {
                renderer.put_text(0, j+1, {let s = format!("{: >4}",i+1); s[s.len()-4..s.len()].to_string()});
                for l in 0usize .. line.len() {
                    let x = l as i32 -self.scroll.0;
                    if x > 0 {
                        if x as u32 >= tw { 
                            break;
                        }
                        renderer.get_mut(x as u32+5, j+1).c = line.as_bytes()[l] as char;
                    }
                }
                renderer.put_text(5, j+1, line[(self.scroll.0 as usize).min(line.len())..line.len().min(tw as usize)].to_string());
                let nbi = bi + line.len() + 1;
                bi = nbi;
            }
        }

        if self.menu.is_none() && cx >= 0 && (cx as u32) < tw && cy >= 0 && (cy as u32) <= th {
            renderer.get_mut((cx+5) as u32, (cy+1) as u32).s.reverse(Instant::now()<self.hold_blink||Instant::now().duration_since(self.hold_blink).as_millis()%1000 < 500);
        }

        /*renderer.put(&TextOptions{
            pos: (5,1),
            offset: None,
            text: body.clone(),
            max_w: Some((w-5) as i32),
            max_h: Some((h-2) as i32),
            wrap: Some(false),
            style: None
        });*/

        renderer.apply(5, 1, tw, th, &|cell, _x, _y| {
            if cell.c < '\x20' {
                cell.c = char::from_u32((cell.c as u32) + 0x2400u32).unwrap();
                cell.s = cell.s.clone().bg(RED).clone();
            }
        });

        print!("\x1b[{};1H",h);
        if let Some((x,y)) = cursor {
            print!("\x1b[?25h\x1b[{};{}H",y+1,x+1);
        } else {
            print!("\x1b[?25l");
        }

        renderer.render();
        renderer.flip();
        stdout().flush().unwrap();
    }

    fn key_pressed(&mut self, env: &mut Env, ev: Event) {
        if let Some(menu) = &mut self.menu {
            let mut new_menu = menu.clone();
            match ev {
                Event::Key(key) => {
                    match key {
                        Key::Esc => {
                            new_menu = BufferMenuState::None;
                        }
                        Key::Char(c) => {
                            match menu {
                                BufferMenuState::None => {
                                    // Not supposed to happen?
                                    panic!("Invalid menu state");
                                }
                                BufferMenuState::Open(_message) => {
                                    if c == ':' {
                                        new_menu = BufferMenuState::Command(String::new());
                                    }
                                    else if c == 'w' {
                                        if self.path.is_some() {
                                            if self.write() {
                                                new_menu = BufferMenuState::Open(format!("Wrote {} bytes",self.body.len()));
                                            } else {
                                                new_menu = BufferMenuState::Open("Could not write".to_string());
                                            }
                                        } else {
                                            if let Ok(dir) = current_dir() {
                                                new_menu = BufferMenuState::Save(format!("{}/",dir.to_str().unwrap()));
                                            } else {
                                                new_menu = BufferMenuState::Open("Could not open write menu".to_string());
                                            }
                                        }
                                    }
                                    else if c == 'r' {
                                        if self.path.is_some() {
                                            if self.saved {
                                                if self.read() {
                                                    new_menu = BufferMenuState::Open(format!("Read {} bytes",self.body.len()));
                                                } else {
                                                    new_menu = BufferMenuState::Open("Could not read".to_string());
                                                }
                                            } else {
                                                new_menu = BufferMenuState::ForceLoad;
                                            }
                                        } else {
                                            if self.saved {
                                                if let Ok(dir) = current_dir() {
                                                    new_menu = BufferMenuState::Load(format!("{}/",dir.to_str().unwrap()));
                                                } else {
                                                    new_menu = BufferMenuState::Open("Could not open load menu".to_string());
                                                }
                                            } else {
                                                new_menu = BufferMenuState::ForceLoad;
                                            }
                                        }
                                    }
                                    else if c == 'q' {
                                        if self.saved {
                                            env.running = false;
                                        } else {
                                            new_menu = BufferMenuState::SaveFailed;
                                        }
                                    }
                                }
                                BufferMenuState::ForceLoad => {
                                    if c == 'y' || c == 'Y' {
                                        if self.path.is_some() {
                                            if self.read() {
                                                new_menu = BufferMenuState::Open(format!("Read {} bytes",self.body.len()));
                                            } else {
                                                new_menu = BufferMenuState::Open("Could not read".to_string());
                                            }
                                        }
                                        else {
                                            if let Ok(dir) = current_dir() {
                                                new_menu = BufferMenuState::Load(format!("{}/",dir.to_str().unwrap()));
                                            } else {
                                                new_menu = BufferMenuState::Open("Could not open load menu".to_string());
                                            }
                                        }
                                    }
                                    if c == 'n' || c == 'N' {
                                        new_menu = BufferMenuState::Open(String::new());
                                    }
                                }
                                BufferMenuState::Command(cmd) => {
                                    if !c.is_control() {
                                        cmd.push(c);
                                        new_menu = menu.clone();
                                    }
                                    else if c == '\n' || c == '\r' {
                                        // TODO: Run the action
                                        new_menu = BufferMenuState::None;
                                    }
                                }
                                BufferMenuState::Save(path) => {
                                    if !c.is_control() {
                                        path.push(c);
                                        new_menu = menu.clone();
                                    }
                                    else if c == '\n' || c == '\r' {
                                        let p = self.path.clone();
                                        if self.path.clone().unwrap_or("".to_string()) != path.clone() {
                                            new_menu = BufferMenuState::Overwrite(path.clone());
                                        } else {
                                            self.path = Some(path.clone());
                                            if self.write() {
                                                new_menu = BufferMenuState::Open(format!("Wrote {} bytes",self.body.len()));
                                            } else {
                                                new_menu = BufferMenuState::Open("Could not write".to_string());
                                                self.path = p;
                                            }
                                        }
                                    }
                                }
                                BufferMenuState::Overwrite(path) => {
                                    if c == 'y' || c == 'Y' {
                                        let p = self.path.clone();
                                        self.path = Some(path.clone());
                                        if self.write() {
                                            new_menu = BufferMenuState::Open(format!("Wrote {} bytes",self.body.len()));
                                        } else {
                                            new_menu = BufferMenuState::Open("Could not write".to_string());
                                            self.path = p;
                                        }
                                    }
                                    if c == 'n' || c == 'N' {
                                        new_menu = BufferMenuState::Open(String::new());
                                    }
                                }
                                BufferMenuState::Load(path) => {
                                    if !c.is_control() {
                                        path.push(c);
                                        new_menu = menu.clone();
                                    }
                                    else if c == '\n' {
                                        let p = self.path.clone();
                                        self.path = Some(path.clone());
                                        if self.read() {
                                            new_menu = BufferMenuState::Open(format!("Read {} bytes",self.body.len()));
                                        } else {
                                            new_menu = BufferMenuState::Open("Could not read".to_string());
                                            self.path = p;
                                        }
                                    }
                                }
                                BufferMenuState::SaveFailed => {
                                    if c == 'y' || c == 'Y' {
                                        env.running = false;
                                    }
                                    else if c == 'n' || c == 'N' {
                                        new_menu = BufferMenuState::Open(String::new());
                                    }
                                }
                            }
                        }
                        Key::Backspace => {
                            match menu {
                                BufferMenuState::Command(cmd) => {
                                    if cmd.len() > 0 {
                                        cmd.pop();
                                    }
                                    new_menu = menu.clone();
                                }
                                BufferMenuState::Save(path) => {
                                    if path.len() > 0 {
                                        path.pop();
                                    }
                                    new_menu = menu.clone();
                                }
                                BufferMenuState::Load(path) => {
                                    if path.len() > 0 {
                                        path.pop();
                                    }
                                    new_menu = menu.clone();
                                }
                                _ => {} 
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
            self.menu = match new_menu {
                BufferMenuState::None => None,
                menu => Some(menu)
            };
        } else {
            match ev {
                Event::Key(key) => {
                    // println!("{:?}",key);
                    let mut blink = false;
                    match key {
                        Key::Esc => {
                            self.menu = Some(BufferMenuState::Open(String::new()));
                        }
                        Key::Char(c) => {
                            let ci = self.cur(self.fix(self.cursor));
                            if c == '\x09' {
                                self.body.insert_str(ci, "    ");
                                self.cursor = self.ipos(ci+4);
                            } else if c == '\r' {
                                self.body.insert(ci, '\n');
                                self.cursor = self.ipos(ci+1);
                            } else {
                                self.body.insert(ci, c);
                                self.cursor = self.ipos(ci+1);
                            }
                            self.saved = false;
                        }
                        Key::Backspace => {
                            let ci = self.cur(self.fix(self.cursor));
                            if ci > 0 {
                                self.body.remove(ci-1);
                                self.cursor = self.ipos(ci-1);
                                self.saved = false;
                            }
                        }
                        /*Key::Ctrl(_c) => {
                            
                        }*/
                        Key::Alt(_c) => {
                            // ?
                        }
                        Key::Up => {
                            self.cursor.1 -= 1;
                            if self.cursor.1 < 0 {
                                self.cursor = (0,0);
                            }
                        }
                        Key::Down => {
                            let lines = self.body.split('\n').collect::<Vec<&str>>();
                            self.cursor.1 += 1;
                            if self.cursor.1 as usize >= lines.len() {
                                self.cursor = ((lines.len()-1) as i32,lines.last().unwrap().len() as i32);
                            }
                        }
                        Key::Left => {
                            let ci = self.cur(self.fix(self.cursor));
                            if ci != 0 {
                                self.cursor = self.ipos(ci-1);
                            }
                        }
                        Key::Right => {
                            self.cursor = self.ipos(self.cur(self.fix(self.cursor))+1)
                        }
                        Key::CtrlUp => {
                            if self.scroll.1 > 0 {
                                self.scroll.1 -= 1;
                            }
                        }
                        Key::CtrlDown => {
                            let lines = self.body.split('\n').collect::<Vec<&str>>();
                            if (self.scroll.1 as usize) +1 < lines.len() {
                                self.scroll.1 += 1;
                            }
                        }
                        Key::CtrlLeft => {
                            if self.scroll.0 > 0 {
                                self.scroll.0 -= 1;
                            }
                        }
                        Key::CtrlRight => {
                            let maxlen = self.body.split('\n').map(|l|l.len()).max().unwrap();
                            if (self.scroll.0 as usize) +1 < maxlen {
                                self.scroll.0 += 1;
                            }
                        }
                        Key::End => {
                            self.cursor.0 = self.body.split('\n').nth(self.cursor.1 as usize).unwrap().len() as i32;
                        }
                        Key::Home => {
                            if env.settings.controls.smart_home {
                                let line = self.body.split('\n').nth(self.cursor.1 as usize).unwrap();
                                let i = leading_whitespaces(line) as i32;

                                self.cursor.0 = if self.cursor.0 == i { 0 } else { i };
                            } else {
                                self.cursor.0 = 0;
                            }
                        }
                        _ => {
                            let ci = self.cur(self.fix(self.cursor));
                            let s = format!("{:?}",key);
                            self.body.insert_str(ci, s.as_str());
                            self.cursor = self.ipos(ci+s.len());
                            blink = true;
                        }
                    }
                    if !blink {
                        self.hold_blink = Instant::now() + BLINK_HOLD;
                    }
                }
                _ => {}
            }
        }
    }
}

struct Windows {
    windows: Vec<Box<dyn Window>>,
    current: usize,
}

impl Windows {
    fn new() -> Self {
        Self {
            windows: vec![],
            current: usize::MAX,
        }
    }

    fn push(&mut self, win: Box<dyn Window>, focus: bool) {
        self.windows.push(win);
        if focus {
            self.current = self.windows.len() -1usize;
        }
    }

    fn focused(&mut self) -> &mut Box<dyn Window> {
        return &mut self.windows[self.current];
    }
}

struct Env {
    windows: Windows,
    running: bool,
    settings: Settings,
}

fn raw_stdin() -> termios {
    let mut termios = core::mem::MaybeUninit::uninit();
    unsafe { libc::tcgetattr(libc::STDIN_FILENO, termios.as_mut_ptr()); }
    let mut termios = unsafe { termios.assume_init() };
    let v =  termios.clone();
    termios.c_lflag &= !(libc::IGNBRK | libc::ICANON | libc::ECHO);
    unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &termios); }
    return v;
}

fn unraw_stdin(original: termios) {
    unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &original); }
}

fn main() {
    print!("\x1b[?1049h");
    let original_termios = raw_stdin();

    let mut renderer = Renderer::new();

    let mut render_clk = Clock::new(Duration::from_millis(16));

    let mut events = Arc::new(Mutex::new(VecDeque::<(Event,Vec<u8>)>::new()));
    let mut tevents = events.clone();

    let mut args = args();

    let _program = args.next().unwrap();

    let filename = args.next()
        .and_then(|p|
            if Path::new(&p).is_file() {
                path::absolute(p)
                .and_then(|q|
                    Ok(q.to_string_lossy().to_string())
                )
                .ok()
            } else {
                None
            }
        )
    ;

    /*let mut body = String::new();

    for c in '\x00'..'\x21' {
        body.push(c);
    }*/

    // Kind of annoying, but stdin reads are blocking,
    // so they need to be done in a dedicated thread
    spawn(move||{
        for e in stdin().events_and_raw() {
            tevents.lock().unwrap().push_front(e.unwrap());
        }
    });

    let mut env = Env{
        windows: Windows::new(),
        running: true,
        settings: Settings{
            color: ColorSettings{
                background: Color::RGB(40, 42, 54),
                head: Color::RGB(68, 71, 90),
                current: Color::RGB(50, 52, 64),
                foreground: Color::RGB(248, 248, 242),
                comment: Color::RGB(98, 114, 164),
                red: Color::RGB(255, 85, 85),
                yellow: Color::RGB(241, 250, 140),
            },
            controls: ControlSettings{
                smart_home: true,
            },
        }
    };

    let mut dst = "".to_string();
    let mut ser = toml::Serializer::new(&mut dst);
    if let Err(err) = env.settings.serialize(ser) {
        eprintln!("{}",err);
        panic!();
    }
    if let Err(err) = fs::write("./settings.toml".to_string(), dst) {
        eprintln!("{}",err);
        panic!();
    }

    env.windows.push(
        Box::new(
            if let Some(filename) = filename {
                Buffer::from_file(&filename)
            }
            else {
                Buffer::new()
            }
        ), 
        true
    );

    while env.running {
        sleep(Duration::from_millis(1));

        // Process Events
        while let Some(event) = events.lock().unwrap().pop_back() {
            let (ev, _keys) = event;
            let e = (&mut env) as *mut Env;
            unsafe { env.windows.focused().key_pressed(&mut *e, ev); }
        }

        if render_clk.tick() {
            let e = (&mut env) as *mut Env;
            unsafe { env.windows.focused().render(&mut *e, &mut renderer); }
        }

    }

    print!("\x1b[?1049l\x1b[?25h");
    stdout().flush().unwrap();
    unraw_stdin(original_termios);
}
