//! FXS ("Fanatic", FXS.dll), the game's effect scripts: the text scripts of `cd_image/twk` read into objects of typed
//! variables whose values are formulas over other objects' variables, worked out when asked (at most once a frame), each
//! object's `Heartbeat` worked out every frame and an object ended when its `Use` reaches 0 (`e_Kill`). Ported from
//! `AnarchyReimagined`'s `dropin/fxs` (`parser.rs`, `objclass.rs`, `object.rs`, `variable.rs`, `runtime.rs`,
//! `formula.rs`, `eval.rs`, `lazy.rs`, `stateblob.rs`, `fxs.rs`), as safe Rust over `Rc` objects in place of the
//! original's reference-counted C++ ones.
//!
//! - `FXS_t` ([`Fxs`] here, one a thread as the original's singleton): the named objects (by name ID) and the `Noname`
//!   ones, read by `ReadFromScript`; `LoadPFTweakData` (`Tweak_Playfield_<n>`), `ExecuteScript(text, spawner)`, the frame
//!   (`FrameProcess`) and the automatic-run queries (`Use` 3).
//! - `FXS_ObjectClass_t` ([`Class`]): the children by name ID, the variables (the base game's and two expansions',
//!   merged by `SetExpansion`), ten state blobs an expansion, `Heartbeat`, `Use`, the parent and the spawner.
//! - `FXS_Variable_t` ([`Variable`]): a value of a type (1 string, 2 unsigned, 3 signed, 4 float, 5 double, 6 vector,
//!   7 quaternion, 8 matrix, 9 sound) or an array of them, read at an index, at a float position (interpolated) or at a
//!   double one (a vector chart: by distance along the points).
//! - `FXS_Runtime<Type>_t` ([`Rt`]): a literal (`Simple`) or a formula with an initial value's formula, worked out on
//!   the owner's call stack.
//! - `FXS_RuntimeFormula_Base_t` ([`Formula`]): values and operators in turn, stepped by FUN_10005600 ([`step`]).
//! - `FXS_LazyPointer_t` ([`Lazy`]): a variable by path (`This`, `Parent`, `Spawner` or a named object, a child of it,
//!   the variable; an index variable; a selector `<Current>`, `<X>`...), found when first asked.
//!
//! Gamecode's side is here too: the slot `SignalFXSScript` calls (0x100b2dfc: the text run as a script with the sender
//! as its spawner, and an object with an `FXID` started as that environment effect), and the game interface's variables
//! (DisplaySystem's `GameInterfaceObject_t`, FUN_1005cdcf: `GAME`'s `GameDeltaTime`, `SystemTime`, ... written each frame).
//!
//! Not here: the state blobs' rendering (they are read and kept), the effects the `FXID`s start (listed in
//! [`Fxs::started`] for the renderer), the debugger.

pub mod enums;
pub mod math;
pub mod parser;
pub mod clouds;
pub mod planet;
pub mod sun;
pub mod visuals;

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::{Rc, Weak};

use math::{fistp, fmod, r32, round, sqrt, u2f, M16, Q4, V3};
use parser::{name_id, Parser};

/// A sound's parameters (FUN_1000a2bf): the ID, position, the delay, volume and the rest `PlayGameSound` takes, the
/// velocity, and whether to play.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sound {
    pub id: u32,
    pub pos: V3,
    pub f10: f32,
    pub volume: f32,
    pub i18: i32,
    pub f1c: f32,
    pub f20: f32,
    pub f24: f32,
    pub f28: f32,
    pub vel: V3,
    pub play: bool,
}

/// The value a formula works out (FUN_10008eb5's record): its type (1 unsigned, 2 signed, 3 float, 4 double, 5 vector,
/// 6 quaternion, 7 matrix, 8 sound) and a field per type.
#[derive(Clone, Copy, Debug)]
pub struct Value {
    pub kind: i32,
    pub u: u32,
    pub s: i32,
    pub f: f32,
    pub d: f64,
    pub v: V3,
    pub q: Q4,
    pub m: M16,
    pub snd: Sound,
}

impl Value {
    pub fn new(kind: i32) -> Value {
        Value { kind, u: 0, s: 0, f: 0.0, d: 0.0, v: [0.0; 3], q: [0.0, 0.0, 0.0, 1.0], m: math::IDENTITY, snd: Sound::default() }
    }
}

/// A formula's entry: an operator (its number) or a value (an unsigned or signed number, a float, a double's float, a
/// vector, a lazy pointer — none when its path had one name or over six).
enum Entry {
    Op(i32),
    U(u32),
    S(i32),
    F(f32),
    D(f32),
    V(V3),
    Lazy(Option<Box<Lazy>>),
}

/// `FXS_RuntimeFormula_Base_t`: its entries, worked out at most once a frame (its next frame and interval, 1).
#[derive(Default)]
struct Formula {
    next_frame: u32,
    interval: u32,
    entries: Vec<Entry>,
}

/// A runtime value: its variable type (2 to 9), a literal or a formula, its last value, an initial value's formula
/// (worked out once, first), the parts of a vector, quaternion (an axis and degrees when `axis_angle`), matrix or sound
/// built from formulas, and the object whose variable holds it.
pub struct Rt {
    kind: i32,
    simple: bool,
    value: Value,
    formula: Formula,
    initial: Option<RtRef>,
    parts: Vec<RtRef>,
    axis_angle: bool,
    owner: Weak<ObjCell>,
}

pub type RtRef = Rc<RefCell<Rt>>;

/// The value's type in a formula (the variable's type less one).
fn acc_kind(var_kind: i32) -> i32 {
    var_kind - 1
}

impl Rt {
    fn new(kind: i32) -> Rt {
        Rt { kind, simple: false, value: Value::new(acc_kind(kind)), formula: Formula { next_frame: 0, interval: 1, entries: Vec::new() }, initial: None, parts: Vec::new(), axis_angle: false, owner: Weak::new() }
    }

    fn simple(kind: i32, value: Value) -> RtRef {
        let mut r = Rt::new(kind);
        r.simple = true;
        r.value = value;
        Rc::new(RefCell::new(r))
    }
}

/// `FXS_Variable_t`: a named value of a type, or an array of them (`count` elements; 0 for one value).
pub struct Variable {
    pub name: u32,
    kind: Cell<i32>,
    count: u32,
    values: Vec<RtRef>,
    string: Option<Vec<u8>>,
    /// The vector chart's position (in elements) and distance.
    chart: Cell<(f32, f64)>,
}

/// An object: `FXS_Object_t`, whose class is gone (`None`) once it is ended.
pub struct ObjCell {
    pub name: u32,
    class: RefCell<Option<Class>>,
}

pub type Obj = Rc<ObjCell>;

/// A state blob as a script configures it (`FXS_StateBlob_t`): what it sets, kept for the renderer.
#[derive(Default)]
pub struct StateBlob {
    pub criteria: Vec<RtRef>,
    pub targets: Vec<[u32; 2]>,
    pub materials: Vec<(u32, RefCell<Option<Box<Lazy>>>)>,
    /// A mesh's place and the object (its blueprint) by name ID.
    pub meshes: Vec<(u32, u32)>,
    pub textures: Vec<(u32, u32, Vec<u8>)>,
    pub stages: Vec<[u32; 4]>,
    pub transforms: Vec<(u32, u32, RtRef)>,
    pub states: Vec<[u32; 3]>,
    pub special: Option<RtRef>,
}

/// `FXS_ObjectClass_t`.
#[derive(Default)]
pub struct Class {
    children: BTreeMap<u32, Obj>,
    variables: [BTreeMap<u32, Rc<Variable>>; 3],
    blobs: [[Option<Rc<StateBlob>>; 10]; 3],
    heartbeat: Option<Rc<Variable>>,
    use_: Option<Rc<Variable>>,
    parent: Weak<ObjCell>,
    spawner: Option<Obj>,
}

/// `FXS_LazyPointer_t`.
pub struct Lazy {
    /// The object, child (0 for none) and variable IDs, until resolved.
    path: Option<[u32; 3]>,
    object: Option<Obj>,
    variable: Option<Rc<Variable>>,
    index: Option<Box<Lazy>>,
    selector: u32,
}

/// The CRT's `rand()`, FXS.dll's (msvcr100: the state times 214013 plus 2531011, bits 16 to 30), seeded 1.
struct Rand(u32);

impl Rand {
    fn next(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
        ((self.0 >> 16) & 0x7fff) as i32
    }
}

/// The engine's state: `FXS_t`'s, the lazy pointers' call stack and variable status, the frame counter, and what the
/// host takes each frame.
pub struct Fxs {
    data_path: PathBuf,
    expansion: u32,
    named: BTreeMap<u32, Obj>,
    unnamed: Vec<Obj>,
    /// The objects' call stack (100 deep, `PushCallstack`) and how deep it is.
    callstack: Vec<Obj>,
    depth: i32,
    status: u32,
    /// The frame counter `FrameProcess` advances (DAT_10018564).
    frame: u32,
    rand: Rand,
    /// The sounds the scripts played (FUN_10005560), the texts sent (`[SEND]`) with the object sending them, the effects
    /// the signal's slot started (an `FXID` and its object), the prints and the errors.
    pub sounds: Vec<Sound>,
    sends: Vec<(Vec<u8>, Obj)>,
    pub started: Vec<(u32, Obj)>,
    pub prints: Vec<String>,
    pub errors: Vec<String>,
    /// The last script event's object (Gamecode's FUN_1002a15e holder).
    current: Option<Obj>,
}

thread_local! {
    static FXS: RefCell<Fxs> = RefCell::new(Fxs {
        data_path: PathBuf::new(),
        expansion: 0,
        named: BTreeMap::new(),
        unnamed: Vec::new(),
        callstack: Vec::new(),
        depth: 0,
        status: 0,
        frame: 0,
        rand: Rand(1),
        sounds: Vec::new(),
        sends: Vec::new(),
        started: Vec::new(),
        prints: Vec::new(),
        errors: Vec::new(),
        current: None,
    });
}

/// The engine, briefly (never held while a value is worked out).
pub fn with<R>(f: impl FnOnce(&mut Fxs) -> R) -> R {
    FXS.with(|e| f(&mut e.borrow_mut()))
}

fn id(name: &str) -> u32 {
    name_id(name.as_bytes())
}

// --- the call stack and the variable status (lazy.rs) ---

/// `PushCallstack(object)`.
fn push_callstack(o: &Obj) {
    with(|e| {
        if e.depth < 100 {
            e.callstack.truncate(e.depth as usize);
            e.callstack.push(o.clone());
        }
        e.depth += 1;
    });
}

/// `PopCallstack()`.
fn pop_callstack() {
    with(|e| {
        if e.depth > 0 {
            e.depth -= 1;
            if (e.depth as usize) < e.callstack.len() {
                e.callstack.truncate(e.depth as usize);
            }
        }
    });
}

/// `GetFromCallstack(n)`.
fn from_callstack(n: i32) -> Option<Obj> {
    with(|e| {
        let at = e.depth - n - 1;
        (0..100).contains(&at).then(|| e.callstack.get(at as usize).cloned()).flatten()
    })
}

fn status() -> u32 {
    with(|e| e.status)
}

fn set_status(s: u32) {
    with(|e| e.status = s);
}

// --- objects (objclass.rs, object.rs) ---

impl ObjCell {
    fn new(name: u32) -> Obj {
        Rc::new(ObjCell { name, class: RefCell::new(Some(Class::default())) })
    }

    /// Whether its class is still there.
    pub fn alive(&self) -> bool {
        self.class.borrow().is_some()
    }

    /// `GetVariable(id)` (the base game's).
    pub fn variable(&self, id: u32) -> Option<Rc<Variable>> {
        self.class.borrow().as_ref()?.variables[0].get(&id).cloned()
    }

    /// `GetVariablePointer(name)`.
    pub fn variable_named(&self, name: &str) -> Option<Rc<Variable>> {
        self.variable(id(name))
    }

    /// `GetChild(id)`.
    pub fn child(&self, id: u32) -> Option<Obj> {
        self.class.borrow().as_ref()?.children.get(&id).cloned()
    }

    pub fn child_named(&self, name: &str) -> Option<Obj> {
        self.child(id(name))
    }

    fn parent(&self) -> Option<Obj> {
        self.class.borrow().as_ref()?.parent.upgrade()
    }

    fn spawner(&self) -> Option<Obj> {
        self.class.borrow().as_ref()?.spawner.clone()
    }

    /// A state blob (the base game's slot `k`).
    pub fn blob(&self, k: usize) -> Option<Rc<StateBlob>> {
        self.class.borrow().as_ref()?.blobs[0].get(k)?.clone()
    }
}

