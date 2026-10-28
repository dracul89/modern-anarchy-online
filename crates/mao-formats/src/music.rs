//! The interactive music's project (`cd_image/sound/music/env/anarchy.sws`), as SIMPlayer.dll loads it (the `simplayer`
//! drop-in's `project.rs`, `sample.rs`, `transition.rs`, `fade.rs`, `pause.rs`) through serialize.dll's object archive (the
//! `serialize` drop-in's `archive.rs` and `message.rs`).
//!
//! # The archive (`ObjectArchive_c::LoadFromStream`)
//! `1`, a symbol count and the symbols (a prefix and a name, each NUL-terminated); an object count; the root stream's size and
//! bytes; then each object's stream after its size. A stream (`Message_c::UnFlatten`, with no symbol table of its own: the
//! archive's is shared) is `1`, a field count and the fields: the symbol's index (one byte below 0x80, two with the top bits
//! 10, four with 11, big-endian), the type, the element size (0: values of their own length, each after a 32-bit length), the
//! size used and the values. An object's class is its `__class_id__` (prefix "0"): the index of the class's name among the
//! symbols. Each class level of an object reads its fields under its own prefix, "1", "2", ... (`GetStream`), a lookup falling
//! back to "0"; an object field is an index into the objects (-1 for none).
//!
//! # The project (`CProject(ObjectArchive_c*)`, root field "proj_ver1")
//! Layers (`lcnt`; `lid`, `lname`, `lpause`, `lvol` each), samples (`scnt`; `samp`, `CSampleExtras` objects): a name (the file
//! `<layer>/<name>.wav`), its offset (`totoffms`), transitions out (`ntrans`, `trans`) and layer transitions out (`tpcount`,
//! `ptrans`), the end marker time (`endmtime`), the layer (`lid`), fades in and out (`finfo`), the entry flag (`entflg`), the
//! volume (`vol`) and the beat grid; a transition (`CTransition`) names its samples by number (`sampid`), its priority (`pri`),
//! fades (`finfo`, `ofinfo`), the fade out time (`fot`), the fade in position or time (`fip`, `fit`) and the fade time
//! (`ftime`); a fade (`CFadeInfo`) its curve (`ctype`), condition (`fcond`), factor (`cfac`) and levels (`levin`, `levout`).

use std::collections::HashMap;

/// A field: its type, element size and values.
#[derive(Clone, Debug)]
struct Field {
    kind: u32,
    element: i32,
    data: Vec<u8>,
}

/// A stream's fields by symbol.
type Stream = HashMap<u32, Field>;

/// serialize.dll's object archive.
#[derive(Debug)]
pub struct Archive {
    symbols: Vec<(String, String)>,
    by_name: HashMap<(String, String), u32>,
    root: Stream,
    objects: Vec<Stream>,
}

/// The field types (`Message_c`).
const BOOL: u32 = 0;
const INT32: u32 = 3;
const STRING: u32 = 6;
const FLOAT: u32 = 10;
const OBJECT: u32 = 17;

fn read_stream(data: &[u8]) -> Option<Stream> {
    let word = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
    let count = word(4)?;
    let mut at = 8;
    let mut fields = HashMap::new();
    for _ in 0..count {
        let first = *data.get(at)?;
        let index = if first & 0x80 == 0 {
            at += 1;
            first as u32
        } else if first & 0xc0 == 0x80 {
            at += 2;
            ((first & 0x3f) as u32) << 8 | *data.get(at - 1)? as u32
        } else {
            at += 4;
            ((first & 0x3f) as u32) << 24 | (*data.get(at - 3)? as u32) << 16 | (*data.get(at - 2)? as u32) << 8 | *data.get(at - 1)? as u32
        };
        let (kind, element, used) = (word(at)?, word(at + 4)? as i32, word(at + 8)? as usize);
        at += 12;
        let values = data.get(at..at + used)?.to_vec();
        at += used;
        fields.insert(index, Field { kind, element, data: values });
    }
    Some(fields)
}

