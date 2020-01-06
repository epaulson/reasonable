extern crate datafrog;
use datafrog::{Iteration, Variable};

use crate::index::URIIndex;
use crate::floyd_warshall::FloydWarshall;
use crate::disjoint_sets::DisjointSets;
use crate::types::{URI, Triple, has_pred, has_obj, has_pred_obj};

use std::fs;
use std::collections::HashMap;
use rdf::reader::turtle_parser::TurtleParser;
use rdf::reader::n_triples_parser::NTriplesParser;
use rdf::reader::rdf_parser::RdfParser;
use rdf::node::Node;
use rdf::graph::Graph;
use roaring::RoaringBitmap;

#[allow(dead_code)]
const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
#[allow(dead_code)]
const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
#[allow(dead_code)]
const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
#[allow(dead_code)]
const RDFS_SUBPROP: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
#[allow(dead_code)]
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
#[allow(dead_code)]
const OWL_SAMEAS: &str = "http://www.w3.org/2002/07/owl#sameAs";
#[allow(dead_code)]
const OWL_EQUIVALENTCLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
#[allow(dead_code)]
const OWL_INVERSEOF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
#[allow(dead_code)]
const OWL_SYMMETRICPROP: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
#[allow(dead_code)]
const OWL_EQUIVPROP: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
#[allow(dead_code)]
const OWL_FUNCPROP: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
#[allow(dead_code)]
const OWL_INVFUNCPROP: &str = "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
#[allow(dead_code)]
const OWL_INTERSECTION: &str = "http://www.w3.org/2002/07/owl#intersectionOf";

pub struct Reasoner {
    iter1: Iteration,
    index: URIIndex,
    input: Vec<Triple>,

    spo: Variable<Triple>,
    pso: Variable<Triple>,
    osp: Variable<Triple>,
    all_triples_input: Variable<Triple>,

    prp_dom: Variable<(URI, URI)>,
    prp_rng: Variable<(URI, URI)>,
    rdf_type: Variable<(URI, URI)>,

    prp_fp_1: Variable<(URI, ())>,
    prp_fp_2: Variable<Triple>,

    prp_ifp_1: Variable<(URI, ())>,
    prp_ifp_2: Variable<Triple>,

    prp_spo1_1: Variable<(URI, URI)>,

    cax_sco_1: Variable<(URI, URI)>,
    cax_sco_2: Variable<(URI, URI)>,

    owl_inverse_of: Variable<(URI, URI)>,
    owl_inverse_of2: Variable<(URI, URI)>,

    symmetric_properties: Variable<(URI, ())>,

    equivalent_properties: Variable<(URI, URI)>,
    equivalent_properties_2: Variable<(URI, URI)>,

    firsts: Variable<(URI, URI)>,
    rests: Variable<(URI, URI)>,

    cls_int_2: Variable<(URI, URI)>,
}

