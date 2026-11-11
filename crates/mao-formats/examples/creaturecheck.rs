//! `creaturecheck <game>`: every creature record (type 0xfde97): how many parse, name a body mesh, and have that mesh
//! in the database and readable; the records that fail at each step, with the mesh's number.
use mao_formats::resource::{Id, ResourceDatabase};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let db = ResourceDatabase::open_game(std::path::Path::new(&args[0])).unwrap();
    let ids: Vec<Id> = db.ids().unwrap().filter_map(Result::ok).filter(|i| i.kind == mao_formats::npc::TEMPLATE).collect();
    let (mut unparsed, mut no_mesh, mut missing, mut unread, mut good) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), 0usize);
    for id in &ids {
        let Some(data) = db.get(*id).ok().flatten() else { continue };
        let Some(record) = mao_formats::npc::Template::parse(&data) else {
            unparsed.push(id.instance);
            continue;
        };
        let Some(mesh) = record.mesh else {
            no_mesh.push(id.instance);
            continue;
        };
        match db.get(Id { kind: 0xf6952, instance: mesh }).ok().flatten() {
            None => missing.push((id.instance, mesh)),
            Some(m) => match mao_formats::catmesh::parse(&m) {
                Ok(_) => good += 1,
                Err(e) => unread.push((id.instance, mesh, e.to_string())),
            },
        }
    }
    println!("{} creature records: {good} with a readable body; {} do not parse, {} name no mesh, {} name a mesh that is not there, {} have a mesh that does not read", ids.len(), unparsed.len(), no_mesh.len(), missing.len(), unread.len());
    println!("do not parse: {:x?}", &unparsed[..unparsed.len().min(40)]);
    println!("no mesh: {:x?}", &no_mesh[..no_mesh.len().min(40)]);
    println!("mesh not there: {:x?}", &missing[..missing.len().min(40)]);
    println!("mesh does not read: {:x?}", &unread[..unread.len().min(20)]);
}
