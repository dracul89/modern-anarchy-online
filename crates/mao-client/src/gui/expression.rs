//! Vendored from the `ao-utils` crate of the AnarchyReimagined drop-ins (its scanner and parser run from Utils.dll's
//! tables), over the client's own [`Variant`] and without its C runtime.
//!
//! Utils' expression language (`ExpressionParser_c::ParseExpression`): a
//! flex scanner and a bison parser (the yacc.c skeleton) computing a
//! [`Variant`], run from the tables Utils.dll carries ([`tables`], copied
//! from the DLL) by the skeletons' code as the DLL compiles it (`yylex` at
//! 100161E1, `yyparse` at 10015790, the buffer functions between).
//!
//! The tokens, read off the tables:
//!
//! - numbers: `[0-9]+` (`%d`, wrapping), `0x[0-9a-f]+` (lowercase only;
//!   `%x`), `[0-9]+.[0-9]+` (`%f`, a float);
//! - strings in double quotes, `\` escaping any one character (taken
//!   literally); a `\` at the very end of the input appends
//!   `Variant(true)` to the string (the original's `Variant("\\")` turned
//!   into the bool constructor; kept);
//! - `true`, `false`;
//! - variables: runs of letters, digits and `_ $ % : ?` (so `1x` is a
//!   variable, `1e5` too), looked up as they are scanned;
//! - `&& || == != >= <= << >>` and the single characters
//!   `( ) * + - / < > & | ^ ~`; spaces, tabs and newlines between tokens;
//! - anything else prints "Unknown character %02x" (the byte sign-extended)
//!   and goes to the parser as that character: a byte from 0x80 up is a
//!   negative token, which the parser takes as the end of the input.
//!
//! `^` is a power (`pow` on both sides as doubles, giving a double), `&&`
//! and `||` give bools from `AsBool`, comparisons bools, the rest
//! `Variant`'s operators. Precedence and associativity are the tables'.
//!
//! Like the original, the scanner and parser keep their state between
//! parses in globals, here a [`State`]: a string left open at the end of
//! one input is still open when the next one is scanned, and a variable
//! lookup that parses another expression (a context evaluating one)
//! restarts the shared scanner under the outer parse, which then sees the
//! end of its input. The state sits in a `RefCell` borrowed only between
//! such calls, so re-entering through [`Host::variable`] is allowed.

use std::cell::RefCell;
use std::collections::BTreeMap;

use super::variant::Variant;

fn lower(text: &mut [u8]) {
    text.make_ascii_lowercase();
}

/// `sscanf("%d")` / `("%x")` of a run of digits: wrapping, as the C runtime's conversion does.
fn scan_int(digits: &[u8], hex: bool) -> i32 {
    let base = if hex { 16 } else { 10 };
    digits.iter().map_while(|&c| (c as char).to_digit(base)).fold(0u32, |v, d| v.wrapping_mul(base).wrapping_add(d)) as i32
}

/// `sscanf("%f")` of `digits.digits`.
fn scan_float(text: &[u8]) -> f32 {
    std::str::from_utf8(c_text(text)).ok().and_then(|t| t.parse::<f64>().ok()).unwrap_or(0.0) as f32
}

mod tables;

use tables::*;

/// What a parse needs from its surroundings.
pub trait Host {
    /// A variable's value (`FUN_10016fdf`); `name` is the scanned text.
    /// It may parse another expression with the same state.
    fn variable(&mut self, name: &[u8]) -> Variant;

    /// A diagnostic the original prints with `printf("%s\n")` (given
    /// without its newline).
    fn print(&mut self, line: &[u8]);

    /// flex's `YY_FATAL_ERROR`: the original prints the message and a
    /// newline to stderr and exits with code 2.
    fn fatal(&mut self, message: &str) -> !;

    /// `_CIpow`, for `^`.
    fn pow(&mut self, x: f64, y: f64) -> f64 {
        x.powf(y)
    }
}