/// FUN_10003941: the class deleted (its children's first).
fn delete_class(o: &Obj) {
    let class = o.class.borrow_mut().take();
    if let Some(c) = class {
        for child in c.children.values() {
            delete_class(child);
        }
    }
}

/// `CleanObject()`: the children (their classes deleted), variables, spawner and state blobs gone.
fn clean_object(o: &Obj) {
    let children: Vec<Obj> = o.class.borrow().as_ref().map(|c| c.children.values().cloned().collect()).unwrap_or_default();
    for child in &children {
        delete_class(child);
    }
    if let Some(c) = o.class.borrow_mut().as_mut() {
        *c = Class::default();
    }
}

/// `SetVariable(variable, expansion)`: owned by the object, kept by its ID (replacing one there); `Heartbeat` and `Use`
/// noted.
fn set_variable(o: &Obj, v: Rc<Variable>, expansion: usize) {
    for rt in &v.values {
        set_owner(rt, o);
    }
    let mut class = o.class.borrow_mut();
    let Some(c) = class.as_mut() else { return };
    if v.name == id("Heartbeat") {
        c.heartbeat = Some(v.clone());
    }
    if v.name == id("Use") {
        c.use_ = Some(v.clone());
    }
    c.variables[expansion].insert(v.name, v);
}

/// FUN_10002a4d: the owner set on a value (its parts and initial value are worked out under it).
fn set_owner(rt: &RtRef, o: &Obj) {
    rt.borrow_mut().owner = Rc::downgrade(o);
}

/// `SetExpansion(expansion)`: the expansions' variables (up to 2) moved into the base game's, their state blobs over its.
fn class_set_expansion(o: &Obj, expansion: u32) {
    for e in 1..=expansion.min(2) as usize {
        let (vars, blobs) = {
            let mut class = o.class.borrow_mut();
            let Some(c) = class.as_mut() else { return };
            (std::mem::take(&mut c.variables[e]), std::mem::replace(&mut c.blobs[e], Default::default()))
        };
        for v in vars.into_values() {
            set_variable(o, v, 0);
        }
        let mut class = o.class.borrow_mut();
        let Some(c) = class.as_mut() else { return };
        for (i, b) in blobs.into_iter().enumerate() {
            if b.is_some() {
                c.blobs[0][i] = b;
            }
        }
    }
}

/// `ReadFromScript(parser, expansion)`: a `{ ... }` block of variables, `StateBlob`s, child `Object`s and `Expansion
/// BP01`/`EP01` blocks.
fn class_read(o: &Obj, p: &mut Parser, expansion: usize) -> bool {
    if !p.read_brace() {
        return false;
    }
    loop {
        if !p.read_type(true) {
            return false;
        }
        let kind = p.kind;
        if kind < 1 {
            continue;
        }
        if kind < 10 {
            let Some(name) = p.read_name() else { return false };
            match variable_read(p, name, kind) {
                Some(v) if v.kind.get() != 0 => set_variable(o, Rc::new(v), expansion),
                _ => return false,
            }
            continue;
        }
        match kind {
            10 => {
                if expansion != 0 {
                    p.error("You are not allowed to define an expansion inside an expansion!");
                    return false;
                }
                let Some(name) = p.read_text() else { return false };
                if name == b"BP01" && !class_read(o, p, 1) {
                    return false;
                }
                // (the token is compared as the original compares it: the parser's buffer, overwritten by the block just read)
                let now = if name == b"BP01" { p.token.clone() } else { name };
                if now == b"EP01" && !class_read(o, p, 2) {
                    return false;
                }
            }
            0xb => {
                let Some((index, blob)) = blob_read(o, p) else { return false };
                if let Some(c) = o.class.borrow_mut().as_mut() {
                    c.blobs[expansion][index] = Some(Rc::new(blob));
                }
            }
            0x13 => {
                let Some(name) = p.read_name() else { return false };
                let existing = o.child(name);
                let child = match existing {
                    Some(c) => {
                        clean_object(&c);
                        c
                    }
                    None => {
                        let c = ObjCell::new(name);
                        if let Some(cl) = o.class.borrow_mut().as_mut() {
                            cl.children.insert(name, c.clone());
                        }
                        c
                    }
                };
                if let Some(cl) = child.class.borrow_mut().as_mut() {
                    cl.parent = Rc::downgrade(o);
                }
                if !class_read(&child, p, 0) {
                    clean_object(&child);
                    return false;
                }
            }
            0x15 => return true,
            _ => {}
        }
    }
}

/// `Heartbeat` worked out, then the children's (`RunHeartbeat`).
fn run_heartbeat(o: &Obj) {
    let (h, children) = match o.class.borrow().as_ref() {
        Some(c) => (c.heartbeat.clone(), c.children.values().cloned().collect::<Vec<_>>()),
        None => return,
    };
    if let Some(h) = h {
        word(&h);
    }
    for child in children {
        if child.alive() {
            run_heartbeat(&child);
        }
    }
}

/// FUN_1000105e: whether an object's `Use` has reached 0.
fn used_up(o: &Obj) -> bool {
    let u = o.class.borrow().as_ref().and_then(|c| c.use_.clone());
    u.is_some_and(|u| word(&u) == 0)
}

// --- the readers (variable.rs, runtime.rs, formula.rs, stateblob.rs) ---

/// `ReadFromScript(parser, type)`: the value (or, after `[size]`, that many elements) read.
fn variable_read(p: &mut Parser, name: u32, kind: i32) -> Option<Variable> {
    let mut v = Variable { name, kind: Cell::new(kind), count: 0, values: Vec::new(), string: None, chart: Cell::new((0.0, 0.0)) };
    if p.next_kind == parser::ARRAY {
        let size = p.read_array_size()?;
        if size == 0 {
            let message = if kind == 6 {
                if !p.next() || p.kind == parser::VECTOR_CHART {
                    return None;
                }
                "Vector-arrays without known size must be defined by VectorChart."
            } else {
                "Only vector-arrays can be defined with unknown size."
            };
            p.error(message);
            return None;
        }
        v.count = size as u32;
        match kind {
            1 => {
                p.error("String-arrays is not supported.");
                return None;
            }
            2..=9 => {
                for _ in 0..size as u32 {
                    v.values.push(read_runtime(p, kind)?);
                }
            }
            _ => {}
        }
        return Some(v);
    }
    match kind {
        1 => v.string = Some(p.read_text()?),
        2..=9 => v.values.push(read_runtime(p, kind)?),
        _ => {}
    }
    Some(v)
}

/// The reader of each runtime type (unsigned to sound).
fn read_runtime(p: &mut Parser, kind: i32) -> Option<RtRef> {
    match kind {
        2..=4 => read_scalar(p, kind),
        5 => read_double(p),
        6 => read_vector(p),
        7 => read_quaternion(p),
        8 => read_matrix(p),
        _ => read_sound(p),
    }
}

/// FUN_10008a11: a formula read: values (unsigned, signed, float and double literals, `v(` vectors, capitalised names as
/// lazy pointers) and the operators between them, until a value isn't followed by one. `dynamic` is set when a value
/// isn't of the `wanted` type or `~` (random) appears: such a formula is worked out each frame.
fn formula_parse(f: &mut Formula, p: &mut Parser, dynamic: &mut bool, wanted: i32) -> bool {
    *dynamic = false;
    while let Some(t) = p.read_text() {
        let (kind, entry) = match parser::literal_type(&t) {
            0 => (1, Entry::U(parser::unsigned_value(p, &t))),
            1 => (2, Entry::S(parser::signed_value(&t))),
            2 => (3, Entry::F(parser::float_value(&t))),
            3 => (4, Entry::D(parser::double_value(&t))),
            4 => (5, Entry::V(p.read_vector())),
            8 => (9, Entry::Lazy(parse_path(&t).map(Box::new))),
            _ => break,
        };
        f.entries.push(entry);
        if wanted != kind {
            *dynamic = true;
        }
        if p.next_kind != parser::OPERATOR {
            return true;
        }
        let Some(t) = p.read_text() else {
            p.error("Expected operator!");
            return false;
        };
        let op = parser::operator(&t);
        f.entries.push(Entry::Op(op));
        if op == 5 {
            *dynamic = true;
        }
    }
    p.error("Expected value!");
    false
}

/// The scalar readers (FUN_1000a89e, FUN_1000a076, FUN_10008fd7) and the double's (FUN_10008d9d): a formula, with
/// `= formula` an initial value; a literal of its value when it can't change.
fn read_scalar(p: &mut Parser, kind: i32) -> Option<RtRef> {
    let wanted = acc_kind(kind);
    let c = Rc::new(RefCell::new(Rt::new(kind)));
    let mut dynamic = false;
    if !formula_parse(&mut c.borrow_mut().formula, p, &mut dynamic, wanted) {
        return None;
    }
    if p.next_kind == parser::ASSIGN {
        p.next();
        let i = Rc::new(RefCell::new(Rt::new(kind)));
        if !formula_parse(&mut i.borrow_mut().formula, p, &mut dynamic, wanted) {
            return None;
        }
        dynamic = true;
        c.borrow_mut().initial = Some(i);
    }
    if !dynamic {
        let v = work(&c);
        return Some(Rt::simple(kind, v));
    }
    Some(c)
}

fn read_double(p: &mut Parser) -> Option<RtRef> {
    read_scalar(p, 5)
}

/// A float that can't change (`IsSimple`, a float's slot 2).
fn is_simple(rt: &RtRef) -> bool {
    rt.borrow().simple
}

/// The parts read, `n` floats.
fn read_parts(p: &mut Parser, c: &RtRef, n: usize) -> Option<bool> {
    let mut simple = true;
    for _ in 0..n {
        let f = read_scalar(p, 4)?;
        simple &= is_simple(&f);
        c.borrow_mut().parts.push(f);
    }
    Some(simple)
}

/// A formula starting with a name, with `=` an initial value (the vector's, quaternion's and matrix's).
fn read_named(p: &mut Parser, kind: i32) -> Option<RtRef> {
    let c = Rc::new(RefCell::new(Rt::new(kind)));
    let mut dynamic = false;
    if !formula_parse(&mut c.borrow_mut().formula, p, &mut dynamic, acc_kind(kind)) {
        return None;
    }
    if p.next_kind != parser::ASSIGN {
        return Some(c);
    }
    p.next();
    let i = Rc::new(RefCell::new(Rt::new(kind)));
    if !formula_parse(&mut i.borrow_mut().formula, p, &mut dynamic, acc_kind(kind)) {
        return None;
    }
    c.borrow_mut().initial = Some(i);
    Some(c)
}

/// FUN_1000abc3: `v( x y z )` (a literal when the three are), or a formula starting with a name.
fn read_vector(p: &mut Parser) -> Option<RtRef> {
    match parser::literal_type(&p.lookahead) {
        4 => {
            p.next();
            let c = Rc::new(RefCell::new(Rt::new(6)));
            let simple = read_parts(p, &c, 3)?;
            p.next();
            if !simple {
                return Some(c);
            }
            let v = work(&c);
            Some(Rt::simple(6, v))
        }
        8 => read_named(p, 6),
        _ => {
            p.error("Vector function must start with pointer to vector or vector define.");
            None
        }
    }
}

/// FUN_10009c3b: `v( x y z ) degrees` (an axis and an angle), `q( x y z w )` (each a literal when all four parts are),
/// or a formula starting with a name.
fn read_quaternion(p: &mut Parser) -> Option<RtRef> {
    match parser::literal_type(&p.lookahead) {
        4 => {
            p.next();
            let c = Rc::new(RefCell::new(Rt::new(7)));
            let mut simple = read_parts(p, &c, 3)?;
            p.next();
            simple &= read_parts(p, &c, 1)?;
            c.borrow_mut().axis_angle = true;
            if !simple {
                return Some(c);
            }
            let v = work(&c);
            Some(Rt::simple(7, v))
        }
        5 => {
            p.next();
            let c = Rc::new(RefCell::new(Rt::new(7)));
            let simple = read_parts(p, &c, 4)?;
            p.next();
            if !simple {
                return Some(c);
            }
            let v = work(&c);
            Some(Rt::simple(7, v))
        }
        8 => read_named(p, 7),
        _ => {
            p.error("Quaternion function must start with pointer to quaternion, quaternion define or vector define.");
            None
        }
    }
}