impl Archive {
    pub fn parse(data: &[u8]) -> Option<Archive> {
        let word = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));
        if word(0)? != 1 {
            return None;
        }
        let count = word(4)?;
        let mut at = 8;
        let mut symbols = Vec::new();
        let mut text = || -> Option<String> {
            let rest = data.get(at..)?;
            let end = rest.iter().position(|&b| b == 0)?;
            at += end + 1;
            Some(rest[..end].iter().map(|&b| b as char).collect())
        };
        for _ in 0..count {
            let prefix = text()?;
            let name = text()?;
            symbols.push((prefix, name));
        }
        let objects = word(at)?;
        let size = word(at + 4)? as usize;
        let root = read_stream(data.get(at + 8..at + 8 + size)?)?;
        at += 8 + size;
        let mut streams = Vec::new();
        for _ in 0..objects {
            let size = word(at)? as usize;
            streams.push(read_stream(data.get(at + 4..at + 4 + size)?)?);
            at += 4 + size;
        }
        let by_name = symbols.iter().enumerate().map(|(i, s)| (s.clone(), i as u32)).collect();
        Some(Archive { symbols, by_name, root, objects: streams })
    }

    /// A field of a stream, under the prefix else "0".
    fn field<'a>(&self, stream: &'a Stream, prefix: &str, name: &str) -> Option<&'a Field> {
        [prefix, "0"].iter().find_map(|p| stream.get(self.by_name.get(&(p.to_string(), name.to_string()))?))
    }

    /// The `index`th value of a fixed-size field of the type.
    fn value(&self, stream: &Stream, prefix: &str, name: &str, kind: u32, index: usize) -> Option<[u8; 4]> {
        let f = self.field(stream, prefix, name).filter(|f| f.kind == kind)?;
        let size = f.element.max(1) as usize;
        let bytes = f.data.get(index * size..index * size + size)?;
        let mut out = [0u8; 4];
        out[..size.min(4)].copy_from_slice(&bytes[..size.min(4)]);
        Some(out)
    }

    fn int(&self, stream: &Stream, prefix: &str, name: &str, index: usize) -> Option<i32> {
        self.value(stream, prefix, name, INT32, index).map(i32::from_le_bytes)
    }

    fn float(&self, stream: &Stream, prefix: &str, name: &str, index: usize) -> Option<f32> {
        self.value(stream, prefix, name, FLOAT, index).map(f32::from_le_bytes)
    }

    fn bool(&self, stream: &Stream, prefix: &str, name: &str, index: usize) -> Option<bool> {
        self.value(stream, prefix, name, BOOL, index).map(|b| b[0] != 0)
    }

    fn object(&self, stream: &Stream, prefix: &str, name: &str, index: usize) -> Option<usize> {
        self.value(stream, prefix, name, OBJECT, index).map(i32::from_le_bytes).filter(|&i| i >= 0).map(|i| i as usize)
    }

    fn string(&self, stream: &Stream, prefix: &str, name: &str, index: usize) -> Option<String> {
        let f = self.field(stream, prefix, name).filter(|f| f.kind == STRING && f.element == 0)?;
        let mut at = 0;
        for k in 0.. {
            let len = u32::from_le_bytes(f.data.get(at..at + 4)?.try_into().ok()?) as usize;
            if k == index {
                let text = f.data.get(at + 4..at + 4 + len)?;
                let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
                return Some(text[..end].iter().map(|&b| b as char).collect());
            }
            at += 4 + len;
        }
        None
    }

    /// An object's class name.
    fn class(&self, object: usize) -> Option<&str> {
        let id = self.int(self.objects.get(object)?, "0", "__class_id__", 0)?;
        self.symbols.get(id as usize).map(|s| s.1.as_str())
    }
}

/// A fade (`CFadeInfo`): its levels, curve (0 flat, 1 arc, 2 sine) and factor.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fade {
    pub level_in: f32,
    pub level_out: f32,
    pub curve: i32,
    pub factor: f32,
    pub condition: i32,
}

/// `CFadeInfo::s_nFadeElementInterval`: a table value per 100 ms.
pub const FADE_INTERVAL: u32 = 100;

impl Fade {
    /// `CalcFadeTable(ms)`: one value per interval, flat (1), along an arc or a sine half-wave between the levels, blended with
    /// the straight line by the factor (FUN_10001b2e, FUN_10001a89: the arc `sin(2π − acos(t + k0))`, signed by the factor's
    /// sign; the sine `sin(πt + 3π/2)`; in floats as the original stores them).
    pub fn table(&self, ms: u32) -> Vec<f32> {
        self.table_at(ms, FADE_INTERVAL)
    }