/// Tokens past the character range (`yylex`'s return values).
const VALUE: i32 = 0x102;
const OR: i32 = 0x103;
const AND: i32 = 0x104;
const EQ: i32 = 0x105;
const NE: i32 = 0x106;
const GE: i32 = 0x107;
const LE: i32 = 0x108;
const SHL: i32 = 0x109;
const SHR: i32 = 0x10a;

// ----------------------------------------------------------------- scanner

/// `YY_BUF_SIZE` and `YY_READ_BUF_SIZE`.
const BUF_SIZE: i32 = 0x4000;
const READ_BUF_SIZE: i32 = 0x2000;
/// The DFA's jam state, and the first template state (whose classes go
/// through `yy_meta`).
const JAM_STATE: i32 = 52;
const JAM_BASE: i16 = 82;
const FIRST_TEMPLATE: i32 = 53;
/// `YY_END_OF_BUFFER` and the first `YY_STATE_EOF`.
const END_OF_BUFFER: i16 = 24;
const STATE_EOF: i16 = 25;
/// The start states: `INITIAL` and inside a string.
const INITIAL: i32 = 1;
const IN_STRING: i32 = 3;

#[derive(Clone, Copy, PartialEq)]
enum BufferStatus {
    New,
    Normal,
    EofPending,
}

/// `struct yy_buffer_state`, with offsets for its pointers.
struct Buffer {
    /// `yy_ch_buf`: `buf_size` + 2 bytes.
    ch_buf: Vec<u8>,
    buf_pos: usize,
    buf_size: i32,
    n_chars: i32,
    /// `yy_is_our_buffer` (true: `yy_create_buffer` made it).
    is_our_buffer: bool,
    fill_buffer: bool,
    status: BufferStatus,
}

impl Buffer {
    /// `yy_create_buffer`, then `yy_init_buffer`.
    fn new(size: i32) -> Buffer {
        let mut b = Buffer {
            ch_buf: vec![0; size as usize + 2],
            buf_pos: 0,
            buf_size: size,
            n_chars: 0,
            is_our_buffer: true,
            fill_buffer: true,
            status: BufferStatus::New,
        };
        b.init();
        b
    }

    /// `yy_init_buffer` (the file and interactivity aside): flushed, filled
    /// on demand.
    fn init(&mut self) {
        self.flush();
        self.fill_buffer = true;
    }

    /// `yy_flush_buffer` (the caller reloads the state: the buffer is
    /// always the current one).
    fn flush(&mut self) {
        self.n_chars = 0;
        self.ch_buf[0] = 0;
        self.ch_buf[1] = 0;
        self.buf_pos = 0;
        self.status = BufferStatus::New;
    }
}

/// What `yy_get_next_buffer` found.
enum Eob {
    ContinueScan,
    EndOfFile,
    LastMatch,
}

/// flex's globals (offsets into the current buffer for its pointers).
struct Scanner {
    /// `yy_init`, `yy_start`.
    init: bool,
    start: i32,
    buffer: Option<Buffer>,
    hold_char: u8,
    c_buf_p: usize,
    text_ptr: usize,
    /// `yy_n_chars`.
    n_chars: i32,
    last_accepting_state: i32,
    last_accepting_cpos: usize,
}

/// One DFA step from `state` on character class `c`.
fn transition(mut state: i32, mut c: u8) -> i32 {
    while YY_CHK[(YY_BASE[state as usize] as i32 + i32::from(c)) as usize] as i32 != state {
        state = i32::from(YY_DEF[state as usize]);
        if state >= FIRST_TEMPLATE {
            c = YY_META[c as usize];
        }
    }
    i32::from(YY_NXT[(YY_BASE[state as usize] as i32 + i32::from(c)) as usize])
}

/// Where `yylex`'s code is (its labels).
enum Label {
    Match,
    FindAction,
    DoAction(i16),
}