/// FUN_1000936d: `m( 16 floats )` or a formula starting with a name.
fn read_matrix(p: &mut Parser) -> Option<RtRef> {
    match parser::literal_type(&p.lookahead) {
        6 => {
            p.next();
            let c = Rc::new(RefCell::new(Rt::new(8)));
            let simple = read_parts(p, &c, 16)?;
            p.next();
            if !simple {
                return Some(c);
            }
            let v = work(&c);
            Some(Rt::simple(8, v))
        }
        8 => read_named(p, 8),
        _ => {
            p.error("Matrix function must start with pointer to Matrix or Matrix define.");
            None
        }
    }
}

/// FUN_1000a462: `s( Name position f f unsigned f f f f velocity )` or a formula starting with a name (an initial value
/// reported, not read).
fn read_sound(p: &mut Parser) -> Option<RtRef> {
    match parser::literal_type(&p.lookahead) {
        7 => {
            let c = Rc::new(RefCell::new(Rt::new(9)));
            p.next();
            let name = p.read_name()?;
            let mut n = Value::new(1);
            n.u = name;
            c.borrow_mut().parts.push(Rt::simple(2, n));
            let readers: [fn(&mut Parser) -> Option<RtRef>; 9] =
                [read_vector, read_f, read_f, read_u, read_f, read_f, read_f, read_f, read_vector];
            for read in readers {
                let r = read(p)?;
                c.borrow_mut().parts.push(r);
            }
            p.next();
            Some(c)
        }
        8 => {
            let c = Rc::new(RefCell::new(Rt::new(9)));
            let mut dynamic = false;
            if !formula_parse(&mut c.borrow_mut().formula, p, &mut dynamic, 8) {
                return None;
            }
            if p.next_kind == parser::ASSIGN {
                p.error("Initial value not supported for sound variables.\n");
            }
            Some(c)
        }
        _ => {
            p.error("Sound function must start with pointer to Sound or Sound define.");
            None
        }
    }
}

fn read_f(p: &mut Parser) -> Option<RtRef> {
    read_scalar(p, 4)
}

fn read_u(p: &mut Parser) -> Option<RtRef> {
    read_scalar(p, 2)
}

/// FUN_1000dac8: a render argument: a type statement and an unsigned, signed or float value (a float that can change is
/// marked complex).
fn argument(p: &mut Parser, complex: &mut bool) -> Option<u32> {
    *complex = false;
    if !p.read_type(true) {
        return None;
    }
    let r = match p.kind {
        2 => read_scalar(p, 2)?,
        3 => read_scalar(p, 3)?,
        4 => {
            let r = read_scalar(p, 4)?;
            if !is_simple(&r) {
                *complex = true;
                return Some(0);
            }
            return Some(work(&r).f.to_bits());
        }
        _ => {
            p.errors.push("FXS_ScriptParser.cpp:1401: error: Unsupported type for D3D argument!".into());
            return None;
        }
    };
    let v = work(&r);
    Some(if v.kind == 2 { v.s as u32 } else { v.u })
}

/// FUN_10003229: a state blob block (its index read first).
fn blob_read(o: &Obj, p: &mut Parser) -> Option<(usize, StateBlob)> {
    let t = p.read_text()?;
    let index = parser::unsigned_value(p, &t);
    if index >= 10 {
        p.error("State-blob index is out of range.");
        return None;
    }
    if !p.read_brace() {
        return None;
    }
    let mut b = StateBlob::default();
    let mut complex = false;
    let fail = |p: &mut Parser, m: &str| -> Option<(usize, StateBlob)> {
        p.error(m);
        None
    };
    while p.read_type(true) {
        match p.kind - 0xc {
            0 => {
                let a = argument(p, &mut complex)?;
                let c = argument(p, &mut complex)?;
                if c == 0x3c {
                    let u = read_scalar(p, 2)?;
                    set_owner(&u, o);
                    b.special = Some(u);
                } else {
                    let d = argument(p, &mut complex)?;
                    if complex {
                        return fail(p, "Complex render state not supported yet!");
                    }
                    b.states.push([a, c, d]);
                }
            }
            1 => {
                let mut s = [0u32; 4];
                for v in &mut s {
                    *v = argument(p, &mut complex)?;
                }
                if complex {
                    return fail(p, "Complex texture stage state not supported yet!");
                }
                b.stages.push(s);
            }
            2 => {
                let a = argument(p, &mut complex)?;
                if !p.read_type(true) {
                    return None;
                }
                if p.kind != parser::OBJECT {
                    return fail(p, "Type not supported by stateblob mesh configuration.");
                }
                let name = p.read_name()?;
                b.meshes.push((a, name));
            }
            3 => {
                let a = argument(p, &mut complex)?;
                let c = argument(p, &mut complex)?;
                if !p.read_type(true) {
                    return None;
                }
                if p.kind != parser::STRING {
                    return fail(p, "Type not supported by stateblob texture configuration.");
                }
                let name = p.read_text()?;
                b.textures.push((a, c, name));
            }
            4 => {
                let a = argument(p, &mut complex)?;
                let c = argument(p, &mut complex)?;
                if !p.read_type(true) {
                    return None;
                }
                if p.kind != 8 {
                    return fail(p, "Type not supported by stateblob transform configuration.");
                }
                let m = read_matrix(p)?;
                b.transforms.push((a, c, m));
            }
            5 => {
                let a = argument(p, &mut complex)?;
                let name = p.read_text()?;
                b.materials.push((a, RefCell::new(parse_path(&name).map(Box::new))));
            }
            6 => {
                let a = argument(p, &mut complex)?;
                let c = argument(p, &mut complex)?;
                b.targets.push([a, c]);
            }
            8 => {
                let u = read_scalar(p, 2)?;
                set_owner(&u, o);
                b.criteria.push(u);
            }
            9 => return Some((index as usize, b)),
            _ => return fail(p, "Unsupported state-blob type."),
        }
    }
    None
}

/// FUN_1000cb00: a lazy pointer from a token like `Object.Variable`, `Object.Child.Variable`,
/// `Object.Variable[Other.Index]` (two to six names), with `<Selector>` naming a component; none for one name or over six.
fn parse_path(text: &[u8]) -> Option<Lazy> {
    let mut t = text.to_vec();
    let mut count = 1usize;
    let mut at = None;
    for k in 0..t.len() {
        match t[k] {
            b'.' | b'[' => {
                count += 1;
                t[k] = b' ';
            }
            b']' | b'>' => t[k] = b' ',
            b'<' => {
                at = Some(k + 1);
                t[k] = b' ';
            }
            _ => {}
        }
    }
    if !(2..=6).contains(&count) {
        return None;
    }
    let words: Vec<u32> = t.split(|&c| c == b' ').filter(|w| !w.is_empty()).map(name_id).collect();
    let n = |k: usize| words.get(k).copied().unwrap_or(0);
    let lazy = |path: [u32; 3], index: Option<Lazy>| Lazy { path: Some(path), object: None, variable: None, index: index.map(Box::new), selector: 0 };
    let mut lp = match count {
        2 => lazy([n(0), 0, n(1)], None),
        3 => lazy([n(0), n(1), n(2)], None),
        4 => lazy([n(0), 0, n(1)], Some(lazy([n(2), 0, n(3)], None))),
        5 => lazy([n(0), n(1), n(2)], Some(lazy([n(3), 0, n(4)], None))),
        _ => lazy([n(0), n(1), n(2)], Some(lazy([n(3), n(4), n(5)], None))),
    };
    if let Some(at) = at {
        let word: Vec<u8> = t[at..].iter().copied().skip_while(|&c| c == b' ').take_while(|&c| c != b' ').collect();
        lp.set_in_variable(name_id(&word));
    }
    Some(lp)
}

impl Lazy {
    /// `SetInVariable(id)`: the selector of `Current`, `X`, `Y`, `Z` and `CurrentX` to `CurrentZ`.
    fn set_in_variable(&mut self, name: u32) {
        let names = [("Current", 0x8000_0000), ("X", 1), ("Y", 2), ("Z", 3), ("CurrentX", 0x8000_0001), ("CurrentY", 0x8000_0002), ("CurrentZ", 0x8000_0003)];
        if let Some((_, s)) = names.iter().find(|(n, _)| id(n) == name) {
            self.selector = *s;
        }
    }

    /// `GetPointer()`: the variable, its path resolved the first time (from the call stack's top for `This`, its parent
    /// for `Parent`, its top parent's spawner for `Spawner`, else the object of that ID) and then dropped; none once the
    /// object held has lost its class.
    fn pointer(&mut self) -> Option<Rc<Variable>> {
        if self.object.as_ref().is_some_and(|o| !o.alive()) {
            return None;
        }
        if self.variable.is_some() || self.path.is_none() {
            return self.variable.clone();
        }
        let [object, child, var] = self.path.take()?;
        let obj = if object == id("This") {
            from_callstack(0)
        } else if object == id("Spawner") {
            from_callstack(0).and_then(|mut p| {
                while let Some(up) = p.parent() {
                    p = up;
                }
                p.spawner()
            })
        } else if object != id("Parent") {
            with(|e| e.named.get(&object).cloned())
        } else {
            from_callstack(0).and_then(|p| p.parent())
        };
        if let Some(o) = obj.as_ref().filter(|o| o.alive()) {
            self.variable = if child == 0 { o.variable(var) } else { o.child(child).and_then(|c| c.variable(var)) };
        }
        self.object = obj;
        self.variable.clone()
    }

    /// `GetIndexPointer()`.
    fn index_pointer(&mut self) -> Option<Rc<Variable>> {
        self.index.as_mut()?.pointer()
    }
}

// --- working values out (runtime.rs, formula.rs) ---

/// FUN_10008bd3: whether to work the formula out now (once its frame has come, unless the variable status is negative),
/// its next frame set.
fn due(f: &mut Formula) -> bool {
    let (frame, status) = with(|e| (e.frame, e.status));
    if f.next_frame <= frame && (status as i32) >= 0 {
        f.next_frame = f.interval.wrapping_add(frame);
        return true;
    }
    false
}

/// The integer of a worked-out value (unsigned and signed alike).
fn integer(acc: &Value) -> u32 {
    match acc.kind {
        1 => acc.u,
        2 => acc.s as u32,
        3 => round(acc.f) as u32,
        4 => round(r32(acc.d)) as u32,
        _ => 0,
    }
}

/// The value worked out: its initial value's first (once, on the owner's call stack with the variable status 0), then
/// its formula when due (on the owner's call stack: unsigned FUN_1000a9b7, signed FUN_1000a18e, float FUN_100090f5,
/// double FUN_10008c9d, vector FUN_1000adc9, quaternion FUN_10009a76, matrix FUN_1000966a, sound FUN_1000a616).
fn work(rt: &RtRef) -> Value {
    if rt.borrow().simple {
        return rt.borrow().value;
    }
    let (kind, owner) = {
        let r = rt.borrow();
        (r.kind, r.owner.upgrade())
    };
    let initial = rt.borrow_mut().initial.take();
    if let Some(i) = initial {
        if let Some(o) = &owner {
            push_callstack(o);
        }
        let saved = status();
        set_status(0);
        let v = work(&i);
        set_status(saved);
        if let Some(_o) = &owner {
            pop_callstack();
        }
        store(&mut rt.borrow_mut().value, kind, &v);
    }
    let go = due(&mut rt.borrow_mut().formula);
    if go {
        if let Some(o) = &owner {
            push_callstack(o);
        }
        let mut acc = Value::new(acc_kind(kind));
        let parts = rt.borrow().parts.clone();
        let axis_angle = rt.borrow().axis_angle;
        match kind {
            6 if !parts.is_empty() => acc.v = [work(&parts[0]).f, work(&parts[1]).f, work(&parts[2]).f],
            7 if !parts.is_empty() => {
                let [x, y, z, w] = [0, 1, 2, 3].map(|k| work(&parts[k]).f);
                acc.q = if axis_angle {
                    let axis = [x, y, z];
                    let axis = if math::is_zero(&axis) { [0.0, 1.0, 0.0] } else { math::with_len(&axis, 1.0) };
                    math::axis_angle(&axis, r32(w as f64 * std::f32::consts::PI as f64 / 180.0))
                } else {
                    [x, y, z, w]
                };
            }
            8 if !parts.is_empty() => {
                // (worked out last to first, as the original does)
                let mut m = [0f32; 16];
                for k in (0..16).rev() {
                    m[k] = work(&parts[k]).f;
                }
                acc.m = m;
            }
            9 => {
                acc.snd = if parts.len() < 10 {
                    Sound::default()
                } else {
                    let vel = work(&parts[9]).v;
                    let f28 = work(&parts[8]).f;
                    let f24 = work(&parts[7]).f;
                    let f20 = work(&parts[6]).f;
                    let f1c = work(&parts[5]).f;
                    let i18 = work(&parts[4]).u;
                    let volume = work(&parts[3]).f;
                    let f10 = work(&parts[2]).f;
                    let pos = work(&parts[1]).v;
                    let id = work(&parts[0]).u;
                    Sound { id, pos, f10, volume, i18: i18 as i32, f1c, f20, f24, f28, vel, play: true }
                };
            }
            _ => {}
        }
        let mut entries = std::mem::take(&mut rt.borrow_mut().formula.entries);
        run(&mut entries, &mut acc);
        rt.borrow_mut().formula.entries = entries;
        store(&mut rt.borrow_mut().value, kind, &acc);
        if let Some(_o) = &owner {
            pop_callstack();
        }
    }
    rt.borrow().value
}

