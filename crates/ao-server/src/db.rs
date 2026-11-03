//! The accounts, the characters and what they have, in SQLite (the user asked for it: "create and actual account use sqllite as the
//! database in the interim").

use mao_net::system::Character;

use crate::progress::{Hero, Item};

/// The schema of the database.
const SCHEMA: &str = "\
CREATE TABLE IF NOT EXISTS accounts (\n\
    name TEXT PRIMARY KEY,\n\
    password TEXT NOT NULL\n\
);\n\
CREATE TABLE IF NOT EXISTS characters (\n\
    id INTEGER PRIMARY KEY,\n\
    account TEXT NOT NULL,\n\
    name TEXT NOT NULL,\n\
    breed INTEGER NOT NULL,\n\
    sex INTEGER NOT NULL,\n\
    profession INTEGER NOT NULL,\n\
    level INTEGER NOT NULL,\n\
    area TEXT NOT NULL,\n\
    banned INTEGER NOT NULL,\n\
    ban_reason TEXT NOT NULL,\n\
    head INTEGER NOT NULL,\n\
    height INTEGER NOT NULL,\n\
    width INTEGER NOT NULL,\n\
    flag INTEGER NOT NULL,\n\
    created INTEGER NOT NULL\n\
);\n\
CREATE TABLE IF NOT EXISTS missions (\n\
    character INTEGER NOT NULL,\n\
    mission_type INTEGER NOT NULL,\n\
    instance INTEGER NOT NULL,\n\
    title TEXT NOT NULL,\n\
    description TEXT NOT NULL,\n\
    credits INTEGER NOT NULL,\n\
    xp INTEGER NOT NULL,\n\
    PRIMARY KEY (character, mission_type, instance)\n\
);\n\
CREATE TABLE IF NOT EXISTS items (\n\
    character INTEGER NOT NULL,\n\
    placement INTEGER NOT NULL,\n\
    template INTEGER NOT NULL,\n\
    count INTEGER NOT NULL,\n\
    quality INTEGER NOT NULL,\n\
    PRIMARY KEY (character, placement)\n\
);";

/// The account columns: the character slots the account has, and its expansion flags (the list's trailer, as the captures show).
const ACCOUNT_COLUMNS: &[&str] = &["ALTER TABLE accounts ADD COLUMN slots INTEGER NOT NULL DEFAULT 6", "ALTER TABLE accounts ADD COLUMN expansions INTEGER NOT NULL DEFAULT 1"];

/// The columns added after the first version of the database (an older file gets them; a new one has them from the start).
const PROGRESS_COLUMNS: &[&str] = &[
    "ALTER TABLE characters ADD COLUMN xp INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE characters ADD COLUMN health INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE characters ADD COLUMN credits INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE characters ADD COLUMN playfield INTEGER NOT NULL DEFAULT 0",
    "ALTER TABLE characters ADD COLUMN px REAL NOT NULL DEFAULT 0",
    "ALTER TABLE characters ADD COLUMN py REAL NOT NULL DEFAULT 0",
    "ALTER TABLE characters ADD COLUMN pz REAL NOT NULL DEFAULT 0",
];

/// The accounts and characters of the server, in a SQLite database file.
pub struct Db {
    conn: std::sync::Mutex<rusqlite::Connection>,
}

impl Db {
    /// The database: the file the caller names (the `MAO_DB` of the server), or an in-memory one when it cannot be opened.
    pub fn open(path: &str) -> Self {
        let conn = rusqlite::Connection::open(path).unwrap_or_else(|_| rusqlite::Connection::open(":memory:").expect("an in-memory database"));
        let _ = conn.execute_batch(SCHEMA);
        for column in ACCOUNT_COLUMNS.iter().chain(PROGRESS_COLUMNS) {
            // (a column the file already has is an error to add again, and that is fine)
            let _ = conn.execute(column, []);
        }
        Db { conn: std::sync::Mutex::new(conn) }
    }