/// What one `yylex` call ends with, the scanner's borrow released.
enum Step {
    Token(i32),
    /// Rule 11: the variable's value becomes `yylval`, the token VALUE.
    Variable(Vec<u8>),
    /// Rule 22: the message, then the character as the token.
    Unknown(i32),
    Fatal(&'static str),
}

/// The text up to its first NUL (where C string functions stop).
fn c_text(text: &[u8]) -> &[u8] {
    text.split(|&c| c == 0).next().unwrap_or(&[])
}

impl Scanner {
    const fn new() -> Scanner {
        Scanner {
            init: true,
            start: 0,
            buffer: None,
            hold_char: 0,
            c_buf_p: 0,
            text_ptr: 0,
            n_chars: 0,
            last_accepting_state: 0,
            last_accepting_cpos: 0,
        }
    }

    fn buffer(&mut self) -> &mut Buffer {
        self.buffer.as_mut().expect("the scanner has a buffer once started")
    }

    /// `yy_load_buffer_state`.
    fn load_buffer_state(&mut self) {
        let b = self.buffer.as_ref().expect("a current buffer");
        self.n_chars = b.n_chars;
        self.text_ptr = b.buf_pos;
        self.c_buf_p = b.buf_pos;
        self.hold_char = b.ch_buf[b.buf_pos];
    }

    /// `yyrestart`: the current buffer (created at the first call)
    /// emptied. The start condition is kept.
    fn restart(&mut self) {
        if self.buffer.is_none() {
            self.buffer = Some(Buffer::new(BUF_SIZE));
        }
        self.buffer().init();
        self.load_buffer_state();
    }

    /// `yy_get_previous_state`: rescans the text so far (a NUL as class 1).
    fn previous_state(&mut self) -> i32 {
        let mut state = self.start;
        for cp in self.text_ptr..self.c_buf_p {
            let byte = self.buffer().ch_buf[cp];
            let c = if byte != 0 { YY_EC[byte as usize] } else { 1 };
            if YY_ACCEPT[state as usize] != 0 {
                self.last_accepting_state = state;
                self.last_accepting_cpos = cp;
            }
            state = transition(state, c);
        }
        state
    }

    /// `yy_try_NUL_trans`: the state after a NUL that is text, or 0 when
    /// it jams.
    fn try_nul_transition(&mut self, state: i32) -> i32 {
        if YY_ACCEPT[state as usize] != 0 {
            self.last_accepting_state = state;
            self.last_accepting_cpos = self.c_buf_p;
        }
        let next = transition(state, 1);
        if next == JAM_STATE { 0 } else { next }
    }

    /// `yy_get_next_buffer`: moves the partial token to the buffer's start
    /// and reads more input after it (growing the buffer when the token
    /// fills it).
    fn next_buffer(&mut self, input: &[u8], read: &mut usize) -> Result<Eob, &'static str> {
        let (c_buf_p, text_ptr) = (self.c_buf_p, self.text_ptr);
        if c_buf_p as i64 > i64::from(self.n_chars) + 1 {
            return Err("fatal flex scanner internal error--end of buffer missed");
        }
        if !self.buffer().fill_buffer {
            return Ok(if c_buf_p - text_ptr == 1 { Eob::EndOfFile } else { Eob::LastMatch });
        }
        let number_to_move = c_buf_p as i32 - text_ptr as i32 - 1;
        let b = self.buffer.as_mut().expect("a current buffer");
        for i in 0..number_to_move.max(0) as usize {
            b.ch_buf[i] = b.ch_buf[text_ptr + i];
        }
        if b.status == BufferStatus::EofPending {
            b.n_chars = 0;
            self.n_chars = 0;
        } else {
            let mut num_to_read = b.buf_size - number_to_move - 1;
            while num_to_read <= 0 {
                if !b.is_our_buffer {
                    return Err("fatal error - scanner input buffer overflow");
                }
                let new_size = b.buf_size.wrapping_mul(2);
                if new_size <= 0 {
                    b.buf_size += b.buf_size / 8;
                } else {
                    b.buf_size *= 2;
                }
                b.ch_buf.resize(b.buf_size as usize + 2, 0);
                num_to_read = b.buf_size - number_to_move - 1;
            }
            num_to_read = num_to_read.min(READ_BUF_SIZE);
            // YY_INPUT (FUN_10016e61): the next bytes of the text.
            let count = (input.len() - *read).min(num_to_read as usize);
            let at = number_to_move as usize;
            b.ch_buf[at..at + count].copy_from_slice(&input[*read..*read + count]);
            *read += count;
            b.n_chars = count as i32;
            self.n_chars = count as i32;
        }
        let result = if self.n_chars == 0 {
            if number_to_move == 0 {
                self.restart();
                Eob::EndOfFile
            } else {
                self.buffer().status = BufferStatus::EofPending;
                Eob::LastMatch
            }
        } else {
            Eob::ContinueScan
        };
        self.n_chars += number_to_move;
        let n = self.n_chars as usize;
        let b = self.buffer();
        b.ch_buf[n] = 0;
        b.ch_buf[n + 1] = 0;
        self.text_ptr = 0;
        Ok(result)
    }
}