/// A worked-out value kept as the runtime type keeps it.
fn store(to: &mut Value, kind: i32, acc: &Value) {
    match kind {
        2 => to.u = integer(acc),
        3 => to.s = integer(acc) as i32,
        4 => {
            to.f = match acc.kind {
                1 => r32(u2f(acc.u)),
                2 => r32(acc.s as f64),
                3 => acc.f,
                4 => r32(acc.d),
                _ => 0.0,
            }
        }
        5 => {
            to.d = match acc.kind {
                1 => u2f(acc.u),
                2 => acc.s as f64,
                3 => acc.f as f64,
                4 => acc.d,
                _ => 0.0,
            }
        }
        6 => to.v = if acc.kind == 5 { acc.v } else { [0.0; 3] },
        7 => to.q = if acc.kind == 6 { acc.q } else { [0.0, 0.0, 0.0, 1.0] },
        8 => to.m = if acc.kind == 7 { acc.m } else { math::IDENTITY },
        9 => to.snd = if acc.kind == 8 { acc.snd } else { Sound::default() },
        _ => {}
    }
}

/// FUN_10008914: the formula worked out into `acc`, each value applied with the operator before it (none before the
/// first), until a step returns false.
fn run(entries: &mut [Entry], acc: &mut Value) {
    let mut op = 0;
    let mut k = 0;
    while k < entries.len() {
        if !step(acc, op, &mut entries[k]) {
            return;
        }
        k += 1;
        let Some(Entry::Op(next)) = entries.get(k) else { return };
        op = *next;
        k += 1;
    }
}

// --- reading variables (variable.rs) ---

/// FUN_10003ca5: an unsigned or signed variable's value.
pub fn word(v: &Variable) -> u32 {
    v.values.first().map_or(0, |rt| {
        let x = work(rt);
        if v.kind.get() == 3 { x.s as u32 } else { x.u }
    })
}

fn element(v: &Variable, i: u32) -> Option<&RtRef> {
    v.values.get(i as usize)
}

/// The element at position `t` (0 to 1 along the array): its index, the next one (wrapping to the first) and the
/// fraction between them.
fn position(v: &Variable, t: f32) -> (u32, u32, f32) {
    let n = v.count;
    let s = r32(u2f(n) * t as f64);
    let mut i = fistp(s as f64 - 0.49999) as u32;
    if i >= n {
        i = n.wrapping_sub(1);
    }
    let next = if i.wrapping_add(1) < n { i + 1 } else { 0 };
    let frac = r32(s as f64 - u2f(i));
    (i, next, frac)
}

fn at(v: &Variable, i: u32) -> Value {
    element(v, i).map_or(Value::new(acc_kind(v.kind.get())), work)
}

/// FUN_10003d24.
fn float_at_t(v: &Variable, t: f32) -> f32 {
    let (i, next, frac) = position(v, t);
    let a = at(v, next).f;
    let b = at(v, i).f;
    let k = (a as f64 - b as f64) * frac as f64;
    let c = at(v, i).f;
    r32(c as f64 + k)
}

/// FUN_10003dbe (the doubles' last values).
fn double_at_t(v: &Variable, t: f32) -> f64 {
    let (i, next, frac) = position(v, t);
    let d = |i: u32| element(v, i).map_or(0.0, |r| r.borrow().value.d);
    (d(next) - d(i)) * frac as f64 + d(i)
}

/// FUN_10003e31.
fn vector_at_t(v: &Variable, t: f32) -> V3 {
    let (i, next, frac) = position(v, t);
    let cur1 = at(v, i).v;
    let cur2 = at(v, i).v;
    let nxt = at(v, next).v;
    math::add(&math::scale(&math::sub(&nxt, &cur2), frac), &cur1)
}

/// FUN_10002728: the point at distance `d` along a vector array, walked on from the last distance (back to the start
/// for a shorter one), a step (an element, the first to the next whole one) at a time, wrapping by the distance modulo
/// the last point's.
fn vector_chart(v: &Variable, mut d: f64) -> V3 {
    let n = u2f(v.count);
    let (mut pos, mut dist) = v.chart.get();
    let delta = r32(d - dist);
    if 0.0 > delta {
        dist = 0.0;
        pos = 0.0;
    } else if delta == 0.0 {
        return vector_at_t(v, r32(pos as f64 / n));
    }
    let t = fistp(pos as f64 - 0.49999).wrapping_add(1) as u32;
    let mut step = r32(u2f(t) - pos as f64);
    if step <= 0.0 {
        step = 1e-5;
    }
    loop {
        let a = vector_at_t(v, r32(pos as f64 / n));
        let b = vector_at_t(v, r32((pos as f64 + step as f64) / n));
        let mut len = math::len2(&math::sub(&b, &a));
        if 0.0 < len {
            len = r32(sqrt(len as f64));
        }
        let e = len as f64 + dist;
        if d == e {
            v.chart.set((r32(pos as f64 + step as f64), d));
            return b;
        }
        if d < e {
            let into = r32(d - dist);
            pos = r32(into as f64 * step as f64 / len as f64 + pos as f64);
            v.chart.set((pos, d));
            return vector_at_t(v, r32(pos as f64 / n));
        }
        dist = e;
        pos = r32(pos as f64 + step as f64);
        step = 1.0;
        if n <= pos as f64 {
            d = fmod(d, e);
            pos = 0.0;
        }
        v.chart.set((pos, dist));
    }
}

/// FUN_10003cb4: a sound variable's sound played, then worked out again and returned.
fn sound_of(rt: &RtRef) -> Sound {
    let s = work(rt).snd;
    play_sound(&s);
    work(rt).snd
}

/// FUN_10005560: the sound played through SandyInterface when it asks to be and is louder than 0.01.
fn play_sound(s: &Sound) {
    if s.play && 0.01f32 < s.volume {
        with(|e| e.sounds.push(*s));
    }
}

// --- the formula's step (eval.rs) ---

/// The operand: the word (the number or float bits), a double, a vector, a quaternion, a matrix, a sound, and a string
/// variable's text.
struct Operand {
    kind: i32,
    w: u32,
    d: f64,
    v: V3,
    q: Q4,
    m: M16,
    snd: Sound,
    name: Option<Vec<u8>>,
}

fn d(x: f32) -> f64 {
    x as f64
}

/// The original's rounding of a double to an integer (through a float).
fn round_d(x: f64) -> i32 {
    round(r32(x))
}

/// The float of an arithmetic step rounded to an integer.
fn fround(x: f64) -> i32 {
    fistp(d(r32(x)) - 0.49999)
}

fn rand() -> i32 {
    with(|e| e.rand.next())
}

/// `rand() % (to - from) + from`, unsigned.
fn rand_u(from: u32, to: u32) -> u32 {
    let r = rand() as u32;
    r.checked_rem(to.wrapping_sub(from)).unwrap_or(0).wrapping_add(from)
}

/// `rand() % (to - from) + from`, signed.
fn rand_s(from: i32, to: i32) -> i32 {
    let r = rand();
    r.checked_rem(to.wrapping_sub(from)).unwrap_or(0).wrapping_add(from)
}

/// `rand() % 10000 / 10000`.
fn rand_frac() -> f64 {
    (rand() % 10000) as f64 / 10000.0
}

/// The operand's value from a lazy pointer's variable (none: the original returns false here, the variable status left
/// as set): its value, an element by an index variable, or at a float or double position.
fn from_variable(o: &mut Operand, lp: &mut Lazy) -> Option<()> {
    let var = lp.pointer()?;
    let saved = status();
    set_status(lp.selector);
    let idx = lp.index_pointer();
    let vk = var.kind.get();
    match idx {
        None => {
            if vk == 1 {
                o.kind = 1;
                let text = var.string.clone().unwrap_or_default();
                o.w = name_id(&text);
                o.name = Some(text);
            } else {
                let rt = var.values.first()?;
                match vk {
                    2 => (o.kind, o.w) = (1, work(rt).u),
                    3 => (o.kind, o.w) = (2, work(rt).s as u32),
                    4 => (o.kind, o.w) = (3, work(rt).f.to_bits()),
                    // (a double variable reads its last value)
                    5 => (o.kind, o.d) = (4, rt.borrow().value.d),
                    6 => {
                        o.kind = 5;
                        o.v = work(rt).v;
                        component(o);
                    }
                    7 => (o.kind, o.q) = (6, work(rt).q),
                    8 => (o.kind, o.m) = (7, work(rt).m),
                    9 => (o.kind, o.snd) = (8, sound_of(rt)),
                    _ => return None,
                }
            }
        }
        Some(idx) => match idx.kind.get() {
            2 | 3 => {
                let i = word(&idx);
                let e = |i: u32| element(&var, i).cloned();
                let zero = Value::new(acc_kind(vk));
                match vk {
                    2 => (o.kind, o.w) = (1, e(i).map_or(0, |r| work(&r).u)),
                    3 => (o.kind, o.w) = (2, e(i).map_or(0, |r| work(&r).s as u32)),
                    4 => (o.kind, o.w) = (3, e(i).map_or(0.0, |r| work(&r).f).to_bits()),
                    5 => (o.kind, o.d) = (4, e(i).map_or(0.0, |r| r.borrow().value.d)),
                    6 => {
                        o.kind = 5;
                        o.v = e(i).map_or(zero.v, |r| work(&r).v);
                        component(o);
                    }
                    7 => (o.kind, o.q) = (6, e(i).map_or(zero.q, |r| work(&r).q)),
                    8 => (o.kind, o.m) = (7, e(i).map_or(zero.m, |r| work(&r).m)),
                    9 => (o.kind, o.snd) = (8, e(i).map_or(Sound::default(), |r| sound_of(&r))),
                    _ => return None,
                }
            }
            4 => {
                let t = idx.values.first().map_or(0.0, |r| work(r).f);
                match vk {
                    // (an integer at a position is 0)
                    2 => (o.kind, o.w) = (1, 0),
                    3 => (o.kind, o.w) = (2, 0),
                    4 => (o.kind, o.w) = (3, float_at_t(&var, t).to_bits()),
                    5 => (o.kind, o.d) = (4, double_at_t(&var, t)),
                    6 => {
                        o.kind = 5;
                        o.v = vector_at_t(&var, t);
                        component(o);
                    }
                    7 => (o.kind, o.q) = (6, [0.0, 1.0, 0.0, 0.0]),
                    8 => (o.kind, o.m) = (7, math::IDENTITY),
                    9 => (o.kind, o.snd) = (8, Sound::default()),
                    _ => return None,
                }
            }
            5 => {
                let t = idx.values.first().map_or(0.0, |r| r.borrow().value.d);
                match vk {
                    2 => (o.kind, o.w) = (1, 0),
                    3 => (o.kind, o.w) = (2, 0),
                    4 => (o.kind, o.w) = (3, 0),
                    5 => (o.kind, o.d) = (4, 0.0),
                    6 => {
                        o.kind = 5;
                        o.v = vector_chart(&var, t);
                        component(o);
                    }
                    7 => (o.kind, o.q) = (6, [0.0, 1.0, 0.0, 0.0]),
                    8 => (o.kind, o.m) = (7, math::IDENTITY),
                    9 => (o.kind, o.snd) = (8, Sound::default()),
                    _ => return None,
                }
            }
            // (the operand stays the lazy pointer)
            _ => {}
        },
    }
    set_status(saved);
    Some(())
}

/// A vector operand's component, when the variable status names one.
fn component(o: &mut Operand) {
    let c = match status() & 0x7fff_ffff {
        1 => 0,
        2 => 1,
        3 => 2,
        _ => return,
    };
    o.w = o.v[c].to_bits();
    o.kind = 3;
}

