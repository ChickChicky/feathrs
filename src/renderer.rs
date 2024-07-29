#![allow(dead_code)]

use std::io::{stdout, Write};
use terminal_size::{terminal_size,Width,Height};
use macon::Builder;

#[derive(Clone, Copy, PartialEq)]
pub enum Color {
    Unset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BrightBlack,
    BrightRed,
    BrightGreen,
    BrightYellow,
    BrightBlue,
    BrightMagenta,
    BrightCyan,
    BrightWhite,
    Color256(u8),
    RGB(u8,u8,u8)
}

impl Color {
    pub fn foreground(self) -> String {
        match self {
            Color::Unset   => "39".to_string(),
            Color::Black   => "30".to_string(),
            Color::Red     => "31".to_string(),
            Color::Green   => "32".to_string(),
            Color::Yellow  => "33".to_string(),
            Color::Blue    => "34".to_string(),
            Color::Magenta => "35".to_string(),
            Color::Cyan    => "36".to_string(),
            Color::White   => "37".to_string(),
            Color::BrightBlack   => "90".to_string(),
            Color::BrightRed     => "91".to_string(),
            Color::BrightGreen   => "92".to_string(),
            Color::BrightYellow  => "93".to_string(),
            Color::BrightBlue    => "94".to_string(),
            Color::BrightMagenta => "95".to_string(),
            Color::BrightCyan    => "96".to_string(),
            Color::BrightWhite   => "97".to_string(),
            Color::Color256(i) => format!("38;5;{}",i),
            Color::RGB(r,g,b) => format!("38;2;{};{};{}",r,g,b)
        }
    }
    pub fn background(self) -> String {
        match self {
            Color::Unset   => "49".to_string(),
            Color::Black   => "40".to_string(),
            Color::Red     => "41".to_string(),
            Color::Green   => "42".to_string(),
            Color::Yellow  => "43".to_string(),
            Color::Blue    => "44".to_string(),
            Color::Magenta => "45".to_string(),
            Color::Cyan    => "46".to_string(),
            Color::White   => "47".to_string(),
            Color::BrightBlack   => "100".to_string(),
            Color::BrightRed     => "101".to_string(),
            Color::BrightGreen   => "102".to_string(),
            Color::BrightYellow  => "103".to_string(),
            Color::BrightBlue    => "104".to_string(),
            Color::BrightMagenta => "105".to_string(),
            Color::BrightCyan    => "106".to_string(),
            Color::BrightWhite   => "107".to_string(),
            Color::Color256(i) => format!("48;5;{}",i),
            Color::RGB(r,g,b) => format!("48;2;{};{};{}",r,g,b)
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Style {
    fg: Color,
    bg: Color,
    bold: bool,
    faint: bool,
    italic: bool,
    underline: bool,
    strike: bool,
    reverse: bool
}

impl Style {
    pub fn default() -> Style {
        Style {
            fg: Color::Unset,
            bg: Color::Unset,
            bold: false,
            faint: false,
            italic: false,
            underline: false,
            strike: false,
            reverse: false
        }
    }

    pub fn fg(&mut self, fg: Color) -> &mut Self {
        self.fg = fg; self
    }
    pub fn bg(&mut self, bg: Color) -> &mut Self {
        self.bg = bg; self
    }
    pub fn bold(&mut self, bold: bool) -> &mut Self {
        self.bold = bold; self
    }
    pub fn faint(&mut self, faint: bool) -> &mut Self {
        self.faint = faint; self
    }
    pub fn italic(&mut self, italic: bool) -> &mut Self {
        self.italic = italic; self
    }
    pub fn underline(&mut self, underline: bool) -> &mut Self {
        self.underline = underline; self
    }
    pub fn strike(&mut self, strike: bool) -> &mut Self {
        self.strike = strike; self
    }
    pub fn reverse(&mut self, reverse: bool) -> &mut Self {
        self.reverse = reverse; self
    }

    pub fn to_string(self) -> String {
        let mut s = String::new();
        
        s += &format!(
            "\x1b[{};{}m",
            self.fg.background(),
            self.bg.background()
        );
        
        if self.bold {
            s += "\x1b[1m";
        }
        if self.faint {
            s += "\x1b[2m";
        }
        if self.italic {
            s += "\x1b[3m";
        }
        if self.underline {
            s += "\x1b[4m";
        }
        if self.reverse {
            s += "\x1b[7m";
        }
        if self.strike {
            s += "\x1b[9m";
        }

        return s;
    }

    pub fn diff_to_string(self, other: Style) -> String {
        if self == other { return String::new(); }
        
        let mut s = "\x1b[".to_string();

        let mut prev: bool = false;

        if self.fg != other.fg {
            s += &self.fg.foreground();
            if self.bg != other.bg {
                s += ";";
                s += &self.bg.background();
            }
            prev = true;
        }
        else if self.bg != other.bg {
            s += &self.bg.background();
            prev = true;
        }
        
        if self.bold != other.bold {
            if prev { s += ";" }
            s += if self.bold {"1"} else {"22"};
            prev = true;
        }
        if self.faint != other.faint {
            if prev { s += ";" }
            s += if self.faint {"2"} else {"22"};
            prev = true;
        }
        if self.italic != other.italic {
            if prev { s += ";" }
            s += if self.italic {"3"} else {"23"};
            prev = true;
        }
        if self.underline != other.underline {
            if prev { s += ";" }
            s += if self.underline {"4"} else {"4"};
            prev = true;
        }
        if self.reverse != other.reverse {
            if prev { s += ";" }
            s += if self.reverse {"7"} else {"27"};
            prev = true;
        }
        if self.strike != other.strike {
            if prev { s += ";" }
            s += if self.strike {"9"} else {"29"};
            // prev = true;
        }

        return s+"m";
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Cell {
    pub c: char,
    pub s: Style,
}

impl Cell {
    fn empty() -> Cell {
        Cell {
            c: ' ',
            s: Style::default()
        }
    }
}

#[derive(Clone)]
pub struct Buff {
    pub cells: Vec<Cell>,
    pub width: u32,
    pub height: u32,
}

impl Buff {
    pub fn empty() -> Buff {
        let (Width(w),Height(h)) = terminal_size().unwrap();
        Buff {
            cells: vec![Cell::empty(); w as usize * h as usize],
            width: w as u32,
            height: h as u32,
        }
    }
    pub fn null() -> Buff {
        Buff {
            cells: Vec::new(),
            width: 0u32,
            height: 0u32
        }
    }
}

pub struct Renderer {
    pub backbuffer: Buff,
    pub buffer: Buff,
    pub cursor: Option<(u32, u32)>
}

#[derive(Clone)]
pub enum TextStyle {
    Style(Style),
    StyleVec(Vec<Style>, Option<Style>),
    StyleMap(fn(cell: &mut Cell, x: i32, y: i32, i: usize)->()),
}

#[derive(Clone, Builder)]
pub struct TextOptions {
    pub pos: (i32, i32),
    pub offset: Option<(i32, i32)>,
    pub text: String,
    pub max_w: Option<i32>,
    pub max_h: Option<i32>,
    pub wrap: Option<bool>,
    pub style: Option<TextStyle>,
}

impl TextOptions {
    pub fn idx_to_xy(self, width: u32, height: u32, i: usize) -> Option<(i32,i32)> {
        let wrap = self.wrap.unwrap_or(false);
        let (mut min_x, mut min_y) = self.pos;
        let (mut x, mut y) = ( min_x, min_y );
        let ( off_x, off_y ) = self.offset.unwrap_or((0i32,0i32));
        x += off_x;
        y += off_y;
        min_x = min_x.clamp(0, width as i32);
        min_y = min_y.clamp(0, height as i32);
        let max_x = (self.max_w.unwrap_or((width as i32)-x)+min_x).clamp(0, width as i32);
        let max_y = (self.max_h.unwrap_or((height as i32)-y)+min_y).clamp(0, height as i32);
        if max_x == 0 || max_y == 0 { return None; }
        let mut ci = 0usize;
        for c in self.text.chars() {
            if ci == i { 
                return 
                    if x >= min_x && x < max_x && y >= min_y && y < max_y
                        { Some((x,y)) } 
                    else 
                        { None }
                ; 
            }
            if (x >= max_x && wrap) || c == '\n' {
                x = min_x+off_x;
                y += 1;
                if y >= max_y {
                    break;
                }
            } else {
                x += 1;
            }
            ci += 1;
        }
        if ci == i && x >= min_x && x < max_x && y >= min_y && y < max_y
            { Some((x,y)) } 
        else 
            { None }
    }
}

impl Renderer {
    pub fn new() -> Renderer {
        Renderer {
            backbuffer: Buff::null(),
            buffer: Buff::empty(),
            cursor: None,
        }
    }
    
    /** Clears out the buffer and sets it to the appropriate size */
    pub fn clear(&mut self) {
        self.buffer = Buff::empty();
    }
    
    /** Updates the back buffer to the new buffer */
    pub fn flip(&mut self) {
        self.backbuffer = self.buffer.clone();
    }

    pub fn void(&mut self) {
        self.backbuffer = Buff::null();
    }
    
    /** Sets a cell at the provided coordinates */
    pub fn set(&mut self, x: u32, y: u32, cell: Cell) {
        self.buffer.cells[(x + y * self.buffer.width) as usize] = cell;
    }
    
    /** Puts a string at the given position with the given style */
    pub fn put_text(&mut self, x: u32, y: u32, text: String) {
        let mut i = (x + y * self.buffer.width) as usize;
        for c in text.chars() {
            if i+1 >= self.buffer.cells.len() { break; }
            self.buffer.cells[i].c = c;
            i += 1;
        }
    }

    /** Gets the cell at the given position */
    pub fn get(&mut self, x: u32, y: u32) -> Cell {
        return self.buffer.cells[(x + y * self.buffer.width) as usize];
    }

    /** Gets a mutable reference to the cell at the given position */
    pub fn get_mut(&mut self, x: u32, y: u32) -> &mut Cell {
        return &mut self.buffer.cells[(x + y * self.buffer.width) as usize];
    }

    /** Puts some text */
    pub fn put(&mut self, text: &TextOptions) {
        let wrap = text.wrap.unwrap_or(false);

        let (mut min_x, mut min_y) = text.pos;
        
        let (mut x, mut y) = ( min_x, min_y );
        
        let ( off_x, off_y ) = text.offset.unwrap_or((0i32,0i32));
        
        x += off_x;
        y += off_y;
        
        min_x = min_x.clamp(0, self.buffer.width as i32);
        min_y = min_y.clamp(0, self.buffer.height as i32);
        
        let max_x = (text.max_w.unwrap_or((self.buffer.width as i32)-x)+min_x).clamp(0, self.buffer.width as i32);
        let max_y = (text.max_h.unwrap_or((self.buffer.height as i32)-y)+min_y).clamp(0, self.buffer.height as i32);
        
        if max_x == 0 || max_y == 0 { return; }
        
        let mut ci = 0usize;
        
        for c in text.text.chars() {
            if (x >= max_x && wrap) || c == '\n' {
                x = min_x+off_x;
                y += 1;
                if y >= max_y {
                    break;
                }
            } else {
                if x >= min_x && x < max_x && y >= min_y && y < max_y {
                    let i = ((x as u32) + (y as u32) * self.buffer.width) as usize;
                    // self.buffer.cells[i].c = if !c.is_control() {c} else {' '};
                    self.buffer.cells[i].c = c;
                    if let Some(ref style) = text.style {
                        match style {
                            TextStyle::Style(s) => {
                                self.buffer.cells[i as usize].s = *s;
                            },
                            TextStyle::StyleVec(vec, default) => {
                                if let Some(s) = vec.get(ci).or(default.as_ref()) {
                                    self.buffer.cells[i].s = *s;
                                }
                            },
                            TextStyle::StyleMap(map) => {
                                map(&mut self.buffer.cells[i], x, y, i);
                            }
                        }
                    }
                }
                x += 1;
            }
            ci += 1;
        }
    }
    
    /** Sets the cells inside the provided rectangle */
    pub fn fill(&mut self, x: u32, y: u32, w: u32, h: u32, cell: Cell) {
        for xx in x..x+w {
            for yy in y..y+h {
                self.buffer.cells[(xx + yy * self.buffer.width) as usize] = cell;
            }
        }
    }
    
    /** Sets the style of the cells inside the provided rectangle */
    pub fn paint(&mut self, x: u32, y: u32, w: u32, h: u32, style: Style) {
        for xx in x..x+w {
            for yy in y..y+h {
                self.buffer.cells[(xx + yy * self.buffer.width) as usize].s = style;
            }
        }
    }

    /** Applies a function to all the cells in the provided rectangle */
    pub fn apply(&mut self, x: u32, y: u32, w: u32, h: u32, modifier: &dyn Fn(&mut Cell, u32, u32) -> ()) {
        for xx in x..x+w {
            for yy in y..y+h {
                modifier(&mut self.buffer.cells[(xx + yy * self.buffer.width) as usize],xx,yy);
            }
        }
    }
    
    /** Renders the current buffer to the screen, while optimizing the process to give the best render speeds */
    pub fn render(&mut self) {
        let mut buff = String::new();
        if self.buffer.width != self.backbuffer.width || self.buffer.height != self.backbuffer.height {
            let mut style = Style::default();
            for y in 0 .. self.buffer.height {
                for x in 0 .. self.buffer.width {
                    let cell = self.buffer.cells[(x + y * self.buffer.width) as usize];
                    buff += &cell.s.diff_to_string(style);
                    buff.push(cell.c);
                    style = cell.s;
                    if !cell.c.is_ascii() {
                        buff += &format!("\x1b[{}G",x+2);
                    }
                }
                if y < self.buffer.height-1 {
                    buff.push('\n');
                }
            }
        } else {
            let mut style = Style::default();
            for y in 0 .. self.buffer.height {
                let mut row = false;
                let mut streak = self.buffer.width;
                for x in 0 .. self.buffer.width {
                    let cell = self.buffer.cells[(x + y * self.buffer.width) as usize];
                    let bcell = self.backbuffer.cells[(x + y * self.buffer.width) as usize];
                    if cell != bcell {
                        if !row {
                            buff += &format!("\x1b[{};H",y+1);
                            row = true;
                        }
                        if streak+1 != x {
                            buff += &format!("\x1b[{}G",x+1);
                        }
                        streak = x;
                        buff += &cell.s.diff_to_string(style);
                        buff.push(cell.c);
                        style = cell.s;
                        if !cell.c.is_ascii() {
                            streak = self.buffer.width;
                        }
                    }
                }
            }
        }
        /* if buff.len() > 0 {
            println!("{}",buff.replace("\x1b", "\x1b[33m^\x1b[39m"));
        } */
        /*if let Some((x,y)) = self.cursor {
            buff += &format!("\x1b[?25h\x1b[{};{}H",y+1,x+1);
        } else {
            buff += "\x1b[?25l\x1b[H";
        }*/
        buff += "\x1b[m";
        stdout().write(buff.as_bytes()).unwrap();
        stdout().flush().unwrap();
    }
}