#[allow(unused)]
impl Reasoner {
    pub fn new() -> Self {
        let mut iter1 = Iteration::new();
        let mut index = URIIndex::new();

        // variables within the iteration
        let spo = iter1.variable::<(URI, (URI, URI))>("spo");
        let pso = iter1.variable::<(URI, (URI, URI))>("pso");
        let osp = iter1.variable::<(URI, (URI, URI))>("osp");
        let all_triples_input = iter1.variable::<(URI, (URI, URI))>("all_triples_input");

        let prp_dom = iter1.variable::<(URI, URI)>("prp_dom");
        let prp_rng = iter1.variable::<(URI, URI)>("prp_rng");
        let rdf_type = iter1.variable::<(URI, URI)>("rdf_type");

        //prp-fp variables
        // T(?p, rdf:type, owl:FunctionalProperty
        // prp-fp:
        //      T(?p, rdf:type, owl:FunctionalProperty) .
        //      T(?x, ?p, ?y1) .
        //      T(?x, ?p, ?y2) =>
        //          T(?y1, owl:sameAs, ?y2) .
        //   ----- rewritten -----
        //      T(?p, rdf:type, owl:FunctionalProperty) .
        //      T(?p, ?x, ?y1) . (pso)
        //      T(?p, ?x, ?y2) . (pso) =>
        //          T(?y1, owl:sameAs, ?y2) .
        let prp_fp_1 = iter1.variable::<(URI, ())>("prp_fp_1");
        let prp_fp_2 = iter1.variable::<Triple>("prp_fp_2");
        // T(?p, ?x, ?y1), T(?p, ?x, ?y2) fulfilled from PSO index

        // T(?p, rdf:type, owl:InverseFunctionalProperty
        // prp-ifp
        //      T(?p, rdf:type, owl:InverseFunctionalProperty) .
        //      T(?p, ?x1, ?y) . (pso)
        //      T(?p, ?x2, ?y) . (pso) =>
        //          T(?x1, owl:sameAs, ?x2) .
        let prp_ifp_1 = iter1.variable::<(URI, ())>("prp_ifp_1");
        let prp_ifp_2 = iter1.variable::<Triple>("prp_ifp_2");
        // T(?p, ?x1, ?y), T(?p, ?x2, ?y) fulfilled from PSO index

        // prp-spo1
        // T(?p1, rdfs:subPropertyOf, ?p2) .
        // T(?p1, ?x, ?y) (pso) =>
        //  T(?x, ?p2, ?y)
        // IMPLS
        // T(?p1, rdfs:subPropertyOf, ?p2)
        let prp_spo1_1 = iter1.variable::<(URI, URI)>("prp_spo1_1");

        // cax-sco
        //  T(?c1, rdfs:subClassOf, ?c2)
        //  T(?c1, ?x, rdf:type) (osp) => T(?x, rdf:type, ?c2)
        //
        //  T(?c1, rdfs:subClassOf, ?c2)
        let cax_sco_1 = iter1.variable::<(URI, URI)>("cax_sco_1");
        //  T(?c1, ?x, rdf:type)
        let cax_sco_2 = iter1.variable::<(URI, URI)>("cax_sco_2");

        // prp-inv1
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p1, ?y) => T(?y, ?p2, ?x)
        // prp-inv2
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p2, ?y) => T(?y, ?p1, ?x)
        let owl_inverse_of = iter1.variable::<(URI, URI)>("owl_inverseOf");
        let owl_inverse_of2 = iter1.variable::<(URI, URI)>("owl_inverse_of2");

        // prp-symp
        //      T(?p, rdf:type, owl:SymmetricProperty)
        //      T(?x, ?p, ?y)
        //      => T(?y, ?p, ?x)
        let symmetric_properties = iter1.variable::<(URI, ())>("symmetric_properties");

        // prp-eqp1
        // T(?p1, owl:equivalentProperty, ?p2)
        // T(?x, ?p1, ?y)
        // => T(?x, ?p2, ?y)
        //
        // prp-eqp2
        // T(?p1, owl:equivalentProperty, ?p2)
        // T(?x, ?p2, ?y)
        // => T(?x, ?p1, ?y)
        let equivalent_properties = iter1.variable::<(URI, URI)>("equivalent_properties");
        let equivalent_properties_2 = iter1.variable::<(URI, URI)>("equivalent_properties_2");

        // list relations
        let firsts = iter1.variable::<(URI, URI)>("firsts");
        let rests = iter1.variable::<(URI, URI)>("rests");

        // cls-int1
        // T(?c owl:intersectionOf ?x), LIST[?x, ?c1...?cn],
        // T(?y rdf:type ?c_i) for i in range(1,n) =>
        //  T(?y rdf:type ?c)
        //
        // ?c owl:intersectionOf ?x
        let cls_int_2 = iter1.variable::<(URI, URI)>("cls_int_2");
        //
        // firsts:
        // ?y rdf:type

        Reasoner {
            iter1: iter1,
            index: index,
            input: Vec::new(),
            spo: spo,
            pso: pso,
            osp: osp,
            all_triples_input: all_triples_input,

            firsts: firsts,
            rests: rests,

            prp_dom: prp_dom,
            prp_rng: prp_rng,
            rdf_type: rdf_type,

            prp_fp_1: prp_fp_1,
            prp_fp_2: prp_fp_2,

            prp_ifp_1: prp_ifp_1,
            prp_ifp_2: prp_ifp_2,

            prp_spo1_1: prp_spo1_1,
            cax_sco_1: cax_sco_1,
            cax_sco_2: cax_sco_2,
            owl_inverse_of: owl_inverse_of,
            owl_inverse_of2: owl_inverse_of2,
            symmetric_properties: symmetric_properties,
            equivalent_properties: equivalent_properties,
            equivalent_properties_2: equivalent_properties_2,

            cls_int_2: cls_int_2,
        }
    }

    pub fn load_triples(&mut self, triples: Vec<(&'static str, &'static str, &'static str)>) {
        let trips: Vec<(URI, (URI, URI))> = triples.iter().map(|trip| {
            (self.index.put_str(trip.0), (self.index.put_str(trip.1), self.index.put_str(trip.2)))
        }).collect();
        self.input.extend(trips);
        // self.all_triples_input.insert(trips.into());
    }

    pub fn load_file(&mut self, filename: &str) -> Result<(), String> {
        let data = fs::read_to_string(filename).expect("Unable to read file");

        let graph: Graph = {
            if filename.ends_with(".ttl") {
                TurtleParser::from_string(data).decode().expect("bad turtle")
            } else if  filename.ends_with(".n3") {
                NTriplesParser::from_string(data).decode().expect("bad turtle")
            } else {
                return Err("no parser for file".to_string());
            }
        };

        //} else if filename.ends_with(".n3") {
        //    NTriplesParser::from_string(data)
        //}
        //let graph = Box::new(reader.decode().expect("bad reader"));
        //if let Ok(graph) = reader.decode() {
        println!("count: {} {}", filename, graph.count());
        let triples : Vec<(URI, (URI, URI))> = graph.triples_iter().map(|_triple| {
            let triple = _triple;
            let subject = match triple.subject() {
                Node::UriNode{uri} => uri.to_string(),
                Node::LiteralNode{literal, data_type: _, language: _} => &literal,
                Node::BlankNode{id} => &id,
            };

            let predicate = match triple.predicate() {
                Node::UriNode{uri} => uri.to_string(),
                Node::LiteralNode{literal, data_type: _, language: _} => &literal,
                Node::BlankNode{id} => &id,
            };

            let object = match triple.object() {
                Node::UriNode{uri} => uri.to_string(),
                Node::LiteralNode{literal, data_type: _, language: _} => &literal,
                Node::BlankNode{id} => &id,
            };
            //println!("{} {} {}", subject, predicate, object);

            let (s, (p, o)) = (self.index.put(subject.to_string()), (self.index.put(predicate.to_string()), self.index.put(object.to_string())));


            (s, (p,o))

        }).collect();

        //self.all_triples_input.insert(triples.into());
        self.input.extend(triples);

        Ok(())
    }

    pub fn reason(&mut self) {
        let rdftype_node = self.index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let rdfsdomain_node = self.index.put_str("http://www.w3.org/2000/01/rdf-schema#domain");
        let rdfsrange_node = self.index.put_str("http://www.w3.org/2000/01/rdf-schema#range");
        let owlsameas_node = self.index.put_str("http://www.w3.org/2002/07/owl#sameAs");
        let owlinverseof_node = self.index.put_str("http://www.w3.org/2002/07/owl#inverseOf");
        let owlsymmetricprop_node = self.index.put_str("http://www.w3.org/2002/07/owl#SymmetricProperty");
        let owlequivprop_node = self.index.put_str("http://www.w3.org/2002/07/owl#equivalentProperty");
        let owlequivclassprop_node = self.index.put_str("http://www.w3.org/2002/07/owl#equivalentClass");
        let owlfuncprop_node = self.index.put_str("http://www.w3.org/2002/07/owl#FunctionalProperty");
        let owlinvfuncprop_node = self.index.put_str("http://www.w3.org/2002/07/owl#InverseFunctionalProperty");
        let rdfssubprop_node = self.index.put_str("http://www.w3.org/2000/01/rdf-schema#subPropertyOf");
        let rdfssubclass_node = self.index.put_str("http://www.w3.org/2000/01/rdf-schema#subClassOf");
        let owlintersection_node = self.index.put_str("http://www.w3.org/2002/07/owl#intersectionOf");

        let rdffirst_node = self.index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#first");
        let rdfrest_node = self.index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest");
        let rdfnil_node = self.index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil");

        let prp_fp_isfuncprop = self.iter1.variable::<Triple>("a");
        let prp_fp_hasprop1 = self.iter1.variable::<Triple>("b");

        let owl_intersection_of = self.iter1.variable::<(URI, URI)>("owl_intersection_of");

        // prp-inv1
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p1, ?y) => T(?y, ?p2, ?x)
        // prp-inv2
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p2, ?y) => T(?y, ?p1, ?x)
        //
        // (p1, p2)
        let prp_inv1 = self.iter1.variable::<(URI, URI)>("prp_inv1");

        let list_test1 = self.iter1.variable::<(URI, URI)>("list_test1");

        // cls-int1
        //    T(?c owl:intersectionOf ?x), LIST[?x, ?c1...?cn],
        //    T(?y rdf:type ?c_i) for i in range(1,n) =>
        //     T(?y rdf:type ?c)
        let cls_int_1_1 = self.iter1.variable::<(URI, URI)>("cls_int_1_1");
        let cls_int_1_2 = self.iter1.variable::<(URI, URI)>("cls_int_1_2");
 
        // cls-int2
        //    T(?c owl:intersectionOf ?x), LIST[?x, ?c1...?cn],
        //     T(?y rdf:type ?c) =>
        //    T(?y rdf:type ?c_i) for i in range(1,n)
        let cls_int_2_1 = self.iter1.variable::<(URI, URI)>("cls_int_1_1");
        let cls_int_2_2 = self.iter1.variable::<(URI, URI)>("cls_int_1_2");

        // cax-eqc1
        // T(?c1, owl:equivalentClass, ?c2), T(?x, rdf:type, ?c1)  =>
        //  T(?x, rdf:type, ?c2)
        // cax-eqc2
        // T(?c1, owl:equivalentClass, ?c2), T(?x, rdf:type, ?c2)  =>
        //  T(?x, rdf:type, ?c1)
        let owl_equivalent_class = self.iter1.variable::<(URI, URI)>("owl_equivalent_class");
        let cax_eqc_1_1 = self.iter1.variable::<(URI)>("cax_eqc_1_1");
        let cax_eqc_2_1 = self.iter1.variable::<(URI)>("cax_eqc_2_1");

        // let fw = FloydWarshall::new(&self.input);
        let ds = DisjointSets::new(&self.input);

        self.all_triples_input.extend(self.input.iter().cloned());
        while self.iter1.changed() {
            self.spo.from_map(&self.all_triples_input, |&(sub, (pred, obj))| (sub, (pred, obj)));
            self.pso.from_map(&self.all_triples_input, |&(sub, (pred, obj))| (pred, (sub, obj)));
            self.osp.from_map(&self.all_triples_input, |&(sub, (pred, obj))| (obj, (sub, pred)));

            self.rdf_type.from_map(&self.spo, |&triple| { has_pred(triple, rdftype_node) });
            self.prp_dom.from_map(&self.spo, |&triple| { has_pred(triple, rdfsdomain_node) });
            self.prp_rng.from_map(&self.spo, |&triple| { has_pred(triple, rdfsrange_node) });

            self.owl_inverse_of.from_map(&self.spo, |&triple| has_pred(triple, owlinverseof_node) );
            self.owl_inverse_of2.from_map(&self.owl_inverse_of, |&(p1, p2)| (p2, p1) );

            owl_intersection_of.from_map(&self.spo, |&triple| has_pred(triple, owlintersection_node));

            let mut reflexive_equivalent_class = Vec::new();
            owl_equivalent_class.from_map(&self.spo, |&triple| {
                let (c1, c2) = has_pred(triple, owlequivclassprop_node);
                if c1 > 0 && c2 > 0 {
                    reflexive_equivalent_class.push((c2, c1));
                }
                (c1, c2)
            });
            owl_equivalent_class.extend(reflexive_equivalent_class.iter());

            self.symmetric_properties.from_map(&self.spo, |&triple| {
                has_pred_obj(triple, (rdftype_node, owlsymmetricprop_node))
            });

            self.equivalent_properties.from_map(&self.spo, |&triple| has_pred(triple, owlequivprop_node) );
            self.equivalent_properties_2.from_map(&self.equivalent_properties, |&(p1, p2)| (p2, p1));

            self.all_triples_input.from_join(&self.prp_dom, &self.pso, |&tpred, &class, &(sub, obj)| {
                (sub, (rdftype_node, class))
            });
            self.all_triples_input.from_join(&self.prp_rng, &self.pso, |&tpred, &class, &(sub, obj)| {
                (obj, (rdftype_node, class))
            });

            // prp-fp
            self.prp_fp_1.from_map(&self.spo, |&triple| { has_pred_obj(triple, (rdftype_node, owlfuncprop_node)) });
            self.prp_fp_2.from_join(&self.prp_fp_1, &self.pso, |&p, &(), &(x, y1)| (p, (x, y1)) );
            self.all_triples_input.from_join(&self.prp_fp_2, &self.pso, |&p, &(x1, y1), &(x2, y2)| {
                // if x1 and x2 are the same, then emit y1 and y2 are the same
                match x1 {
                    x2 => (y1, (owlsameas_node, y2)),
                    _ => (0, (0,0)),
                }
            });

            // prp-ifp
            self.prp_ifp_1.from_map(&self.spo, |&triple| { has_pred_obj(triple, (rdftype_node, owlinvfuncprop_node)) });
            self.prp_ifp_2.from_join(&self.prp_ifp_1, &self.pso, |&p, &(), &(x, y1)| (p, (x, y1)) );
            self.all_triples_input.from_join(&self.prp_ifp_2, &self.pso, |&p, &(x1, y1), &(x2, y2)| {
                // if y1 and y2 are the same, then emit x1 and x2 are the same
                match y1 {
                    y2 => (x1, (owlsameas_node, x2)),
                    _ => (0, (0,0)),
                }
            });

            // prp-spo1
            self.prp_spo1_1.from_map(&self.spo, |&triple| has_pred(triple, rdfssubprop_node));
            self.all_triples_input.from_join(&self.prp_spo1_1, &self.pso, |&p1, &p2, &(x, y)| (x, (p2, y)));

            // cax-sco
            self.cax_sco_1.from_map(&self.spo, |&triple| has_pred(triple, rdfssubclass_node));
            // ?c1, ?x, rdf:type
            self.cax_sco_2.from_map(&self.rdf_type, |&(inst, class)| (class, inst));
            self.all_triples_input.from_join(&self.cax_sco_1, &self.cax_sco_2, |&class, &parent, &inst| (inst, (rdftype_node, parent)));

            // cax-eqc1, cax-eqc2
            // find instances of classes that are equivalent
            self.all_triples_input.from_join(&owl_equivalent_class, &self.osp, |&c1, &c2, &(inst, pred)| {
                if pred == rdftype_node {
                    // yield inst rdf_type c2
                    (inst, (rdftype_node, c2))
                } else {
                    (0, (0, 0))
                }
            });

            // prp-inv1
            // T(?p1, owl:inverseOf, ?p2)
            // T(?x, ?p1, ?y) => T(?y, ?p2, ?x)
            self.all_triples_input.from_join(&self.owl_inverse_of, &self.pso, |&p1, &p2, &(x, y)| (y, (p2, x)) );

            // prp-inv2
            // T(?p1, owl:inverseOf, ?p2)
            // T(?x, ?p2, ?y) => T(?y, ?p1, ?x)
            self.all_triples_input.from_join(&self.owl_inverse_of2, &self.pso, |&p1, &p2, &(x, y)| (x, (p2, y)) );

            // prp-symp
            // T(?p, rdf:type, owl:SymmetricProperty)
            // T(?x, ?p, ?y)
            //  => T(?y, ?p, ?x)
            self.all_triples_input.from_join(&self.symmetric_properties, &self.pso, |&prop, &(), &(x, y)| {
                (y, (prop, x))
            });

            // prp-eqp1
            // T(?p1, owl:equivalentProperty, ?p2)
            // T(?x, ?p1, ?y)
            // => T(?x, ?p2, ?y)
            self.all_triples_input.from_join(&self.equivalent_properties, &self.pso, |&p1, &p2, &(x, y)| (x, (p2, y)) );
            // prp-eqp2
            // T(?p1, owl:equivalentProperty, ?p2)
            // T(?x, ?p2, ?y)
            // => T(?x, ?p1, ?y)
            self.all_triples_input.from_join(&self.equivalent_properties_2, &self.pso, |&p1, &p2, &(x, y)| (x, (p2, y)) );

            // cls-int1
            // There's a fair amount of complexity here that we have to manage. The rule we are
            // implementing is cls-int-1:
            //
            //      T(?c owl:intersectionOf ?x), LIST[?x, ?c1...?cn],
            //      T(?y rdf:type ?c_i) for i in range(1,n) =>
            //       T(?y rdf:type ?c)
            //
            // Useful structures:
            // - `owl_intersection_of` is keyed by class and values are the list heads
            // - `ds` gives the list values for the given head (ds.get_list_values(listname))
            //
            // Goal: we need to find instances (?y rdf:type ?class) of all classes given by the
            // list identified by the head for each owl:intersectionOf node.
            //
            // We can get the list of classes easily by iterating over each key of the
            // owl_intersection_of variable. However, we need an efficient way of seeing if there
            // are instances of *each* of those classes (union). This could be a N-way join (where
            // N is the number of classes in the list)
            // TODO: finish this up!
            cls_int_1_1.from_map(&owl_intersection_of, |&(intersection_class, listname)| {
                if let Some(values) = ds.get_list_values(listname) {
                    let value_uris: Vec<&String> = values.iter().map(|v| self.index.get(*v).unwrap()).collect();
                    println!("{} (len {}) {} {:?}", listname, values.len(), self.index.get(listname).unwrap(), value_uris);
                    cls_int_1_2.from_map(&self.rdf_type, |&(inst, list_class)| {
                        if values.contains(&list_class) {
                            println!("{} is a {}", inst, list_class);
                        }
                        (inst, list_class)
                    });
                }
                (intersection_class, listname)
            });

            // cls_int_2_1.from_map(&owl_intersection_of, |&(intersection_class, listname)| {
            // });
        }
    }


    pub fn get_triples(&mut self) -> Vec<(String, String, String)> {
        let instances = self.spo.clone().complete();

        instances.iter().map(|inst| {
            let (_s, (_p, _o)) = inst;
            let s = self.index.get(*_s).unwrap();
            let p = self.index.get(*_p).unwrap();
            let o = self.index.get(*_o).unwrap();
            (s.clone(), p.clone(), o.clone())
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_reasoner() -> Result<(), String> {
        let _r = Reasoner::new();
        Ok(())
    }

    #[test]
    fn test_load_file_ttl() -> Result<(), String> {
        let mut r = Reasoner::new();
        r.load_file("rdfs.ttl")
    }

    #[test]
    fn test_load_file_n3() -> Result<(), String> {
        let mut r = Reasoner::new();
        r.load_file("Brick.n3")
    }

    #[test]
    fn test_cax_sco() -> Result<(), String> {
        let mut r = Reasoner::new();
        let trips = vec![
            ("Class2", RDFS_SUBCLASSOF, "Class1"),
            ("a", RDF_TYPE, "Class2")
        ];
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
        assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class1".to_string())));
        Ok(())
    }

    #[test]
    fn test_cax_eqc1() -> Result<(), String> {
        let mut r = Reasoner::new();
        let trips = vec![
            ("Class1", OWL_EQUIVALENTCLASS, "Class2"),
            ("a", RDF_TYPE, "Class1")
        ];
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
        assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class2".to_string())));
        Ok(())
    }

    #[test]
    fn test_cax_eqc2() -> Result<(), String> {
        let mut r = Reasoner::new();
        let trips = vec![
            ("Class1", OWL_EQUIVALENTCLASS, "Class2"),
            ("a", RDF_TYPE, "Class2")
        ];
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
        assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class1".to_string())));
        Ok(())
    }

    #[test]
    fn test_prp_fp() -> Result<(), String> {
        let mut r = Reasoner::new();
        let trips = vec![
            ("PRED", RDF_TYPE, OWL_FUNCPROP),
            ("SUB", "PRED", "OBJECT_1"),
            ("SUB", "PRED", "OBJECT_2"),
        ];
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
        for i in res.iter() {
            let (s, p, o) = i;
            println!("{} {} {}", s, p, o);
        }
        assert!(res.contains(&("OBJECT_1".to_string(), OWL_SAMEAS.to_string(), "OBJECT_2".to_string())));
        Ok(())
    }

    #[test]
    fn test_prp_ifp() -> Result<(), String> {
        let mut r = Reasoner::new();
        let trips = vec![
            ("PRED", RDF_TYPE, OWL_INVFUNCPROP),
            ("SUB_1", "PRED", "OBJECT"),
            ("SUB_2", "PRED", "OBJECT"),
        ];
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
        for i in res.iter() {
            let (s, p, o) = i;
            println!("{} {} {}", s, p, o);
        }
        assert!(res.contains(&("SUB_1".to_string(), OWL_SAMEAS.to_string(), "SUB_2".to_string())));
        Ok(())
    }

    #[test]
    fn test_spo1() -> Result<(), String> {
        let mut r = Reasoner::new();
        let trips = vec![
            ("p1", RDFS_SUBPROP, "p2"),
            ("x", "p1", "y"),
        ];
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
        for i in res.iter() {
            let (s, p, o) = i;
            println!("{} {} {}", s, p, o);
        }
        assert!(res.contains(&("x".to_string(), "p2".to_string(), "y".to_string())));
        Ok(())
    }

    #[test]
    fn test_prp_inv1() -> Result<(), String> {
        let mut r = Reasoner::new();
        let trips = vec![
            ("p1", OWL_INVERSEOF, "p2"),
            ("x", "p1", "y"),
        ];
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
        for i in res.iter() {
            let (s, p, o) = i;
            println!("{} {} {}", s, p, o);
        }
        assert!(res.contains(&("y".to_string(), "p2".to_string(), "x".to_string())));
        Ok(())
    }

    #[test]
    fn test_prp_symp() -> Result<(), String> {
        let mut r = Reasoner::new();
        let trips = vec![
            ("p", RDF_TYPE, OWL_SYMMETRICPROP),
            ("x", "p", "y"),
        ];
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
        for i in res.iter() {
            let (s, p, o) = i;
            println!("{} {} {}", s, p, o);
        }
        assert!(res.contains(&("y".to_string(), "p".to_string(), "x".to_string())));
        Ok(())
    }

    #[test]
    fn test_prp_eqp1() -> Result<(), String> {
        let mut r = Reasoner::new();
        let trips = vec![
            ("p1", OWL_EQUIVPROP, "p2"),
            ("x", "p1", "y"),
        ];
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
        for i in res.iter() {
            let (s, p, o) = i;
            println!("{} {} {}", s, p, o);
        }
        assert!(res.contains(&("x".to_string(), "p2".to_string(), "y".to_string())));
        Ok(())
    }
}
