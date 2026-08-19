use super::*;
use crate::render::render_record;
use records::RecordKind;
use store::Store;

fn rejected_input(input: String, expected: &str) {
    let mut store = Store::open_memory().expect("store");
    let record = store
        .create(
            RecordKind::Adr,
            "Parser test",
            RecordKind::Adr.default_document("Parser test"),
        )
        .expect("record");
    let before = store.get(&record.id).expect("before");
    let history_before = store.history(&record.id).expect("history").len();
    let error = match parse("broken.md", RecordKind::Adr, &input) {
        Ok(document) => {
            store
                .revise(&record.id, document, None)
                .expect("unexpected success");
            panic!("malformed input was accepted")
        }
        Err(error) => error,
    };
    assert!(error.to_string().contains(expected));
    let after = store.get(&record.id).expect("after");
    assert_eq!(before.document, after.document);
    assert_eq!(before.revision, after.revision);
    assert_eq!(
        history_before,
        store.history(&record.id).expect("history").len()
    );
}

fn valid_markdown() -> String {
    let mut store = Store::open_memory().expect("store");
    let record = store
        .create(
            RecordKind::Adr,
            "Parser test",
            RecordKind::Adr.default_document("Parser test"),
        )
        .expect("record");
    render_record(&store, &record.id).expect("render")
}

#[test]
fn missing_section_is_explicit_and_non_mutating() {
    let mut input = valid_markdown();
    let marker = "## Evidence";
    let index = input.find(marker).expect("evidence heading");
    input.truncate(index);
    rejected_input(input, "missing required section 'evidence'");
}

#[test]
fn malformed_frontmatter_is_explicit_and_non_mutating() {
    let input = valid_markdown().replace("status: proposed", "status: [broken");
    rejected_input(input, "frontmatter is malformed");
}

#[test]
fn wrong_record_type_is_explicit_and_non_mutating() {
    let input = valid_markdown().replace("record-type: adr", "record-type: rfc");
    rejected_input(input, "frontmatter record-type is 'rfc', expected 'adr'");
}