/// FUN_10005600: one step of a formula, the operator `op` applied to the value worked out so far (`acc`) and the next
/// entry. `[TRIGGER]` goes on only while the value so far is nonzero; `[STORE]` writes the value into the variable;
/// `[SEND]` sends the string variable's text from the top of the calling object's parents; `[PRINT]` prints it; `[RUN]`
/// and `[AFCM]` only work their operand out. Otherwise the operator by the two types, as the original's. False ends the
/// formula.
fn step(acc: &mut Value, mut op: i32, entry: &mut Entry) -> bool {
    if op == 0x17 {
        let go = match acc.kind {
            1 => acc.u != 0,
            2 => acc.s != 0,
            3 => acc.f != 0.0,
            4 => acc.d != 0.0,
            _ => false,
        };
        if !go {
            return false;
        }
        op = 0;
    }
    if op == 0x19 {
        let Entry::Lazy(Some(lp)) = entry else { return false };
        let Some(var) = lp.pointer() else { return false };
        if lp.index_pointer().is_some() {
            return false;
        }
        let want = acc.kind + 1;
        // (an array's elements are not one value: the original writes over their list there)
        if var.kind.get() == want && var.count == 0 {
            if let Some(rt) = var.values.first() {
                let mut r = rt.borrow_mut();
                match acc.kind {
                    1 => r.value.u = acc.u,
                    2 => r.value.s = acc.s,
                    3 => r.value.f = acc.f,
                    4 => r.value.d = acc.d,
                    5 => r.value.v = acc.v,
                    6 => r.value.q = acc.q,
                    7 => r.value.m = acc.m,
                    8 => r.value.snd = acc.snd,
                    _ => {}
                }
            }
        }
        return true;
    }
    let mut o = Operand { kind: 0, w: 0, d: 0.0, v: [0.0; 3], q: [0.0, 0.0, 0.0, 1.0], m: math::IDENTITY, snd: Sound::default(), name: None };
    match entry {
        Entry::Lazy(lp) => {
            o.kind = 9;
            let Some(lp) = lp else { return false };
            if from_variable(&mut o, lp).is_none() {
                return false;
            }
        }
        Entry::U(u) => (o.kind, o.w) = (1, *u),
        Entry::S(s) => (o.kind, o.w) = (2, *s as u32),
        Entry::F(f) => (o.kind, o.w) = (3, f.to_bits()),
        Entry::D(f) => (o.kind, o.d) = (4, d(*f)),
        Entry::V(v) => (o.kind, o.v) = (5, *v),
        Entry::Op(_) => return true,
    }
    if o.kind == 8 {
        play_sound(&o.snd);
    }
    match op {
        0x18 | 0x1a => return true,
        0x1b => {
            send(o.name);
            return true;
        }
        0x1c => {
            print(acc, o.name.as_deref());
            return true;
        }
        _ => {}
    }
    match acc.kind {
        1 => unsigned(acc, op, &o),
        2 => signed(acc, op, &o),
        3 => float(acc, op, &o),
        4 => double(acc, op, &o),
        5 => vector(acc, op, &o),
        6 => quaternion(acc, op, &o),
        7 => matrix(acc, op, &o),
        8 => sound(acc, op, &o),
        _ => {}
    }
    true
}

/// `[SEND]`: the text sent (`SignalFXSScript`) with the top of the calling object's parents.
fn send(name: Option<Vec<u8>>) {
    let Some(mut obj) = from_callstack(0) else { return };
    if !obj.alive() {
        return;
    }
    while let Some(parent) = obj.parent().filter(|p| p.alive()) {
        obj = parent;
    }
    if let Some(text) = name {
        with(|e| e.sends.push((text, obj)));
    }
}

fn print(acc: &Value, name: Option<&[u8]>) {
    let name = String::from_utf8_lossy(name.unwrap_or_default()).into_owned();
    let line = match acc.kind {
        1 => format!("{name}: {}", acc.u),
        2 => format!("{name}: {}", acc.s),
        3 => format!("{name}: {:.6}", acc.f),
        4 => format!("{name}: {:.6}", r32(acc.d)),
        5 => format!("{name}: v( {:.6} {:.6} {:.6} )", acc.v[0], acc.v[1], acc.v[2]),
        6 => format!("{name}: q( {:.6} {:.6} {:.6} {:.6} )", acc.q[0], acc.q[1], acc.q[2], acc.q[3]),
        7 => "Matrix print not supported yet.".into(),
        8 => "Sound print not supported yet.".into(),
        _ => return,
    };
    with(|e| e.prints.push(line));
}

/// An assignment of a vector, quaternion, matrix or sound operand (op 0), or its comparisons (`==` false, `<>` true,
/// orderings false), shared by the scalar types.
fn aggregate(acc: &mut Value, op: i32, o: &Operand, eq_keeps_kind: bool) {
    match op {
        0 => assign(acc, o),
        0x11 => {
            if !eq_keeps_kind {
                acc.kind = 1;
            }
            acc.u = 0;
        }
        0x12 => (acc.kind, acc.u) = (1, 1),
        0x13..=0x16 => (acc.kind, acc.u) = (1, 0),
        _ => {}
    }
}

/// The operand assigned whole: the type becomes its type.
fn assign(acc: &mut Value, o: &Operand) {
    acc.kind = o.kind;
    match o.kind {
        1 => acc.u = o.w,
        2 => acc.s = o.w as i32,
        3 => acc.f = f32::from_bits(o.w),
        4 => acc.d = o.d,
        5 => acc.v = o.v,
        6 => acc.q = o.q,
        7 => acc.m = o.m,
        8 => acc.snd = o.snd,
        _ => {}
    }
}

fn cmp_u(acc: &mut Value, op: i32, a: u32, b: u32, set_kind_on_eq: bool) {
    let r = match op {
        0x11 => a == b,
        0x12 => a != b,
        0x13 => a < b,
        0x14 => b < a,
        0x15 => b >= a,
        0x16 => a >= b,
        _ => return,
    };
    if op != 0x11 || set_kind_on_eq {
        acc.kind = 1;
    }
    acc.u = r as u32;
}

fn cmp_s(acc: &mut Value, op: i32, a: i32, b: i32) {
    let r = match op {
        0x11 => a == b,
        0x12 => a != b,
        0x13 => a < b,
        0x14 => a > b,
        0x15 => a <= b,
        0x16 => a >= b,
        _ => return,
    };
    acc.kind = 1;
    acc.u = r as u32;
}

fn cmp_f(acc: &mut Value, op: i32, a: f64, b: f64) {
    let r = match op {
        0x11 => a == b,
        0x12 => a != b,
        0x13 => a < b,
        0x14 => a > b,
        0x15 => a <= b,
        0x16 => a >= b,
        _ => return,
    };
    acc.kind = 1;
    acc.u = r as u32;
}

/// With an unsigned value so far.
fn unsigned(acc: &mut Value, op: i32, o: &Operand) {
    let a = acc.u;
    match o.kind {
        1 | 2 => {
            let v = o.w;
            match op {
                0 if o.kind == 1 => acc.u = v,
                0 => (acc.kind, acc.s) = (2, v as i32),
                1 => acc.u = a.wrapping_add(v),
                2 => acc.u = a.wrapping_sub(v),
                3 => acc.u = a.wrapping_mul(v),
                4 => acc.u = a.checked_div(v).unwrap_or(0),
                5 if v != a => acc.u = rand_u(a, v),
                6 => acc.u = if v == 0 { 0 } else { a % v },
                7 if a > v => acc.u = v,
                8 if a < v => acc.u = v,
                0xe => acc.u = a | v,
                0xf => acc.u = a & v,
                0x10 => acc.u = a ^ v,
                _ => cmp_u(acc, op, a, v, false),
            }
        }
        3 => {
            let f = f32::from_bits(o.w);
            let r = round(f) as u32;
            match op {
                0 => (acc.kind, acc.f) = (3, f),
                1 => acc.u = fround(u2f(a) + d(f)) as u32,
                2 => acc.u = fround(u2f(a) - d(f)) as u32,
                3 => acc.u = fround(u2f(a) * d(f)) as u32,
                4 => acc.u = if f == 0.0 { 0 } else { fround(u2f(a) / d(f)) as u32 },
                5 => {
                    if r != a {
                        acc.u = rand_u(a, r);
                    }
                }
                6 => acc.u = if f == 0.0 { 0 } else { a.checked_rem(r).unwrap_or(0) },
                7 if a > r => acc.u = r,
                8 if a < r => acc.u = r,
                // (`|` and `&` swap for a float operand, as the original's)
                0xe => acc.u = a & r,
                0xf => acc.u = a | r,
                0x10 => acc.u = a ^ r,
                _ => cmp_u(acc, op, a, r, false),
            }
        }
        4 => {
            let x = o.d;
            let r = round_d(x) as u32;
            match op {
                0 => (acc.kind, acc.d) = (4, x),
                1 => acc.u = fround(u2f(a) + x) as u32,
                2 => acc.u = fround(u2f(a) - x) as u32,
                3 => acc.u = fround(u2f(a) * x) as u32,
                4 => acc.u = if x == 0.0 { 0 } else { fround(u2f(a) / x) as u32 },
                5 => {
                    if r != a {
                        acc.u = rand_u(a, r);
                    }
                }
                6 => acc.u = if x == 0.0 { 0 } else { a.checked_rem(r).unwrap_or(0) },
                7 if a > r => acc.u = r,
                8 if a < r => acc.u = r,
                0xe => acc.u = a & r,
                0xf => acc.u = a | r,
                0x10 => acc.u = a ^ r,
                _ => cmp_u(acc, op, a, r, false),
            }
        }
        5..=8 => aggregate(acc, op, o, true),
        _ => {}
    }
}

/// With a signed value so far.
fn signed(acc: &mut Value, op: i32, o: &Operand) {
    let a = acc.s;
    match o.kind {
        1 => {
            let v = o.w;
            match op {
                0 => (acc.kind, acc.u) = (1, v),
                1 => acc.s = a.wrapping_add(v as i32),
                2 => acc.s = a.wrapping_sub(v as i32),
                3 => acc.s = a.wrapping_mul(v as i32),
                4 => acc.s = (a as u32).checked_div(v).unwrap_or(0) as i32,
                5 if v != a as u32 => acc.s = rand_u(a as u32, v) as i32,
                6 => acc.s = if v == 0 { 0 } else { (a as u32 % v) as i32 },
                7 if a > v as i32 => acc.s = v as i32,
                8 if a < v as i32 => acc.s = v as i32,
                0xe => acc.s = a | v as i32,
                0xf => acc.s = a & v as i32,
                0x10 => acc.s = a ^ v as i32,
                _ => cmp_s(acc, op, a, v as i32),
            }
        }
        2 => {
            let v = o.w as i32;
            match op {
                0 => acc.s = v,
                1 => acc.s = a.wrapping_add(v),
                2 => acc.s = a.wrapping_sub(v),
                3 => acc.s = a.wrapping_mul(v),
                4 => acc.s = if v == 0 { 0 } else { a.wrapping_div(v) },
                5 if v != a => acc.s = rand_s(a, v),
                6 => acc.s = if v == 0 { 0 } else { a.wrapping_rem(v) },
                7 if a > v => acc.s = v,
                8 if a < v => acc.s = v,
                0xe => acc.s = a | v,
                0xf => acc.s = a & v,
                0x10 => acc.s = a ^ v,
                _ => cmp_s(acc, op, a, v),
            }
        }
        3 => {
            let f = f32::from_bits(o.w);
            let r = round(f);
            match op {
                0 => (acc.kind, acc.f) = (3, f),
                1 => acc.s = fround(a as f64 + d(f)),
                2 => acc.s = fround(a as f64 - d(f)),
                3 => acc.s = fround(a as f64 * d(f)),
                4 => acc.s = if f == 0.0 { 0 } else { fround(a as f64 / d(f)) },
                5 => {
                    if r != a {
                        acc.s = rand_s(a, r);
                    }
                }
                6 => acc.s = if f == 0.0 { 0 } else { a.checked_rem(r).unwrap_or(0) },
                7 if a > r => acc.s = r,
                8 if a < r => acc.s = r,
                0xe => acc.s = a & r,
                0xf => acc.s = a | r,
                0x10 => acc.s = a ^ r,
                _ => cmp_s(acc, op, a, r),
            }
        }
        4 => {
            let x = o.d;
            let r = round_d(x);
            match op {
                0 => (acc.kind, acc.d) = (4, x),
                1 => acc.s = fround(x + a as f64),
                2 => acc.s = fround(a as f64 - x),
                3 => acc.s = fround(x * a as f64),
                4 => acc.s = if x == 0.0 { 0 } else { fround(a as f64 / x) },
                5 => {
                    if r != a {
                        acc.s = rand_s(a, r);
                    }
                }
                6 => acc.s = if x == 0.0 { 0 } else { a.checked_rem(r).unwrap_or(0) },
                7 if a > r => acc.s = r,
                8 if a < r => acc.s = r,
                0xe => acc.s = a & r,
                0xf => acc.s = a | r,
                0x10 => acc.s = a ^ r,
                _ => cmp_s(acc, op, a, r),
            }
        }
        5..=8 => aggregate(acc, op, o, false),
        _ => {}
    }
}

