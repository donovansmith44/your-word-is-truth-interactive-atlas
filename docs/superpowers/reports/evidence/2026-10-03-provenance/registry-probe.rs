extern crate self as atlas_core;
#[path="/home/donovan/w/A-PROVENANCE-review/server/atlas-core/src/sources.rs"]
pub mod sources;
#[path="/home/donovan/w/A-PROVENANCE-review/server/atlas-etl/src/sources.rs"]
mod etl_sources;

use sources::{Confidence, ProvenanceEntry, SourceCategory, SourceEntry, SourcesDocument};

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
        etl_sources::validate_structure(&doc)?;
        let titles=doc.provenance_titles()?;
        for locator in 0..32_u32 {
            let id=format!("provenance-{seed}/locator-{locator}/piece");
            assert_eq!(titles.title_of(&id),Some(format!("Title {seed}").as_str()));
        }
        let mut missing=doc.clone();
        missing.provenances[0].source=format!("missing-{seed}");
        assert!(missing.provenance_titles().is_err());
        let mut duplicate_source=doc.clone();
        duplicate_source.sources.push(source(format!("source-{seed}"),format!("Conflicting {seed}")));
        assert!(etl_sources::validate_structure(&duplicate_source).is_err());
        let admitted=duplicate_source.provenance_titles()?;
        assert_eq!(admitted.title_of(&format!("provenance-{seed}")),Some(format!("Title {seed}").as_str()));
        let mut duplicate_provenance=doc.clone();
        let mut extra=doc.provenances[0].clone();
        extra.source=format!("other-{seed}");
        duplicate_provenance.provenances.push(extra);
        assert!(etl_sources::validate_structure(&duplicate_provenance).is_err());
        let admitted=duplicate_provenance.provenance_titles()?;
        assert_eq!(admitted.title_of(&format!("provenance-{seed}")),Some(format!("Other {seed}").as_str()));
    }
    println!("1024 generated source/locator title resolutions match the named source; 32 missing-source cases refused.");
    println!("32 duplicate source-id registries: existing validate_structure refuses; new provenance_titles accepts and takes first source title.");
    println!("32 duplicate provenance-id registries: existing validate_structure refuses; new provenance_titles accepts and silently takes last mapping/title.");
    let actual_input=std::fs::read_to_string("data/curated/sources.toml")?;
    let actual=etl_sources::parse_sources(&actual_input)?;
    etl_sources::validate_structure(&actual)?;
    let titles=actual.provenance_titles()?;
    for p in &actual.provenances {
        let declared=actual.sources.iter().find(|s|s.id==p.source).unwrap();
        assert_eq!(titles.title_of(&p.id),Some(declared.title.as_str()));
    }
    let mut full_cases=0;
    for p in &actual.provenances {
        let declared=actual.sources.iter().find(|s|s.id==p.source).unwrap();
        let other=actual.sources.iter().find(|s|s.title!=declared.title).unwrap();
        let changed_input=format!("{actual_input}\n[[provenance]]\nid = {:?}\nsource = {:?}\nconfidence = {:?}\n",p.id,other.id,p.confidence.name());
        let changed=etl_sources::parse_sources(&changed_input)?;
        assert!(etl_sources::validate_structure(&changed).is_err());
        let accepted=changed.provenance_titles()?;
        assert_eq!(accepted.title_of(&p.id),Some(other.title.as_str()));
        assert_ne!(accepted.title_of(&p.id),Some(declared.title.as_str()));
        if full_cases==0 {println!("Full TOML reproduction: {} originally names {:?}; appended duplicate points to {} and admitted title becomes {:?}.",p.id,declared.title,other.id,accepted.title_of(&p.id));}
        full_cases+=1;
    }
    println!("{} generated mutations of the complete current curated registry: duplicate existing provenance switches its served title to a different declared source; new compile titling accepts, existing admission rejects.",full_cases);
    println!("Current curated registry: {} sources, {} provenances; every complete title mapping matches its declared source.",actual.sources.len(),actual.provenances.len());
    Ok(())
}