// ------------------------------------------------------------------- state

/// The scanner's and parser's globals.
pub struct State {
    scanner: Scanner,
    /// `yychar`, `yynerrs`, `yylval`.
    yychar: i32,
    yynerrs: i32,
    yylval: Variant,
    /// The last value an expression produced (`DAT_1002e778`):
    /// `ParseExpression` hands it out after a successful parse.
    pub result: Variant,
    /// The text being scanned (`DAT_1002e498`) and how much of it
    /// `YY_INPUT` has taken (`DAT_1002e770`).
    input: Vec<u8>,
    read: usize,
}

impl Default for State {
    fn default() -> State {
        State::new()
    }
}

impl State {
    pub const fn new() -> State {
        State {
            scanner: Scanner::new(),
            yychar: 0,
            yynerrs: 0,
            yylval: Variant::new(),
            result: Variant::new(),
            input: Vec::new(),
            read: 0,
        }
    }

    /// `ParseExpression`'s first steps: the text to scan, the scanner
    /// restarted on it.
    pub fn start(&mut self, text: &[u8]) {
        self.input = text.to_vec();
        self.read = 0;
        self.scanner.restart();
    }

    /// One `yylex` up to the point where it returns or needs the host.
    fn scan(&mut self) -> Step {
        let input = std::mem::take(&mut self.input);
        let step = self.scan_from(&input);
        self.input = input;
        step
    }

