//! Build-only inventory from actual admitted Core requests, never a second route table.
use govorun_core::stylization::{Request, reliability::Batch};
use serde_json::json;
use std::time::Duration;
pub fn schema() -> serde_json::Value {
    let pass = "schema-only-anonymous-pass".to_owned();
    let requests = [
        Request::start(json!({"text":"synthetic schema request with enough characters",
            "style":"formal","inputMode":"hold","punctuationMode":"normal",
            "reducedPunctuation":false,"emojiEnabled":false,"emojiMode":"off",
            "profanityFilterEnabled":false,"stylePasses":[pass.clone()]}),
            "00000000-0000-4000-8000-000000000001".into(), Duration::from_secs(1)),
        Request::status("schema-only-job".into(), Duration::from_secs(1)),
        Request::reconcile(vec![pass.clone()]), Request::retire(vec![pass.clone()]),
        Request::availability(), Request::outcome(pass,"delivered","lt1s"),
        Request::reliability(serde_json::from_value::<Batch>(json!({"schemaVersion":1,
            "failures":{"no_change":1}})).expect("fixed schema batch")),
    ];
    let paths: Vec<_> = requests.into_iter().map(|request|
        request.expect("fixed schema request").path()).collect();
    json!({"title":"OfficialStylePath","type":"string","enum":paths})
}
