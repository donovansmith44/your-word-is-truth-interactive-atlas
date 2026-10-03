extern crate self as atlas_core;
#[path="/home/donovan/w/A-PROVENANCE-review/server/atlas-core/src/sources.rs"]
pub mod sources;
#[path="/home/donovan/w/A-PROVENANCE-review/server/atlas-etl/src/sources.rs"]
mod etl_sources;

use sources::{Confidence, ProvenanceEntry, ProvenanceTitles, SourceCategory, SourceEntry, SourcesDocument};

#[derive(serde::Serialize)]
struct Input<'a> {
    category:&'a Vec<SourceCategory>,
    source:&'a Vec<SourceEntry>,
    provenance:&'a Vec<ProvenanceEntry>,
}
fn input(doc:&SourcesDocument)->String {
    toml::to_string(&Input{category:&doc.categories,source:&doc.sources,provenance:&doc.provenances}).unwrap()
}
fn source(id:String,title:String)->SourceEntry {
    SourceEntry {id,category:"group".into(),title,what_it_is:String::new(),what_we_built:String::new(),license:"Public domain".into(),link:None,licenses_row_key:"held".into()}
}
fn registry(seed:u32)->SourcesDocument {
    SourcesDocument {
        categories:vec![SourceCategory{id:"group".into(),label:"Group".into()}],
        sources:vec![source(format!("source-{seed}"),format!("Title {seed}")),source(format!("other-{seed}"),format!("Other {seed}"))],
        provenances:vec![ProvenanceEntry{id:format!("provenance-{seed}"),source:format!("source-{seed}"),confidence:Confidence::Imported,locator:None}],
    }
}
fn main()->Result<(),Box<dyn std::error::Error>> {
    for seed in 0..32_u32 {
        let doc=registry(seed);
        let admitted=etl_sources::admit_sources(&input(&doc))?;
        let titles=admitted.provenance_titles()?;
        for locator in 0..32_u32 {
            assert_eq!(titles.title_of(&format!("provenance-{seed}/locator-{locator}/piece")),Some(format!("Title {seed}").as_str()));
        }
        let mut missing=doc.clone();
        missing.provenances[0].source=format!("missing-{seed}");
        assert!(etl_sources::admit_sources(&input(&missing)).is_err());
        let mut duplicate_source=doc.clone();
        duplicate_source.sources.push(source(format!("source-{seed}"),format!("Conflicting {seed}")));
        assert!(etl_sources::admit_sources(&input(&duplicate_source)).is_err());
        let mut duplicate_provenance=doc.clone();
        let mut extra=doc.provenances[0].clone();
        extra.source=format!("other-{seed}");
        duplicate_provenance.provenances.push(extra);
        assert!(etl_sources::admit_sources(&input(&duplicate_provenance)).is_err());
        let id=format!("provenance-{seed}");
        assert_eq!(ProvenanceTitles::from_rows(vec![(id.clone(),format!("Title {seed}")),(id.clone(),format!("Other {seed}"))]), Err(sources::DuplicateProvenance { id }));
    }
    println!("1024 generated locator-title cases pass; all prior 32 missing-source, 32 duplicate-source and 32 duplicate-provenance registries now refused by the compiler's exact admitted door.");
    println!("32 prior public-constructor conflicting-ID inputs now refused with the complete typed error.");
    let raw=std::fs::read_to_string("data/curated/sources.toml")?;
    let admitted=etl_sources::admit_sources(&raw)?;
    let doc=admitted.document();
    let titles=admitted.provenance_titles()?;
    let baseline:Vec<(String,String)>=titles.rows().map(|(id,t)|(id.into(),t.into())).collect();
    let mut refused=0;
    for p in &doc.provenances {
        let original=doc.sources.iter().find(|s|s.id==p.source).unwrap();
        let other=doc.sources.iter().find(|s|s.title!=original.title).unwrap();
        let duplicate=format!("{raw}\n[[provenance]]\nid = {:?}\nsource = {:?}\nconfidence = {:?}\n",p.id,other.id,p.confidence.name());
        assert!(etl_sources::admit_sources(&duplicate).is_err());
        let dangling=format!("{raw}\n[[provenance]]\nid = {:?}\nsource = \"no-such-source\"\nconfidence = {:?}\n",format!("{}-dangling",p.id),p.confidence.name());
        assert!(etl_sources::admit_sources(&dangling).is_err());
        refused+=2;
        let mut pair_rows=baseline.clone();
        pair_rows.push((p.id.clone(),other.title.clone()));
        assert_eq!(ProvenanceTitles::from_rows(pair_rows), Err(sources::DuplicateProvenance { id: p.id.clone() }));
    }
    for s in &doc.sources {
        let duplicate=format!("{raw}\n[[source]]\nid = {:?}\ncategory = {:?}\ntitle = \"Another title\"\nwhat_it_is = \"x\"\nwhat_we_built = \"y\"\nlicense = \"z\"\nlicenses_row_key = \"k\"\n",s.id,s.category);
        assert!(etl_sources::admit_sources(&duplicate).is_err());
        refused+=1;
    }
    assert_eq!(refused,doc.provenances.len()*2+doc.sources.len());
    println!("All {} full-curated-TOML ambiguous/dangling mutations now refused; all {} prior public-constructor mutations of its complete title map now refused.",refused,doc.provenances.len());
    Ok(())
}