    fn scan_from(&mut self, input: &[u8]) -> Step {
        let s = &mut self.scanner;
        if s.init {
            s.init = false;
            if s.start == 0 {
                s.start = INITIAL;
            }
            if s.buffer.is_none() {
                s.buffer = Some(Buffer::new(BUF_SIZE));
            }
            s.load_buffer_state();
        }
        'tokens: loop {
            let mut cp = s.c_buf_p;
            let hold = s.hold_char;
            s.buffer().ch_buf[cp] = hold;
            let mut bp = cp;
            let mut state = s.start;
            let mut label = Label::Match;
            loop {
                match label {
                    Label::Match => {
                        loop {
                            let c = YY_EC[s.buffer().ch_buf[cp] as usize];
                            if YY_ACCEPT[state as usize] != 0 {
                                s.last_accepting_state = state;
                                s.last_accepting_cpos = cp;
                            }
                            state = transition(state, c);
                            cp += 1;
                            if YY_BASE[state as usize] == JAM_BASE {
                                break;
                            }
                        }
                        label = Label::FindAction;
                    }
                    Label::FindAction => {
                        let mut act = YY_ACCEPT[state as usize];
                        if act == 0 {
                            cp = s.last_accepting_cpos;
                            state = s.last_accepting_state;
                            act = YY_ACCEPT[state as usize];
                        }
                        // YY_DO_BEFORE_ACTION
                        s.text_ptr = bp;
                        s.hold_char = s.buffer().ch_buf[cp];
                        s.buffer().ch_buf[cp] = 0;
                        s.c_buf_p = cp;
                        label = Label::DoAction(act);
                    }
                    Label::DoAction(act) => {
                        let text_ptr = s.text_ptr;
                        let text = move |s: &mut Scanner| s.buffer().ch_buf[text_ptr..cp].to_vec();
                        match act {
                            // Back up to the last accepting state.
                            0 => {
                                let hold = s.hold_char;
                                s.buffer().ch_buf[cp] = hold;
                                cp = s.last_accepting_cpos;
                                state = s.last_accepting_state;
                                label = Label::FindAction;
                                continue;
                            }
                            1 | 2 => {
                                // The token is digits: sscanf always
                                // stores (into the value the original
                                // otherwise leaves as stack garbage).
                                let t = text(s);
                                let (digits, format) = if act == 1 { (&t[..], "%d") } else { (&t[2..], "%x") };
                                let value = scan_int(digits, format == "%x");
                                self.yylval.set_int32(value);
                                return Step::Token(VALUE);
                            }
                            3 => {
                                let value = scan_float(&text(s));
                                self.yylval.set_float(value);
                                return Step::Token(VALUE);
                            }
                            // An opening quote: an empty string to
                            // append to.
                            4 => {
                                self.yylval.set_string(b"");
                                s.start = IN_STRING;
                            }
                            5 => {
                                s.start = INITIAL;
                                return Step::Token(VALUE);
                            }
                            // `\x`: the character after the
                            // backslash, as a one-character string.
                            6 => {
                                let c = s.buffer().ch_buf[text_ptr + 1];
                                let piece = Variant::from(c_text(&[c]));
                                self.yylval = &self.yylval + &piece;
                            }
                            7 => {
                                let t = text(s);
                                let piece = Variant::from(c_text(&t));
                                self.yylval = &self.yylval + &piece;
                            }
                            // A lone backslash: `Variant("\\")` in
                            // the source, which is the bool
                            // constructor (kept).
                            8 => {
                                self.yylval = &self.yylval + &Variant::from(true);
                            }
                            9 | 10 => {
                                self.yylval.set_bool(act == 10);
                                return Step::Token(VALUE);
                            }
                            11 => return Step::Variable(c_text(&text(s)).to_vec()),
                            12 => return Step::Token(AND),
                            13 => return Step::Token(OR),
                            14 => return Step::Token(EQ),
                            15 => return Step::Token(NE),
                            16 => return Step::Token(GE),
                            17 => return Step::Token(LE),
                            18 => return Step::Token(SHL),
                            19 => return Step::Token(SHR),
                            20 => return Step::Token(i32::from(s.buffer().ch_buf[text_ptr] as i8)),
                            // White space.
                            21 => {}
                            22 => return Step::Unknown(i32::from(s.buffer().ch_buf[text_ptr] as i8)),
                            // The default rule under `nodefault`.
                            23 => return Step::Fatal("flex scanner jammed"),
                            END_OF_BUFFER => {
                                let matched = cp as i32 - s.text_ptr as i32 - 1;
                                let hold = s.hold_char;
                                s.buffer().ch_buf[cp] = hold;
                                if s.buffer().status == BufferStatus::New {
                                    s.n_chars = s.buffer().n_chars;
                                    s.buffer().status = BufferStatus::Normal;
                                }
                                if s.c_buf_p as i64 <= i64::from(s.n_chars) {
                                    // A NUL in the text.
                                    s.c_buf_p = (s.text_ptr as i32 + matched) as usize;
                                    state = s.previous_state();
                                    let next = s.try_nul_transition(state);
                                    bp = s.text_ptr;
                                    if next != 0 {
                                        s.c_buf_p += 1;
                                        cp = s.c_buf_p;
                                        state = next;
                                        label = Label::Match;
                                    } else {
                                        cp = s.c_buf_p;
                                        label = Label::FindAction;
                                    }
                                    continue;
                                }
                                match s.next_buffer(input, &mut self.read) {
                                    Err(message) => return Step::Fatal(message),
                                    Ok(Eob::EndOfFile) => {
                                        // yywrap() is 1.
                                        s.c_buf_p = s.text_ptr;
                                        label = Label::DoAction(STATE_EOF + ((s.start - 1) / 2) as i16);
                                    }
                                    Ok(Eob::ContinueScan) => {
                                        s.c_buf_p = (s.text_ptr as i32 + matched) as usize;
                                        state = s.previous_state();
                                        cp = s.c_buf_p;
                                        bp = s.text_ptr;
                                        label = Label::Match;
                                    }
                                    Ok(Eob::LastMatch) => {
                                        s.c_buf_p = s.n_chars as usize;
                                        state = s.previous_state();
                                        cp = s.c_buf_p;
                                        bp = s.text_ptr;
                                        label = Label::FindAction;
                                    }
                                }
                                continue;
                            }
                            // The end of the input, in either start
                            // state.
                            25 | 26 => return Step::Token(0),
                            _ => return Step::Fatal("fatal flex scanner internal error--no action found"),
                        }
                        continue 'tokens;
                    }
                }
            }
        }
    }
}