    /// The table at another interval (the player sets 50 ms, FUN_1000984d).
    pub fn table_at(&self, ms: u32, interval: u32) -> Vec<f32> {
        let n = ms / interval.max(1);
        let (from, to) = (self.level_in, self.level_out);
        match self.curve {
            0 => vec![1.0; n as usize],
            1 => {
                let (k0, k1, k2) = if 0.0 < self.factor { (0.0f32, 1.0f32, 1.0f32) } else { (-1.0, 0.0, -1.0) };
                let a = self.factor.abs();
                (0..n)
                    .map(|i| {
                        let t = (i as f64 / n.wrapping_sub(1) as f64) as f32;
                        let arc = (((2.0 * std::f64::consts::PI) as f32 as f64 - ((t + k0) as f64).acos() as f32 as f64).sin() as f32) * k2 + k1;
                        let v = arc * a + (1.0 - a) * t;
                        v * (to - from) + from
                    })
                    .collect()
            }
            2 => (0..n)
                .map(|i| {
                    let t = i as f32 / n as f32;
                    let s = ((t as f64 * std::f64::consts::PI + 4.71238911151886) as f32 as f64).sin() as f32;
                    let x = (s + 1.0) * 0.5;
                    let v = x * self.factor + (1.0 - self.factor) * t;
                    v * (to - from) + from
                })
                .collect(),
            _ => vec![0.0; n as usize],
        }
    }
}

/// A transition (`CTransition`) out of a sample: the sample it enters, its priority, the fades, the fade out time (where in
/// the first sample the fade starts), the fade in position (samples) or time (ms) in the second, and the fade's length.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    pub to: usize,
    pub priority: i32,
    pub fade_out: Fade,
    pub fade_in: Fade,
    pub out_time: i32,
    pub in_position: i32,
    pub in_time: i32,
    pub time: i32,
}

/// A sample (`CSampleExtras`): its file's name in its layer's directory, layer, offset (ms), end marker time (ms), fades,
/// entry flag, volume and the transitions out (within the layer, and to other layers).
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    pub name: String,
    pub layer: i32,
    pub offset: u32,
    pub end_time: u32,
    pub fade_in: Fade,
    pub fade_out: Fade,
    pub entry: bool,
    pub volume: f32,
    pub transitions: Vec<Transition>,
    pub layer_transitions: Vec<Transition>,
}

/// A pause track (`PauseTrack_c`, `pause.rs`): beats to play and to pause in turn at a tempo, from the first node to the last
/// and back to the loop node.
#[derive(Clone, Debug, PartialEq)]
pub struct PauseTrack {
    pub bpm: i32,
    pub loop_to: u32,
    /// (play beats, pause beats).
    pub nodes: Vec<(i32, i32)>,
}

/// A layer (`Layer_c`): its name (a directory under the project's: `battle\Neutral`), volume and pause track.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub id: i32,
    pub name: String,
    pub volume: f32,
    pub pause: Option<PauseTrack>,
}

/// A music project.
#[derive(Clone, Debug, Default)]
pub struct Project {
    pub layers: Vec<Layer>,
    pub samples: Vec<Sample>,
}

