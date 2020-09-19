use ::reasonable::owl::Reasoner;
//use ::reasonable::query;
//use ::reasonable::algebra;
use ::reasonable::manager;
//use rdf::node::Node;
//use rdf::uri::Uri;
use std::env;
use std::time::Instant;
use log::info;
//use serde::{Serialize, Deserialize};
//use serde_sexpr::from_str;

use std::collections::HashMap;
//use lexpr::{Value, parse::Error};

// macro_rules! uri {
//     ($ns:expr, $t:expr) => (Node::UriNode{uri: Uri::new(format!($ns, $t))});
// }
// macro_rules! rdf {
//     ($t:expr) => (uri!("http://www.w3.org/1999/02/22-rdf-syntax-ns#{}", $t));
// }
// macro_rules! rdfs {
//     ($t:expr) => (uri!("http://www.w3.org/2000/01/rdf-schema#{}", $t));
// }
// macro_rules! owl {
//     ($t:expr) => (uri!("http://www.w3.org/2002/07/owl#{}", $t));
// }
// macro_rules! brick {
//     ($t:expr) => (uri!("https://brickschema.org/schema/1.1/Brick#{}", $t));
// }
// macro_rules! ex {
//     ($t:expr) => (uri!("http://buildsys.org/ontologies/building_example#{}", $t));
// }
//
// TODO: use datafrog to compute transitive closure of predicates for queries (e.g. ?, +, *)


// what might queries look like?
// (prefixes (
//      (brick https://brickschema.org/schema/1.1/Brick#) 
//      (rdf http://www.w3.org/1999/02/22-rdf-syntax-ns#))
// (select (?ahu ?ds))
// (where (
//      (?ahu rdf:type brick:AHU)
//      (union 
//          ((?ahu brick:feeds+ ?ds))
//          ((?ahu brick:hasPart+ ?ds)))
//      ))
//
// (prefixes (
//      (brick https://brickschema.org/schema/1.1/Brick#) 
//      (rdf http://www.w3.org/1999/02/22-rdf-syntax-ns#))
// (select (?ahu ?ds))
// (where (
//      (?ahu rdf:type brick:AHU)
//      (?ahu brick:feeds+ ?ds)
//      (optional ((?ahu rdf:type brick:RTU)))
//   ))
//
//
//
//    (
//        (prefixes (
//         (brick https://brickschema.org/schema/1.1/Brick#) 
//         (rdf http://www.w3.org/1999/02/22-rdf-syntax-ns#))) 
//        (select (?down ?ahu ?equip)) 
//        (clauses (
//            (pat ?ahu rdf:type brick:AHU) 
//            (pat* ?ahu brick:feeds ?equip)   -- does 'feeds*'
//            (pat ?equip brick:feeds ?down)))
//    )

// enum Atom {
//     Var(String),
//     Node(String)
// }
// 
// enum VarOrPropertyPath {
//     Var(String)
//     // PropertyPath(PropertyPath)
// }
// 
// struct TriplePattern(Atom, Atom, Atom);
// 
// enum Pattern {
//     Triple(TriplePattern),
//     Optional(TriplePattern),
//     Union(Box<Pattern>, Box<Pattern>)
// }
// 
// struct Query {
//     prefixes: HashMap<String, String>,
//     select: Vec<String>,
//     patterns: Vec<Pattern>,
// }
// 
// fn dump(v: &Value) {
//     if let Value::Cons(pfxs) = &v["prefixes"] {
//         let pfxs: HashMap<String, String> = pfxs.list_iter().filter_map(|v| {
//             if let Value::Cons(p) = v {
//                 match p.as_pair() {
//                     (Value::String(pfx), Value::String(uri)) => Some((pfx.to_string(), uri.to_string())),
//                     (Value::Symbol(pfx), Value::Symbol(uri)) => Some((pfx.to_string(), uri.to_string())),
//                     (_, _) => None
//                 }
//             } else {
//                 None
//             }
//         }).collect();
//         println!("{:?}", pfxs);
//     }
// 
//     if let Value::Cons(select) = &v["select"] {
//         if let Value::Vector(v) = select.car() {
//             let vars: Vec<String> = v.iter().map(|var| var.to_string()).collect();
//             println!("select {:?}", vars);
//         }
//     }
// 
//     if let Value::Cons(wher) = &v["where"] {
//         let patterns: Vec<Pattern> = wher.list_iter().filter_map(|v| {
//             None
//         }).collect();
//     }
// 
// }

fn main() {
    let mut mgr = manager::Manager::new();
    // mgr.load_file("/home/gabe/src/Brick/Brick/Brick.ttl").unwrap();
    mgr.load_file("example_models/ontologies/Brick.n3").unwrap();
    mgr.load_file("example_models/small1.n3").unwrap();

    let vr1 = mgr.add_view("test1".to_string(), "SELECT ?x WHERE { ?x rdf:type brick:Sensor }").unwrap();

    //let q1 = r#"(
    //    (prefixes (brick . https://brick) (rdf . http://rdf))
    //    (select #(?ahu ?ds))
    //    (where #(?ahu rdf:type brick:AHU)
    //           #(?ahu brick:feeds+ ?ds))
    //)"#;
    //let v1 = lexpr::from_str(q1).unwrap();
    //dump(&v1);


    env_logger::init();
    let mut r = Reasoner::new();
    env::args().skip(1).map(|filename| {
        info!("Loading file {}", &filename);
        r.load_file(&filename).unwrap()
    }).count();
    let reasoning_start = Instant::now();
    info!("Starting reasoning");
    r.reason();
    info!("Reasoning completed in {:.02}sec", reasoning_start.elapsed().as_secs_f64());

}