    /// An account's character slots and expansion flags (none when there is no such account).
    pub fn account_settings(&self, name: &str) -> Option<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT slots, expansions FROM accounts WHERE name = ?1", [name], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?))).ok()
    }

    /// Sets an account's character slots and expansion flags.
    pub fn set_account_settings(&self, name: &str, slots: i64, expansions: i64) -> Result<(), rusqlite::Error> {
        self.conn.lock().unwrap().execute("UPDATE accounts SET slots = ?2, expansions = ?3 WHERE name = ?1", rusqlite::params![name, slots, expansions])?;
        Ok(())
    }

    /// Whether an account exists.
    pub fn account_exists(&self, name: &str) -> bool {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM accounts WHERE name = ?1", [name], |r| r.get::<_, i64>(0)).map_or(false, |n| n > 0)
    }

    /// The accounts, each with how many characters it has, by name.
    pub fn list_accounts(&self) -> Vec<(String, i64)> {
        let conn = self.conn.lock().unwrap();
        let Ok(mut stmt) = conn.prepare("SELECT a.name, (SELECT COUNT(*) FROM characters c WHERE c.account = a.name) FROM accounts a ORDER BY a.name") else { return Vec::new() };
        let Ok(rows) = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))) else { return Vec::new() };
        rows.flatten().collect()
    }

    /// The characters of an account by name, with their level (for the admin panel).
    pub fn character_names(&self, account: &str) -> Vec<(u32, String, i32)> {
        self.characters_of(account).into_iter().map(|c| (c.id, c.name, c.level)).collect()
    }

    /// Removes an account with its characters and what they carried.
    pub fn delete_account(&self, name: &str) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM items WHERE character IN (SELECT id FROM characters WHERE account = ?1)", [name])?;
        conn.execute("DELETE FROM missions WHERE character IN (SELECT id FROM characters WHERE account = ?1)", [name])?;
        conn.execute("DELETE FROM characters WHERE account = ?1", [name])?;
        conn.execute("DELETE FROM accounts WHERE name = ?1", [name])?;
        Ok(())
    }

    /// Make an account known (the one the server answers the logins of).
    pub fn save_account(&self, name: &str, password: &str) -> Result<(), rusqlite::Error> {
        self.conn.lock().unwrap().execute("INSERT INTO accounts (name, password) VALUES (?1, ?2) ON CONFLICT (name) DO UPDATE SET password = excluded.password", rusqlite::params![name, password])?;
        Ok(())
    }

    /// The characters of one account, oldest first.
    pub fn characters_of(&self, account: &str) -> Vec<Character> {
        let mut out = Vec::new();
        let conn = self.conn.lock().unwrap();
        let result = conn.prepare("SELECT id, name, breed, sex, profession, level, area, banned, ban_reason, head, height, width, flag, created FROM characters WHERE account = ?1 ORDER BY id");
        let Ok(mut stmt) = result else { return out };
        let Ok(rows) = stmt.query_map([account], |r| {
            Ok(Character {
                id: r.get::<_, i64>(0)? as u32,
                created: r.get::<_, i32>(13)? != 0,
                name: r.get::<_, String>(1)?,
                breed: r.get::<_, i32>(2)?,
                sex: r.get::<_, i32>(3)?,
                profession: r.get::<_, i32>(4)?,
                level: r.get::<_, i32>(5)?,
                area: r.get::<_, String>(6)?,
                banned: r.get::<_, i32>(7)? != 0,
                ban_reason: r.get::<_, String>(8)?,
                head: r.get::<_, i32>(9)?,
                height: r.get::<_, i32>(10)?,
                width: r.get::<_, i32>(11)?,
                flag: r.get::<_, i32>(12)?,
                proxy: [0; 6],
            })
        }) else {
            return out;
        };
        for row in rows.flatten() {
            out.push(row);
        }
        out
    }

    /// Add a character to an account, and give it the id the server made.
    pub fn insert_character(&self, account: &str, character: &Character, id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        let health = crate::progress::max_health_for(character.level.max(1) as u32) as i64;
        conn.execute(
            "INSERT INTO characters (id, account, name, breed, sex, profession, level, area, banned, ban_reason, head, height, width, flag, created, health) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            rusqlite::params![
                &id,
                account,
                &character.name,
                &character.breed,
                &character.sex,
                &character.profession,
                &character.level,
                &character.area,
                character.banned as i64,
                &character.ban_reason,
                &character.head,
                &character.height,
                &character.width,
                &character.flag,
                character.created as i64,
                health
            ],
        )?;
        Ok(())
    }

    /// Delete a character of an account, with what it carried.
    pub fn delete_character(&self, account: &str, id: i64) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM items WHERE character = ?1 AND character IN (SELECT id FROM characters WHERE account = ?2)", rusqlite::params![&id, account])?;
        conn.execute("DELETE FROM missions WHERE character = ?1 AND character IN (SELECT id FROM characters WHERE account = ?2)", rusqlite::params![&id, account])?;
        conn.execute("DELETE FROM characters WHERE account = ?1 AND id = ?2", rusqlite::params![account, &id])?;
        Ok(())
    }

    /// A character's progress: its level, experience, health, credits, place and what it carries.
    pub fn load_hero(&self, id: u32) -> Option<Hero> {
        let conn = self.conn.lock().unwrap();
        let mut hero = conn
            .query_row("SELECT level, xp, health, credits, playfield, px, py, pz FROM characters WHERE id = ?1", [id as i64], |r| {
                Ok(Hero {
                    id,
                    level: r.get::<_, i64>(0)? as u32,
                    xp: r.get::<_, i64>(1)? as u64,
                    health: r.get::<_, i64>(2)? as u32,
                    credits: r.get::<_, i64>(3)? as u32,
                    playfield: r.get::<_, i64>(4)? as u32,
                    position: [r.get::<_, f64>(5)? as f32, r.get::<_, f64>(6)? as f32, r.get::<_, f64>(7)? as f32],
                    items: Vec::new(),
                    missions: Vec::new(),
                })
            })
            .ok()?;
        if let Ok(mut stmt) = conn.prepare("SELECT mission_type, instance, title, description, credits, xp FROM missions WHERE character = ?1 ORDER BY rowid") {
            if let Ok(rows) = stmt.query_map([id as i64], |r| {
                Ok(crate::missions::Taken {
                    mission: (r.get::<_, i64>(0)? as u32, r.get::<_, i64>(1)? as u32),
                    title: r.get(2)?,
                    description: r.get(3)?,
                    credits: r.get::<_, i64>(4)? as u32,
                    xp: r.get::<_, i64>(5)? as u32,
                })
            }) {
                hero.missions = rows.flatten().collect();
            }
        }
        if let Ok(mut stmt) = conn.prepare("SELECT placement, template, count, quality FROM items WHERE character = ?1 ORDER BY placement") {
            if let Ok(rows) = stmt.query_map([id as i64], |r| {
                Ok(Item { placement: r.get::<_, i64>(0)? as u32, template: r.get::<_, i64>(1)? as u32, count: r.get::<_, i64>(2)? as u16, quality: r.get::<_, i64>(3)? as u32 })
            }) {
                hero.items = rows.flatten().collect();
            }
        }
        Some(hero)
    }

    /// Writes a character's progress and what it carries.
    pub fn save_hero(&self, hero: &Hero) -> Result<(), rusqlite::Error> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;
        tx.execute(
            "UPDATE characters SET level = ?2, xp = ?3, health = ?4, credits = ?5, playfield = ?6, px = ?7, py = ?8, pz = ?9 WHERE id = ?1",
            rusqlite::params![hero.id as i64, hero.level as i64, hero.xp as i64, hero.health as i64, hero.credits as i64, hero.playfield as i64, hero.position[0] as f64, hero.position[1] as f64, hero.position[2] as f64],
        )?;
        tx.execute("DELETE FROM missions WHERE character = ?1", [hero.id as i64])?;
        for m in &hero.missions {
            tx.execute(
                "INSERT INTO missions (character, mission_type, instance, title, description, credits, xp) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![hero.id as i64, m.mission.0 as i64, m.mission.1 as i64, &m.title, &m.description, m.credits as i64, m.xp as i64],
            )?;
        }
        tx.execute("DELETE FROM items WHERE character = ?1", [hero.id as i64])?;
        for item in &hero.items {
            tx.execute(
                "INSERT INTO items (character, placement, template, count, quality) VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![hero.id as i64, item.placement as i64, item.template as i64, item.count as i64, item.quality as i64],
            )?;
        }
        tx.commit()
    }
}
