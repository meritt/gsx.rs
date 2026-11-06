mod support;

use gsx_core::project::{Facts, project};
use gsx_core::targets::TARGETS;
use support::documents;

#[test]
fn emitted_changes_are_declared_by_the_table() {
    let documents = documents();
    let allowed = Facts { entities_allowed: true, caption_limit: Some(1024), ..Facts::default() };
    for target in TARGETS {
        for (document, facts) in
            documents.values().flat_map(|document| [(document, Facts::default()), (document, allowed)])
        {
            for change in project(document, target, &facts).changes {
                assert!(
                    target.declares(change.feature, change.class, change.reason),
                    "{} emits undeclared ({}, {}, {})",
                    target.name,
                    change.feature.name(),
                    change.class.name(),
                    change.reason
                );
            }
        }
    }
}