/// `yylex`, with the state borrowed only while scanning.
fn yylex(state: &RefCell<State>, host: &mut dyn Host) -> i32 {
    let step = state.borrow_mut().scan();
    match step {
        Step::Token(token) => token,
        Step::Variable(name) => {
            let value = host.variable(&name);
            state.borrow_mut().yylval = value;
            VALUE
        }
        Step::Unknown(c) => {
            host.print(format!("Unknown character {:02x}", c as u32).as_bytes());
            c
        }
        Step::Fatal(message) => host.fatal(message),
    }
}

// ------------------------------------------------------------------ parser

const YYEMPTY: i32 = -2;
const YYFINAL: u8 = 8;
const YYLAST: i32 = 0x73;
const YYPACT_NINF: i8 = -19;
const YYNTOKENS: u8 = 24;
const YYMAXUTOK: i32 = 0x10b;
const YYUNDEFTOK: i32 = 2;
const YYTERROR: i32 = 1;
/// `YYINITDEPTH`: the stacks are fixed arrays (bison can't grow them in
/// C++, its values not being relocatable).
const STACK_DEPTH: usize = 200;

/// `yyparse`: 0 when the input was an expression (its value in
/// [`State::result`]), 1 on a syntax error, 2 when the stack overflowed.
pub fn yyparse(state: &RefCell<State>, host: &mut dyn Host) -> i32 {
    enum Label {
        NewState,
        Backup,
        Default,
        Reduce(usize),
        ErrLab,
        ErrLab1,
    }
    let mut vs: Vec<Variant> = (0..STACK_DEPTH).map(|_| Variant::new()).collect();
    let mut ss = [0i16; STACK_DEPTH];
    let mut top = 0usize;
    let mut yystate: i32 = 0;
    let mut errstatus = 0;
    {
        let mut st = state.borrow_mut();
        st.yychar = YYEMPTY;
        st.yynerrs = 0;
    }
    let mut label = Label::NewState;
    loop {
        match label {
            Label::NewState => {
                ss[top] = yystate as i16;
                if top >= STACK_DEPTH - 1 {
                    host.print(b"parser stack overflow");
                    return 2;
                }
                label = Label::Backup;
            }
            Label::Backup => {
                let mut n = i32::from(YYPACT[yystate as usize]);
                if n == i32::from(YYPACT_NINF) {
                    label = Label::Default;
                    continue;
                }
                if state.borrow().yychar == YYEMPTY {
                    let token = yylex(state, host);
                    state.borrow_mut().yychar = token;
                }
                let mut st = state.borrow_mut();
                let token = if st.yychar <= 0 {
                    st.yychar = 0;
                    0
                } else if st.yychar <= YYMAXUTOK {
                    i32::from(YYTRANSLATE[st.yychar as usize])
                } else {
                    YYUNDEFTOK
                };
                n += token;
                if !(0..=YYLAST).contains(&n) || i32::from(YYCHECK[n as usize]) != token {
                    label = Label::Default;
                    continue;
                }
                let target = YYTABLE[n as usize];
                if target == 0 {
                    label = Label::ErrLab;
                    continue;
                }
                if target == YYFINAL {
                    return 0;
                }
                if st.yychar != 0 {
                    st.yychar = YYEMPTY;
                }
                top += 1;
                vs[top] = st.yylval.clone();
                if errstatus > 0 {
                    errstatus -= 1;
                }
                yystate = i32::from(target);
                label = Label::NewState;
            }
            Label::Default => {
                let rule = YYDEFACT[yystate as usize];
                label = if rule == 0 { Label::ErrLab } else { Label::Reduce(rule as usize) };
            }
            Label::Reduce(rule) => {
                let len = YYR2[rule] as usize;
                // `$$ = $1` by default.
                let mut yyval = vs[top + 1 - len].clone();
                let (a, b) = (top.wrapping_sub(2), top);
                let boolean = Variant::from;
                match rule {
                    2 => state.borrow_mut().result = vs[b].clone(),
                    3 => yyval = vs[b].clone(),
                    4 => yyval = &vs[a] + &vs[b],
                    5 => yyval = &vs[a] - &vs[b],
                    6 => yyval = &vs[a] * &vs[b],
                    7 => yyval = &vs[a] / &vs[b],
                    8 => yyval = &vs[a] & &vs[b],
                    9 => yyval = &vs[a] | &vs[b],
                    10 => yyval = -&vs[b],
                    11 => yyval = Variant::from(host.pow(vs[a].as_double(), vs[b].as_double())),
                    12 => yyval = &vs[a] << &vs[b],
                    13 => yyval = &vs[a] >> &vs[b],
                    14 => yyval = boolean(vs[a].as_bool() && vs[b].as_bool()),
                    15 => yyval = boolean(vs[a].as_bool() || vs[b].as_bool()),
                    16 => yyval = boolean(vs[a].less(&vs[b])),
                    17 => yyval = boolean(vs[a].greater(&vs[b])),
                    18 => yyval = boolean(vs[a].less_equal(&vs[b])),
                    19 => yyval = boolean(vs[a].greater_equal(&vs[b])),
                    20 => yyval = boolean(vs[a].equals(&vs[b])),
                    21 => yyval = boolean(vs[a].not_equals(&vs[b])),
                    // `( expr )`
                    22 => yyval = vs[b - 1].clone(),
                    _ => {}
                }
                top -= len;
                top += 1;
                vs[top] = yyval.clone();
                let symbol = (YYR1[rule] - YYNTOKENS) as usize;
                let below = i32::from(ss[top - 1]);
                let n = i32::from(YYPGOTO[symbol]) + below;
                yystate = if (0..=YYLAST).contains(&n) && i32::from(YYCHECK[n as usize]) == below {
                    i32::from(YYTABLE[n as usize])
                } else {
                    i32::from(YYDEFGOTO[symbol])
                };
                label = Label::NewState;
            }
            Label::ErrLab => {
                if errstatus == 0 {
                    state.borrow_mut().yynerrs += 1;
                    host.print(b"syntax error");
                }
                if errstatus == 3 {
                    let mut st = state.borrow_mut();
                    if st.yychar <= 0 {
                        if st.yychar == 0 {
                            return 1;
                        }
                    } else {
                        st.yychar = YYEMPTY;
                    }
                }
                label = Label::ErrLab1;
            }
            Label::ErrLab1 => {
                errstatus = 3;
                let target = loop {
                    let n = i32::from(YYPACT[yystate as usize]);
                    if n != i32::from(YYPACT_NINF) {
                        let n = n + YYTERROR;
                        if (0..=YYLAST).contains(&n) && i32::from(YYCHECK[n as usize]) == YYTERROR {
                            let target = YYTABLE[n as usize];
                            if target != 0 {
                                break target;
                            }
                        }
                    }
                    if top == 0 {
                        return 1;
                    }
                    top -= 1;
                    yystate = i32::from(ss[top]);
                };
                if target == YYFINAL {
                    return 0;
                }
                top += 1;
                vs[top] = state.borrow().yylval.clone();
                yystate = i32::from(target);
                label = Label::NewState;
            }
        }
    }
}