impl Project {
    /// The project of an archive (its root's `proj_ver1`: the transitions name samples by number).
    pub fn parse(data: &[u8]) -> Option<Project> {
        let a = Archive::parse(data)?;
        let root = a.object(&a.root, "0", "proj_ver1", 0)?;
        let p = a.objects.get(root)?;
        let mut project = Project::default();
        for i in 0..a.int(p, "1", "lcnt", 0).unwrap_or(0).max(0) as usize {
            project.layers.push(Layer {
                id: a.int(p, "1", "lid", i).unwrap_or(0),
                name: a.string(p, "1", "lname", i).unwrap_or_default(),
                volume: a.float(p, "1", "lvol", i).unwrap_or(1.0),
                pause: a.object(p, "1", "lpause", i).and_then(|o| a.objects.get(o)).map(|s| PauseTrack {
                    bpm: a.int(s, "1", "bpm", 0).unwrap_or(0),
                    loop_to: a.int(s, "1", "loopto", 0).unwrap_or(0) as u32,
                    nodes: (0..a.int(s, "1", "numnodes", 0).unwrap_or(0).max(0) as usize)
                        .map(|k| (a.int(s, "1", "pn_playbeats", k).unwrap_or(0), a.int(s, "1", "pn_pausebeats", k).unwrap_or(0)))
                        .collect(),
                }),
            });
        }
        let fade = |object: Option<usize>| -> Fade {
            let Some(s) = object.and_then(|o| a.objects.get(o)) else { return Fade::default() };
            let curve = a.int(s, "1", "ctype", 0).unwrap_or(0);
            Fade {
                curve,
                condition: a.int(s, "1", "fcond", 0).unwrap_or(curve),
                factor: a.float(s, "1", "cfac", 0).unwrap_or(0.0),
                level_in: a.float(s, "1", "levin", 0).unwrap_or(0.0),
                level_out: a.float(s, "1", "levout", 0).unwrap_or(0.0),
            }
        };
        let transition = |object: usize| -> Option<Transition> {
            let s = a.objects.get(object)?;
            Some(Transition {
                to: a.int(s, "1", "sampid", 1)?.max(0) as usize,
                priority: a.int(s, "1", "pri", 0).unwrap_or(0),
                fade_out: fade(a.object(s, "1", "finfo", 0)),
                fade_in: fade(a.object(s, "1", "finfo", 1)),
                out_time: a.int(s, "1", "fot", 0).unwrap_or(0),
                in_position: a.int(s, "1", "fip", 0).unwrap_or(0),
                in_time: a.int(s, "1", "fit", 0).unwrap_or(-1),
                time: a.int(s, "1", "ftime", 0).unwrap_or(0),
            })
        };
        for i in 0..a.int(p, "1", "scnt", 0).unwrap_or(0).max(0) as usize {
            let Some(o) = a.object(p, "1", "samp", i) else { continue };
            let s = &a.objects[o];
            let list = |count: &str, name: &str, prefix: &str| -> Vec<Transition> {
                (0..a.int(s, prefix, count, 0).unwrap_or(0).max(0) as usize).filter_map(|k| a.object(s, prefix, name, k)).filter_map(transition).collect()
            };
            project.samples.push(Sample {
                name: a.string(s, "1", "name", 0).unwrap_or_default(),
                layer: a.int(s, "1", "lid", 0).unwrap_or(0),
                offset: a.int(s, "1", "totoffms", 0).unwrap_or(0) as u32,
                end_time: a.int(s, "1", "endmtime", 0).unwrap_or(0) as u32,
                fade_in: fade(a.object(s, "1", "finfo", 0)),
                fade_out: fade(a.object(s, "1", "finfo", 1)),
                entry: a.bool(s, "1", "entflg", 0).unwrap_or(false),
                volume: a.float(s, "1", "vol", 0).unwrap_or(1.0),
                transitions: list("ntrans", "trans", "1"),
                layer_transitions: list("tpcount", "ptrans", "2"),
            });
        }
        let _ = a.class(root);
        Some(project)
    }

    /// `FindLayerID(name)`: the layer of that name (its `/` made `\`, compared as written), -1 for none.
    pub fn find_layer(&self, name: &str) -> i32 {
        let name = name.replace('/', "\\");
        self.layers.iter().find(|l| l.name == name).map_or(-1, |l| l.id)
    }

    pub fn layer(&self, id: i32) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_tables() {
        let flat = Fade { curve: 0, ..Fade::default() };
        assert_eq!(flat.table(350), vec![1.0; 3]);
        // (a sine from 0 to 1 at factor 1: (1 − cos πt) / 2)
        let sine = Fade { curve: 2, factor: 1.0, level_in: 0.0, level_out: 1.0, condition: 2 };
        let t = sine.table(400);
        assert_eq!(t.len(), 4);
        assert!(t[0].abs() < 1e-6 && (t[2] - 0.5).abs() < 1e-6);
        // (an arc at factor 0 is the straight line)
        let arc = Fade { curve: 1, factor: 0.0, level_in: 1.0, level_out: 0.0, condition: 1 };
        assert_eq!(arc.table(500), vec![1.0, 0.75, 0.5, 0.25, 0.0]);
    }
}