/// With a float value so far.
fn float(acc: &mut Value, op: i32, o: &Operand) {
    let a = d(acc.f);
    match o.kind {
        1 | 2 => {
            let (v, x) = if o.kind == 1 { (o.w, u2f(o.w)) } else { (o.w, o.w as i32 as f64) };
            match op {
                0 => assign(acc, o),
                1 => acc.f = r32(x + a),
                2 => acc.f = r32(a - x),
                3 => acc.f = r32(x * a),
                4 => acc.f = if v == 0 { 0.0 } else { r32(a / x) },
                5 => {
                    let t = round(acc.f);
                    if o.kind == 1 {
                        if v != t as u32 {
                            acc.f = r32(u2f(rand_u(0, v.wrapping_sub(t as u32))) + a);
                        }
                    } else if v as i32 != t {
                        let r = rand().checked_rem((v as i32).wrapping_sub(t)).unwrap_or(0);
                        acc.f = r32(r as f64 + a);
                    }
                }
                6 => acc.f = if v == 0 { 0.0 } else { r32(fmod(a, x)) },
                7 => {
                    let t = r32(x);
                    if t < acc.f {
                        acc.f = t;
                    }
                }
                8 => {
                    let t = r32(x);
                    if t > acc.f {
                        acc.f = t;
                    }
                }
                0x11..=0x16 => {
                    let t = round(acc.f);
                    if o.kind == 1 {
                        cmp_u(acc, op, t as u32, v, true);
                    } else {
                        cmp_s(acc, op, t, v as i32);
                    }
                }
                _ => {}
            }
        }
        3 => {
            let f = f32::from_bits(o.w);
            let b = d(f);
            match op {
                0 => acc.f = f,
                1 => acc.f = r32(a + b),
                2 => acc.f = r32(a - b),
                3 => acc.f = r32(a * b),
                4 => acc.f = if f == 0.0 { 0.0 } else { r32(a / b) },
                5 => {
                    let k = rand_frac();
                    acc.f = r32((b - a) * k + a);
                }
                6 => acc.f = if f == 0.0 { 0.0 } else { r32(fmod(a, b)) },
                7 if f < acc.f => acc.f = f,
                8 if f > acc.f => acc.f = f,
                _ => cmp_f(acc, op, a, b),
            }
        }
        4 => {
            let x = o.d;
            let t = r32(x);
            match op {
                0 => (acc.kind, acc.d) = (4, x),
                1 => acc.f = r32(d(t) + a),
                2 => acc.f = r32(a - d(t)),
                3 => acc.f = r32(d(t) * a),
                4 => acc.f = if t == 0.0 { 0.0 } else { r32(a / d(t)) },
                5 => {
                    let k = rand_frac();
                    acc.f = r32((d(t) - a) * k + a);
                }
                6 => acc.f = if t == 0.0 { 0.0 } else { r32(fmod(a, d(t))) },
                7 if t < acc.f => acc.f = t,
                8 if t > acc.f => acc.f = t,
                _ => cmp_f(acc, op, a, x),
            }
        }
        5..=8 => aggregate(acc, op, o, false),
        _ => {}
    }
}

/// With a double value so far.
fn double(acc: &mut Value, op: i32, o: &Operand) {
    let a = acc.d;
    match o.kind {
        1 | 2 => {
            let (v, x) = if o.kind == 1 { (o.w, u2f(o.w)) } else { (o.w, o.w as i32 as f64) };
            match op {
                0 => assign(acc, o),
                1 => acc.d = x + a,
                2 => acc.d = a - x,
                3 => acc.d = x * a,
                4 => acc.d = if v == 0 { 0.0 } else { a / x },
                5 => {
                    let t = round_d(a) as u32;
                    if v != t {
                        acc.d = u2f(rand_u(0, v.wrapping_sub(t))) + a;
                    }
                }
                6 => {
                    acc.d = if v == 0 {
                        0.0
                    } else if o.kind == 1 {
                        fmod(a, x)
                    } else {
                        d(r32(fmod(a, x)))
                    }
                }
                7 if x < a => acc.d = x,
                8 if x > a => acc.d = x,
                0x11..=0x16 => {
                    let t = round_d(a);
                    if o.kind == 1 {
                        cmp_u(acc, op, t as u32, v, true);
                    } else {
                        cmp_s(acc, op, t, v as i32);
                    }
                }
                _ => {}
            }
        }
        3 => {
            let f = f32::from_bits(o.w);
            let b = d(f);
            match op {
                0 => (acc.kind, acc.f) = (3, f),
                1 => acc.d = b + a,
                2 => acc.d = a - b,
                3 => acc.d = b * a,
                4 => acc.d = if f == 0.0 { 0.0 } else { a / b },
                5 => {
                    let k = rand_frac();
                    acc.d = (b - a) * k + a;
                }
                6 => acc.d = if f == 0.0 { 0.0 } else { fmod(a, b) },
                7 if b < a => acc.d = b,
                8 if b > a => acc.d = b,
                _ => cmp_f(acc, op, a, b),
            }
        }
        4 => {
            let x = o.d;
            match op {
                0 => acc.d = x,
                1 => acc.d = x + a,
                2 => acc.d = a - x,
                3 => acc.d = x * a,
                4 => acc.d = if x == 0.0 { 0.0 } else { a / x },
                5 => {
                    let k = rand_frac();
                    acc.d = (x - a) * k + a;
                }
                6 => acc.d = if x == 0.0 { 0.0 } else { fmod(a, x) },
                7 if x < a => acc.d = x,
                8 if x > a => acc.d = x,
                _ => cmp_f(acc, op, a, x),
            }
        }
        5..=8 => aggregate(acc, op, o, false),
        _ => {}
    }
}

/// The vector's length (stored as the float value, squared when not positive) times `k`.
fn length(acc: &mut Value, k: f64) {
    acc.kind = 3;
    let l2 = math::len2(&acc.v);
    acc.f = l2;
    if 0.0 < l2 {
        acc.f = r32(sqrt(d(l2)));
    }
    acc.f = r32(k * d(acc.f));
}

/// With a vector so far. (`[LENGTH]` multiplies the length by the operand; a double operand's multiplier is the entry's
/// address in the original, 0 here.)
fn vector(acc: &mut Value, op: i32, o: &Operand) {
    let cmp_tail = |acc: &mut Value, op: i32| match op {
        0x11 => (acc.kind, acc.u) = (1, 0),
        0x12 => (acc.kind, acc.u) = (1, 1),
        0x13..=0x16 => (acc.kind, acc.u) = (1, 0),
        _ => {}
    };
    match o.kind {
        1..=4 => {
            let (k, klen, zero) = match o.kind {
                1 => (r32(u2f(o.w)), u2f(o.w), o.w == 0),
                2 => (r32(o.w as i32 as f64), u2f(o.w), o.w == 0),
                3 => (f32::from_bits(o.w), d(f32::from_bits(o.w)), f32::from_bits(o.w) == 0.0),
                _ => (r32(o.d), u2f(o.w), o.d == 0.0),
            };
            match op {
                0 => assign(acc, o),
                3 => {
                    for c in acc.v.iter_mut() {
                        *c = r32(d(k) * d(*c));
                    }
                }
                4 if zero => acc.v = [0.0; 3],
                4 => {
                    for c in acc.v.iter_mut() {
                        *c = r32(d(*c) / d(k));
                    }
                }
                0xa => length(acc, klen),
                0xd if !math::is_zero(&acc.v) => acc.v = math::with_len(&acc.v, k),
                _ => cmp_tail(acc, op),
            }
        }
        5 => {
            let w = &o.v;
            match op {
                0 => acc.v = *w,
                1 => {
                    acc.v[0] = r32(d(w[0]) + d(acc.v[0]));
                    acc.v[1] = r32(d(acc.v[1]) + d(w[1]));
                    acc.v[2] = r32(d(w[2]) + d(acc.v[2]));
                }
                2 => {
                    for (c, w) in acc.v.iter_mut().zip(w) {
                        *c = r32(d(*c) - d(*w));
                    }
                }
                3 => {
                    for (c, w) in acc.v.iter_mut().zip(w) {
                        *c = r32(d(*w) * d(*c));
                    }
                }
                9 => {
                    acc.kind = 3;
                    acc.f = r32((d(acc.v[1]) * d(w[1]) + d(acc.v[0]) * d(w[0])) + d(acc.v[2]) * d(w[2]));
                }
                0xb => math::cross_in(&mut acc.v, w),
                0xc => {
                    acc.kind = 6;
                    acc.q = math::rotation_between(&acc.v, w);
                }
                0x11 => (acc.kind, acc.u) = (1, math::v_eq(&acc.v, w) as u32),
                0x12 => (acc.kind, acc.u) = (1, !math::v_eq(&acc.v, w) as u32),
                0x13..=0x16 => (acc.kind, acc.u) = (1, 0),
                _ => {}
            }
        }
        6 | 7 => match op {
            0 => assign(acc, o),
            0xc if o.kind == 6 => math::rotate_by(&mut acc.v, &o.q),
            0xc => math::transform(&mut acc.v, &o.m),
            _ => cmp_tail(acc, op),
        },
        8 => aggregate(acc, op, o, false),
        _ => {}
    }
}

/// Quaternion `[ROT]` composed the other way (see `quaternion`); off only to compare.
pub static ROT_REVERSED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

/// With a quaternion so far.
fn quaternion(acc: &mut Value, op: i32, o: &Operand) {
    match o.kind {
        6 => match op {
            0 => acc.q = o.q,
            // (FUN_10005600 calls FUN_10004f91 at 0x10008027 with the value so far as `this`, which makes it `o·acc` (Hamilton):
            // the turn so far applied first. Composed the other way here: the scripts' sky draws right only so — the horizon's
            // dish, `v(0,0,1),90 [ROT] v(0,1,0),90` over its mesh data with z negated, lies level below the eye; the faithful
            // order stands it on edge across half the sky. A departure until the conflict is found.)
            0xc if ROT_REVERSED.load(std::sync::atomic::Ordering::Relaxed) => {
                let mut r = o.q;
                math::q_mul_in(&mut r, &acc.q);
                acc.q = r;
            }
            0xc => math::q_mul_in(&mut acc.q, &o.q),
            0x11 => (acc.kind, acc.u) = (1, math::q_eq(&acc.q, &o.q) as u32),
            0x12 => (acc.kind, acc.u) = (1, !math::q_eq(&acc.q, &o.q) as u32),
            0x13..=0x16 => (acc.kind, acc.u) = (1, 0),
            _ => {}
        },
        1..=8 => aggregate(acc, op, o, false),
        _ => {}
    }
}

/// With a matrix so far (a matrix moved by a vector moves by its translation twice, as the original's).
fn matrix(acc: &mut Value, op: i32, o: &Operand) {
    let cmp_tail = |acc: &mut Value, op: i32| match op {
        0x11 => (acc.kind, acc.u) = (1, 0),
        0x12 => (acc.kind, acc.u) = (1, 1),
        0x13..=0x16 => (acc.kind, acc.u) = (1, 0),
        _ => {}
    };
    match o.kind {
        1..=4 => {
            let k = match o.kind {
                1 => r32(u2f(o.w)),
                2 => r32(o.w as i32 as f64),
                3 => f32::from_bits(o.w),
                _ => r32(o.d),
            };
            match op {
                0 => assign(acc, o),
                3 => math::m_scale(&mut acc.m, &[k, k, k]),
                4 if k != 0.0 => {
                    let inv = r32(1.0 / d(k));
                    math::m_scale(&mut acc.m, &[inv, inv, inv]);
                }
                4 => {}
                _ => cmp_tail(acc, op),
            }
        }
        5 => {
            let w = &o.v;
            let row = [acc.m[12], acc.m[13], acc.m[14]];
            match op {
                0 => assign(acc, o),
                1 => math::m_translate(&mut acc.m, &math::add(&row, w)),
                2 => math::m_translate(&mut acc.m, &math::sub(&row, w)),
                3 => math::m_scale(&mut acc.m, w),
                0xc => {
                    math::m_rotate_x(&mut acc.m, w[0]);
                    math::m_rotate_y(&mut acc.m, w[1]);
                    math::m_rotate_z(&mut acc.m, w[2]);
                }
                0x11 => (acc.kind, acc.u) = (1, 0),
                0x12 => (acc.kind, acc.u) = (1, 1),
                0x13..=0x16 => (acc.kind, acc.u) = (1, 0),
                _ => {}
            }
        }
        6 => match op {
            0 => assign(acc, o),
            0xc => math::m_mul_in(&mut acc.m, &math::q_matrix(&o.q)),
            _ => cmp_tail(acc, op),
        },
        7 => match op {
            0 => acc.m = o.m,
            0xc => math::m_mul_in(&mut acc.m, &o.m),
            0x11 => (acc.kind, acc.u) = (1, math::m_eq(&acc.m, &o.m) as u32),
            0x12 => (acc.kind, acc.u) = (1, !math::m_eq(&acc.m, &o.m) as u32),
            0x13..=0x16 => (acc.kind, acc.u) = (1, 0),
            _ => {}
        },
        8 => aggregate(acc, op, o, false),
        _ => {}
    }
}

