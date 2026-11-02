//! Making and reading characters: the creation request of the original client
//! (`mao_net::create` holds the writing side, this module the reading side) and
//! the replies the login server sends back.

use mao_net::create::{NewCharacter, Reply};
use mao_net::frame::Frame;
use mao_net::system::{Error, Stream};

pub use mao_net::create::{
    CHARACTER_DELETED, CREATE_CHARACTER, CREATE_REFUSED, CREATED, DELETE_CHARACTER, SUGGESTED_NAME, SUGGEST_NAME,
};

/// System message 0x0f: the character the client wants to make.
pub fn create_character(c: &NewCharacter) -> Frame {
    mao_net::create::create_character(c)
}

/// System message 0x14: delete the character with this id.
pub fn delete_character(id: u32) -> Frame {
    mao_net::create::delete_character(id)
}

/// System message 0x55: a name for a character of this breed, sex and profession, please.
pub fn suggest_name(breed: i32, sex: i32, profession: i32) -> Frame {
    mao_net::create::suggest_name(breed, sex, profession)
}

/// The reply in a system message, if it is one of these.
pub fn reply(frame: &Frame) -> Result<Option<Reply>, Error> {
    mao_net::create::reply(frame)
}

/// Read a creation request's payload: the `CharacterData_t` of version 4 (the
/// id, the playfield proxy, whether the character is created) and the
/// `CharacterInfo_c` of version 5 (the name, breed, sex, profession, level, the
/// area, the look). The ints are big-endian and unaligned, as the original's
/// `BinaryStream` reads them.
pub fn read_new_character(payload: &[u8]) -> Option<NewCharacter> {
    let mut s = Stream::new(payload);
    let version = s.i32("the version").ok()?;
    if version < 4 {
        return None;
    }
    let _id = s.i32("the id").ok()?;
    let _proxy_version = s.u8("the proxy version").ok()?;
    for _ in 0..6 {
        let _ = s.i32("the proxy").ok()?;
    }
    let _created = s.i32("created").ok()?;
    let _info = s.i32("the info version").ok()?;
    let _instance = s.i32("the instance").ok()?;
    let _org = s.i32("the org instance").ok()?;
    let name = s.string("the name").ok()?;
    let breed = s.i32("the breed").ok()?;
    let sex = s.i32("the sex").ok()?;
    let profession = s.i32("the profession").ok()?;
    let _level = s.i32("the level").ok()?;
    let _area = s.string("the area").ok()?;
    let _banned = s.i32("banned").ok()?;
    let _ban_reason = s.string("the ban reason").ok()?;
    let head = s.i32("the head").ok()?;
    let height = s.i32("the height").ok()?;
    let width = s.i32("the width").ok()?;
    let _trailer = s.i32("the trailer").ok()?;
    Some(NewCharacter { name, breed, sex, profession, head, height, width })
}