/// [`State::start`] then [`yyparse`]: whether the text was an expression,
/// its value then in [`State::result`] (`ParseExpression` without the
/// contexts and monitor).
pub fn parse(state: &RefCell<State>, text: &[u8], host: &mut dyn Host) -> bool {
    state.borrow_mut().start(text);
    yyparse(state, host) == 0
}

// --------------------------------------------------------------- constants

/// `AddConstant`'s table (a `std::map<std::string, Variant>` at
/// 1002E788): names lowercased, looked up after the contexts.
#[derive(Default)]
pub struct Constants(BTreeMap<Vec<u8>, Variant>);

impl Constants {
    pub const fn new() -> Constants {
        Constants(BTreeMap::new())
    }

    /// `AddConstant`: adds or replaces.
    pub fn add(&mut self, name: &[u8], value: &Variant) {
        let mut key = name.to_vec();
        lower(&mut key);
        self.0.insert(key, value.clone());
    }

    /// A variable no context knows (the rest of `FUN_10016fdf`): the
    /// constant, else "Unknown variable %s" printed and int 0.
    pub fn resolve(&self, name: &[u8], print: impl FnOnce(&[u8])) -> Variant {
        let mut key = c_text(name).to_vec();
        lower(&mut key);
        match self.0.get(&key) {
            Some(value) => value.clone(),
            None => {
                print(&[b"Unknown variable ", c_text(name)].concat());
                Variant::from(0i32)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::variant::TypeCode;

    struct TestHost {
        printed: Vec<Vec<u8>>,
        constants: Constants,
    }

    impl Host for TestHost {
        fn variable(&mut self, name: &[u8]) -> Variant {
            let printed = &mut self.printed;
            self.constants.resolve(name, |line| printed.push(line.to_vec()))
        }
        fn print(&mut self, line: &[u8]) {
            self.printed.push(line.to_vec());
        }
        fn fatal(&mut self, message: &str) -> ! {
            panic!("{message}")
        }
    }

    fn eval(text: &str) -> (Option<Variant>, Vec<Vec<u8>>) {
        let state = RefCell::new(State::new());
        let mut constants = Constants::default();
        constants.add(b"Ten", &Variant::from(10));
        let mut host = TestHost { printed: Vec::new(), constants };
        let ok = parse(&state, text.as_bytes(), &mut host);
        let result = state.borrow().result.clone();
        (ok.then_some(result), host.printed)
    }

    fn int(text: &str) -> i32 {
        let (v, printed) = eval(text);
        let v = v.unwrap_or_else(|| panic!("{text}: {printed:?}"));
        v.as_int32()
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(int("1+2*3"), 7);
        assert_eq!(int("(1+2)*3"), 9);
        assert_eq!(int("0x1f + 1"), 32);
        assert_eq!(int("ten * 2"), 20);
        assert_eq!(int("TEN - 1 - 1"), 8);
        assert_eq!(eval("2^10").0.unwrap().as_double(), 1024.0);
        assert_eq!(eval("1.5").0.unwrap().type_code(), TypeCode::Float);
        assert!(eval("1 < 2 && 3 > 2").0.unwrap().as_bool());
        assert_eq!(eval("\"ab\\\"c\"").0.unwrap().as_string(), b"ab\"c");
    }

    #[test]
    fn errors_print() {
        let (v, printed) = eval("1 +");
        assert!(v.is_none());
        assert_eq!(printed, vec![b"syntax error".to_vec()]);
        let (v, printed) = eval("nothing");
        assert_eq!(v.unwrap().as_int32(), 0);
        assert_eq!(printed, vec![b"Unknown variable nothing".to_vec()]);
        let (_, printed) = eval("1 @ 2");
        assert_eq!(printed[0], b"Unknown character 40");
        let deep = "(".repeat(300) + "1";
        let (v, printed) = eval(&deep);
        assert!(v.is_none());
        assert_eq!(printed, vec![b"parser stack overflow".to_vec()]);
    }
}