/// With a sound so far.
fn sound(acc: &mut Value, op: i32, o: &Operand) {
    match o.kind {
        8 => match op {
            0 => acc.snd = o.snd,
            0x11..=0x16 => (acc.kind, acc.u) = (1, 0),
            _ => {}
        },
        1..=7 => aggregate(acc, op, o, false),
        _ => {}
    }
}

// --- FXS_t (fxs.rs) and Gamecode's side ---

/// `SetDataPath(path)`: the folder holding `twk` (the game's `cd_image`).
pub fn set_data_path(path: &Path) {
    with(|e| e.data_path = path.to_path_buf());
}

/// `SetExpansion(expansion)`: each object's expansions merged in.
pub fn set_expansion(expansion: u32) {
    let objects = with(|e| {
        e.expansion = expansion;
        e.unnamed.iter().chain(e.named.values()).cloned().collect::<Vec<_>>()
    });
    for o in objects {
        class_set_expansion(&o, expansion);
    }
}

/// `ReadFromScript(parser)`: global `Object`s (named, replacing a named one's contents, or `Noname`, added), until a
/// statement isn't a type; the first one read.
fn read_from_script(p: &mut Parser) -> (bool, Option<Obj>) {
    let mut first = None;
    loop {
        if !p.read_type(false) {
            return (true, first);
        }
        if p.kind != parser::OBJECT {
            p.error("Unsupported global type!");
            return (false, first);
        }
        let Some(name) = p.read_name() else { return (false, first) };
        let o = if name == id("Noname") {
            let o = ObjCell::new(name);
            with(|e| e.unnamed.push(o.clone()));
            o
        } else {
            match with(|e| e.named.get(&name).cloned()) {
                Some(o) if o.alive() => {
                    clean_object(&o);
                    o
                }
                _ => {
                    let o = ObjCell::new(name);
                    with(|e| e.named.insert(name, o.clone()));
                    o
                }
            }
        };
        if first.is_none() {
            first = Some(o.clone());
        }
        if !class_read(&o, p, 0) {
            clean_object(&o);
            return (false, first);
        }
    }
}

/// The script `dir` read (the fallback's when it is missing, then the extra one), its expansions merged in.
fn read_script(dir: &str, fallback: Option<&str>, extra: Option<&str>, params: Option<&[u8]>) -> Option<Obj> {
    let path = with(|e| e.data_path.clone());
    let mut p = Parser::new(&path, dir, fallback, extra);
    p.open(params);
    let (_, first) = read_from_script(&mut p);
    let expansion = with(|e| {
        e.errors.append(&mut p.errors);
        std::mem::take(&mut e.expansion)
    });
    set_expansion(expansion);
    first
}

/// `LoadPFTweakData(playfield, environment)`: `Tweak_Playfield_<n>`, with the outdoor (environment 0) or indoor default
/// when missing, and the orbit, sand or rock skyline for environments 2 to 4.
pub fn load_playfield(playfield: u32, environment: u32) {
    let fallback = if environment == 0 { "Tweak_Playfield_OutdoorDefault" } else { "Tweak_Playfield_IndoorDefault" };
    let extra = match environment {
        2 => Some("Tweak_Playfield_OrbitSpace"),
        3 => Some("Tweak_Playfield_SandSkyline"),
        4 => Some("Tweak_Playfield_RockSkyline"),
        _ => None,
    };
    read_script(&format!("Tweak_Playfield_{playfield}"), Some(fallback), extra, Some(b""));
}

/// `ExecuteScript(text, spawner)`: `<word> <script> <parameters>`: the script's objects read with the parameters; the
/// first object read, its spawner set when both have classes; none without the parameters part.
pub fn execute_script(text: &[u8], spawner: Option<&Obj>) -> Option<Obj> {
    let first_space = text.iter().position(|&c| c == b' ').map_or(text.len(), |k| k + 1);
    let second_space = text[first_space..].iter().position(|&c| c == b' ').map(|k| first_space + k + 1).filter(|&j| j < text.len())?;
    let script: Vec<u8> = text[first_space..].iter().copied().skip_while(|c| c.is_ascii_whitespace()).take_while(|c| !c.is_ascii_whitespace()).collect();
    let o = read_script(&String::from_utf8_lossy(&script), None, None, Some(&text[second_space..]))?;
    if let Some(s) = spawner {
        if o.alive() && s.alive() {
            if let Some(c) = o.class.borrow_mut().as_mut() {
                c.spawner = Some(s.clone());
            }
        }
    }
    Some(o)
}

/// Gamecode's `SignalFXSScript` slot (0x100b2dfc): the text run as a script with the sender as its spawner; an object
/// with an `FXID` is started as that environment effect (`SetFXTweakObject`, FUN_100afbb1 then FUN_100b01a8).
fn script_signal(text: &[u8], spawner: Option<&Obj>) -> Option<Obj> {
    let o = execute_script(text, spawner)?;
    if !o.alive() {
        return None;
    }
    if let Some(v) = o.variable_named("FXID") {
        let fxid = word(&v);
        with(|e| e.started.push((fxid, o.clone())));
    }
    Some(o)
}

/// What `/fxscript` found: whether the script made an object, that object's `FXID` when it has one (the environment
/// effect started), and what the script's reader complained of.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ran {
    pub made: bool,
    pub fxid: Option<u32>,
    pub errors: Vec<String>,
}

/// The chat line's `/fxscript` (GUI.dll FUN_100b70be): the whole typed line is the signal's text, with no object
/// sending it (`SignalFXSScript(line, 0)`), so the slot reads it as `[SEND]`'s `<word> <script> <parameters>` with the
/// command itself as the word. The reader's complaints made by this run are taken out of the engine's and given back
/// (the original writes them to its log).
pub fn command(line: &[u8]) -> Ran {
    let before = with(|e| e.errors.len());
    let made = script_signal(line, None);
    let errors = with(|e| if e.errors.len() > before { e.errors.split_off(before) } else { Vec::new() });
    Ran { made: made.is_some(), fxid: made.and_then(|o| o.variable_named("FXID").map(|v| word(&v))), errors }
}

/// `InitiateAutomaticRunQueries()` then `GetNextAutomaticRunObject()` to the end: the named objects whose `Use` is 3;
/// PlayfieldInit (FUN_100b2478) starts each one's `FXID` effect.
pub fn start_automatic() {
    let named: Vec<Obj> = with(|e| e.named.values().cloned().collect());
    for o in named {
        let Some(u) = o.variable(id("Use")) else { continue };
        if word(&u) != 3 {
            continue;
        }
        if let Some(v) = o.variable_named("FXID") {
            let fxid = word(&v);
            with(|e| e.started.push((fxid, o.clone())));
        }
    }
}

/// The heartbeat of one object; true when it is used up (its class then deleted).
fn beat(o: &Obj) -> bool {
    run_heartbeat(o);
    if used_up(o) {
        delete_class(o);
        return true;
    }
    false
}

/// The texts sent this far, each through the signal's slot.
fn deliver() {
    loop {
        let sends = with(|e| std::mem::take(&mut e.sends));
        if sends.is_empty() {
            return;
        }
        for (text, from) in sends {
            script_signal(&text, Some(&from));
        }
    }
}

/// `FrameProcess()`: the frame counter advanced and every object's heartbeat run (the `Noname` ones in turn, those a
/// heartbeat starts too, then the named ones), the used up ones ended.
pub fn frame_process() {
    with(|e| e.frame = e.frame.wrapping_add(1));
    let mut i = 0;
    while let Some(o) = with(|e| e.unnamed.get(i).cloned()) {
        if beat(&o) {
            with(|e| {
                if e.unnamed.get(i).is_some_and(|x| Rc::ptr_eq(x, &o)) {
                    e.unnamed.remove(i);
                }
            });
        } else {
            i += 1;
        }
        deliver();
    }
    let named: Vec<(u32, Obj)> = with(|e| e.named.iter().map(|(k, o)| (*k, o.clone())).collect());
    for (k, o) in named {
        if beat(&o) {
            with(|e| e.named.remove(&k));
        }
        deliver();
    }
}

/// What the game writes into the `GAME` object each frame (DisplaySystem's `GameInterfaceObject_t` frame, FUN_1005cdcf,
/// from `VisualEnvFX_t::FrameProcess`'s block: Gamecode FUN_100b05b4's day time, `GameTime_t` +0xc0 and +0xc4, the eye).
#[derive(Clone, Copy, Debug, Default)]
pub struct Game {
    pub delta_time: f32,
    pub day_time: f32,
    pub system_time: u32,
    pub system_second: f32,
    pub camera_position: V3,
    pub camera_rotation: Q4,
    pub storm: f32,
    pub precipitation: f32,
    pub low_wind: V3,
    /// The suns' rotations (`GameSun1Rotation`, `GameSun2Rotation`: [`sun::rotations`]).
    pub sun1: Q4,
    pub sun2: Q4,
    /// The weather's clouds (`ThickCloudsIntensity`, the frame block's +0x114: entry 2) and how much they let through
    /// (`CloudTranspariency`, +0x1134: 1 − entry 8 / 2.333).
    pub thick_clouds: f32,
    pub cloud_transparency: f32,
    /// The high wind's drift this frame (`HighAltitudeWindX/Z`, the frame block's +0xd8 and +0xe0: FUN_100becbe's
    /// `0.004 × (wind + base wind) × dt × 0.5`) and the eye's offset from the headquarters (`OffsetFromHQ_X/Z`).
    pub high_wind: [f32; 2],
    pub offset_from_hq: [f32; 2],
}

/// The game interface's frame: the `GAME` object's variables given the frame's values (their last value, as the
/// original writes over it).
pub fn set_game(g: &Game) {
    let Some(game) = with(|e| e.named.get(&id("GAME")).cloned()) else { return };
    let set = |name: &str, f: &dyn Fn(&mut Value)| {
        if let Some(v) = game.variable_named(name) {
            if let Some(rt) = v.values.first() {
                f(&mut rt.borrow_mut().value);
            }
        }
    };
    set("SetRenderEngine", &|v| v.u = 1);
    set("GameDayTime", &|v| v.f = g.day_time);
    set("GameDeltaTime", &|v| v.f = g.delta_time);
    set("SystemTime", &|v| v.u = g.system_time);
    set("SystemSecond", &|v| v.f = g.system_second);
    set("GameCameraPosition", &|v| v.v = g.camera_position);
    set("GameCameraRotation", &|v| v.q = g.camera_rotation);
    set("LowAltitudeWind", &|v| v.v = g.low_wind);
    set("GameStormIntensity", &|v| v.f = g.storm);
    set("GamePrecipitationIntensity", &|v| v.f = g.precipitation);
    set("ThickCloudsIntensity", &|v| v.f = g.thick_clouds);
    set("HighAltitudeWindX", &|v| v.f = g.high_wind[0]);
    set("HighAltitudeWindZ", &|v| v.f = g.high_wind[1]);
    set("OffsetFromHQ_X", &|v| v.f = g.offset_from_hq[0]);
    set("OffsetFromHQ_Z", &|v| v.f = g.offset_from_hq[1]);
    set("CloudTranspariency", &|v| v.f = g.cloud_transparency);
    set("GameSun1Rotation", &|v| v.q = g.sun1);
    set("GameSun2Rotation", &|v| v.q = g.sun2);
}

/// Gamecode's run of a `ScriptIIR_t` (FUN_10130a1b, past its district music, `sounds::MusicOverrides`): types 0, 2 and 4
/// run their text (the script's format fed the length then the start, `LDBformat::Feed`) as a script with no spawner,
/// keeping the object it makes as the current event; type 4 only near its place (`x`, `z`): past 200 the chance falls as
/// `40000000 / d²` out of 1000 (`rand() % 1000`). Then, but for type 3 and a type 4 too far, the current event's
/// `SupercedeTime`: an event running for less than it has its `Life.StartTime` moved to now less it. Whether an event's
/// object is there (for type 1's music).
pub fn script_event(kind: i32, text: &str, place: [i32; 2], at: [f32; 3], now: i32, rand: &mut crate::audio::Rand) -> bool {
    let run = |text: &str| {
        if let Some(o) = execute_script(text.as_bytes(), None) {
            with(|e| e.current = Some(o));
        }
    };
    match kind {
        0 | 2 => run(text),
        4 => {
            // (`_ftol` of the character's place)
            let (x, z) = (at[0] as i64 as i32, at[2] as i64 as i32);
            let (dx, dz) = (place[0].wrapping_sub(x), place[1].wrapping_sub(z));
            let d2 = dz.wrapping_mul(dz).wrapping_add(dx.wrapping_mul(dx));
            if 40000 < d2 && ((40_000_000i64 / d2 as i64) as i32) < rand.next() % 1000 {
                return with(|e| e.current.is_some());
            }
            run(text);
        }
        1 => {}
        _ => return with(|e| e.current.is_some()),
    }
    if let Some(cur) = with(|e| e.current.clone()).filter(|o| o.alive()) {
        let supercede = cur.variable_named("SupercedeTime");
        let life = cur.child_named("Life");
        let (start, lifetime) = (life.as_ref().and_then(|l| l.variable_named("StartTime")), life.as_ref().and_then(|l| l.variable_named("LifeTime")));
        if let (Some(supercede), Some(start), Some(lifetime)) = (supercede, start, lifetime) {
            let s = float_of(&supercede);
            let st = word(&start);
            let _ = float_of(&lifetime);
            let elapsed = now.wrapping_sub(st as i32) as u32 as f32;
            if s > elapsed {
                if let Some(rt) = start.values.first() {
                    rt.borrow_mut().value.u = now.wrapping_sub(s as i32) as u32;
                }
            }
        }
    }
    with(|e| e.current.is_some())
}

/// The sounds the scripts played this frame, to be played (`PlayGameSound`'s arguments: the pitch and range rounded).
pub fn take_sounds() -> Vec<Sound> {
    with(|e| std::mem::take(&mut e.sounds))
}

/// The effects started this frame (their `FXID`s and objects) and the errors and prints reported.
pub fn take_started() -> Vec<(u32, Obj)> {
    with(|e| std::mem::take(&mut e.started))
}

pub fn take_messages() -> Vec<String> {
    with(|e| {
        let mut m = std::mem::take(&mut e.errors);
        m.append(&mut e.prints);
        m
    })
}

/// Whether a script event has made an object (for type 1's music).
pub fn has_event() -> bool {
    with(|e| e.current.is_some())
}

/// A named object (`QueryObject`).
pub fn query(name: &str) -> Option<Obj> {
    with(|e| e.named.get(&id(name)).cloned())
}

/// A variable's value: unsigned or signed as a word, float, vector.
pub fn float_of(v: &Variable) -> f32 {
    v.values.first().map_or(0.0, |rt| work(rt).f)
}

pub fn vector_of(v: &Variable) -> V3 {
    v.values.first().map_or([0.0; 3], |rt| work(rt).v)
}

pub fn quaternion_of(v: &Variable) -> Q4 {
    v.values.first().map_or([0.0, 0.0, 0.0, 1.0], |rt| work(rt).q)
}

pub fn string_of(v: &Variable) -> Option<String> {
    v.string.as_ref().map(|s| String::from_utf8_lossy(s).into_owned())
}

/// A variable's elements (0 for one value) and its type.
pub fn count_of(v: &Variable) -> u32 {
    v.count
}

pub fn kind_of(v: &Variable) -> i32 {
    v.kind.get()
}

/// Element `i`'s float (slot 15), vector (slot 23) and word (slot 7); the value's for a single one.
pub fn float_at(v: &Variable, i: u32) -> f32 {
    v.values.get(i as usize).map_or(0.0, |rt| work(rt).f)
}

pub fn vector_at(v: &Variable, i: u32) -> V3 {
    v.values.get(i as usize).map_or([0.0; 3], |rt| work(rt).v)
}

pub fn word_at(v: &Variable, i: u32) -> u32 {
    v.values.get(i as usize).map_or(0, |rt| {
        let x = work(rt);
        if v.kind.get() == 3 { x.s as u32 } else { x.u }
    })
}

/// A runtime value's word (a state blob's criterion or special render state).
pub fn rt_word(rt: &RtRef) -> u32 {
    let r = rt.borrow().kind;
    let x = work(rt);
    if r == 3 { x.s as u32 } else { x.u }
}

/// A runtime value's matrix (a state blob's transform).
pub fn rt_matrix(rt: &RtRef) -> M16 {
    work(rt).m
}

/// `UpdateStateBlobVisual`'s check (`StateBlob_c::Validate`'s part here): every criterion nonzero.
pub fn blob_valid(b: &StateBlob) -> bool {
    b.criteria.iter().all(|c| rt_word(c) != 0)
}

/// A state blob material's variable (`GetMaterial` then `GetPointer`), resolved on the owner's call stack.
pub fn blob_material(b: &StateBlob, owner: &Obj) -> Option<Rc<Variable>> {
    let (_, lazy) = b.materials.last()?;
    let mut lazy = lazy.borrow_mut();
    let lp = lazy.as_mut()?;
    push_callstack(owner);
    let v = lp.pointer();
    pop_callstack();
    v
}

/// The live objects (`Noname` and named).
pub fn objects() -> (usize, usize) {
    with(|e| (e.unnamed.len(), e.named.len()))
}

/// Everything forgotten: the playfield's shutdown (Gamecode FUN_100b2972) deletes the FXS instance (`FXS_t::DeleteInstance`)
/// with all its objects, so the next playfield starts from its own tweaks alone (and the tests start clean).
pub fn reset() {
    with(|e| {
        e.named.clear();
        e.unnamed.clear();
        e.callstack.clear();
        e.depth = 0;
        e.status = 0;
        e.frame = 0;
        e.rand = Rand(1);
        e.sounds.clear();
        e.sends.clear();
        e.started.clear();
        e.prints.clear();
        e.errors.clear();
    });
}

/// A script's text read in (for the tests): its objects as `ReadFromScript` reads them.
#[cfg(test)]
pub fn read_text(text: &str) -> Option<Obj> {
    let mut p = Parser::from_text(text);
    let (_, first) = read_from_script(&mut p);
    with(|e| e.errors.append(&mut p.errors));
    first
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulas_heartbeats_and_kill() {
        reset();
        let o = read_text(
            "Object Noname\n{\n  Unsigned Use: e_Automatic\n  Unsigned Count  This.Count + 1u = 0u\n  Float Half  0.0 + This.Count<Current> / 2\n  Unsigned Heartbeat This.Count >= 3u [TRIGGER] e_Kill [STORE] This.Use\n  Float Arr [3] 0.0, 1.0, 2.0\n  Float Mid This.Arr[This.Pos]\n  Float Pos 0.5\n  Vector Up v( 0 1 0 )\n  Vector Rot This.Up [ROT] This.Turn\n  Quaternion Turn v( 0 0 1 ), 90\n}\n",
        )
        .unwrap();
        assert_eq!(with(|e| e.errors.clone()), Vec::<String>::new());
        let v = |n: &str| o.variable_named(n).unwrap();
        // (a literal is worked out once; the counter starts from its initial value 0 and counts once a frame)
        assert_eq!(word(&v("Use")), 3);
        frame_process();
        assert_eq!(word(&v("Count")), 1);
        assert_eq!(float_of(&v("Half")), 0.5);
        // (an array at a float position, interpolated: 3 × 0.5 = 1.5 → between 1.0 and 2.0)
        assert_eq!(float_of(&v("Mid")), 1.5);
        let r = vector_of(&v("Rot"));
        assert!((r[0] + 1.0).abs() < 1e-5 && r[1].abs() < 1e-5, "{r:?}");
        frame_process();
        assert!(o.alive());
        frame_process();
        // (the third frame's count reaches 3: the heartbeat kills it)
        assert!(!o.alive());
        assert_eq!(objects().0, 0);
    }

    #[test]
    fn game_variables_and_sends() {
        reset();
        read_text("Object GAME\n{\n  Float GameDeltaTime: 0\n  Unsigned SystemTime 0u\n  String Dest \"DESTINATION_ENVIRONMENT\"\n}\n");
        let o = read_text(
            "Object Noname\n{\n  Unsigned Use e_Automatic\n  Float T GAME.GameDeltaTime * 2\n  String Child \"# Missing_Script Unsigned Spawns 5u #\"\n  Unsigned Heartbeat 1u [SEND] This.Child\n}\n",
        )
        .unwrap();
        set_game(&Game { delta_time: 0.25, ..Default::default() });
        assert_eq!(float_of(&o.variable_named("T").unwrap()), 0.5);
        frame_process();
        // (the send ran the missing script through the slot: nothing read, nothing started)
        assert!(with(|e| e.started.is_empty() && e.sends.is_empty()));
    }
}

#[cfg(test)]
mod game_data {
    use super::*;

    /// The automatic objects the tweaks of a playfield start (their names), after the ones before.
    fn automatic(playfield: u32, environment: u32) -> std::collections::BTreeSet<u32> {
        with(|e| e.started.clear());
        load_playfield(playfield, environment);
        start_automatic();
        with(|e| e.started.iter().map(|(_, o)| o.name).collect())
    }

    /// Zoning from Old Athens into a shop: the outdoor sky's objects must not start again indoors (the original deletes
    /// the FXS instance when a playfield shuts down, FUN_100b2972).
    #[test]
    fn a_new_playfield_starts_from_its_own_tweaks() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        let data = Path::new(&game).join("cd_image");
        reset();
        set_data_path(&data);
        let indoor_alone = automatic(4704, 1);
        reset();
        set_data_path(&data);
        let outdoor = automatic(540, 0);
        let outdoor_only: Vec<u32> = outdoor.difference(&indoor_alone).copied().collect();
        assert!(!outdoor_only.is_empty(), "the outdoor tweaks start objects of their own");
        // (without the reset the outdoor objects start again)
        let kept = automatic(4704, 1);
        assert!(outdoor_only.iter().all(|n| kept.contains(n)));
        reset();
        set_data_path(&data);
        let after = automatic(4704, 1);
        assert_eq!(after, indoor_alone);
    }

    /// The game's own scripts (set `MAO_GAME`): the outdoor playfield's tweaks read whole, and an alien flyby event run
    /// as a `ScriptIIR_t` runs one, its crafts spawned by its heartbeat.
    #[test]
    fn the_game_s_scripts() {
        let Some(game) = std::env::var_os("MAO_GAME") else { return };
        reset();
        set_data_path(&Path::new(&game).join("cd_image"));
        load_playfield(999_999, 0);
        let errors = with(|e| std::mem::take(&mut e.errors));
        eprintln!("playfield: {} errors {:?}", errors.len(), &errors[..errors.len().min(8)]);
        let g = query("GAME").expect("the GAME object");
        assert!(g.variable_named("ScriptSpawnDest").is_some());
        eprintln!("objects {:?}", objects());
        start_automatic();
        eprintln!("started {:?}", with(|e| e.started.iter().map(|(f, o)| (*f, o.name)).collect::<Vec<_>>()));
        let now = 1_800_000_000u32;
        set_game(&Game { delta_time: 0.1, system_time: now, ..Default::default() });
        let o = execute_script(format!("# Alien_Event_Flyby_01 Unsigned StartTime {now}u Float LifeTime 45.0").as_bytes(), None).expect("the event");
        eprintln!("event errors {:?}", with(|e| std::mem::take(&mut e.errors)));
        assert!(o.alive());
        for k in 0..30 {
            set_game(&Game { delta_time: 0.1, system_time: now + k / 10, system_second: (k % 10) as f32 / 10.0, ..Default::default() });
            frame_process();
        }
        let started = with(|e| e.started.iter().map(|(f, o)| (*f, o.alive())).collect::<Vec<_>>());
        eprintln!("after 30 frames: objects {:?} started {started:?} errors {:?}", objects(), with(|e| e.errors.clone()));
    }
}
