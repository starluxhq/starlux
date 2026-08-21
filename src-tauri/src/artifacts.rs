//! Storage behind the `artifact:` scheme.
//!
//! Interactive answers are served as real documents rather than `srcdoc`, which
//! inherits the host page's CSP and so could never run a script the app itself
//! forbids. A document fetched over its own scheme carries its own policy, and
//! the one below allows the artifact to run while denying it the network.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::Mutex;

use tauri::http;

/// Scripts and styles have to be inline because the whole artifact is one file,
/// but `connect-src 'none'` means nothing it was given can leave the machine.
const POLICY: &str = "default-src 'none'; \
script-src 'unsafe-inline' 'unsafe-eval'; \
style-src 'unsafe-inline'; \
img-src data: blob:; \
font-src data:; \
media-src data: blob:; \
connect-src 'none'; \
form-action 'none'; \
base-uri 'none'";

/// Enough for the artifacts on screen and a little history behind them; the
/// store is a render cache, not a record. Conversations keep the source.
const CAPACITY: usize = 32;

#[derive(Default)]
pub struct Artifacts {
    documents: Mutex<HashMap<String, String>>,
    order: Mutex<VecDeque<String>>,
}

impl Artifacts {
    /// Keyed by content, so re-rendering an unchanged artifact resolves to the
    /// same URL and the frame is not torn down and reloaded.
    pub fn put(&self, html: String) -> String {
        let mut hasher = DefaultHasher::new();
        html.hash(&mut hasher);
        let id = format!("{:016x}", hasher.finish());

        let mut documents = self.documents.lock().unwrap();
        let mut order = self.order.lock().unwrap();

        if documents.insert(id.clone(), html).is_none() {
            order.push_back(id.clone());
            while order.len() > CAPACITY {
                if let Some(evicted) = order.pop_front() {
                    documents.remove(&evicted);
                }
            }
        }

        id
    }

    pub fn get(&self, id: &str) -> Option<String> {
        self.documents.lock().unwrap().get(id).cloned()
    }
}

pub fn response(store: &Artifacts, path: &str) -> http::Response<Vec<u8>> {
    let id = path.trim_start_matches('/');

    let Some(html) = store.get(id) else {
        return http::Response::builder()
            .status(404)
            .body(Vec::new())
            .expect("static response builds");
    };

    http::Response::builder()
        .status(200)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Content-Security-Policy", POLICY)
        .body(html.into_bytes())
        .expect("static response builds")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_document_keeps_the_same_id() {
        let store = Artifacts::default();
        assert_eq!(store.put("<p>hi</p>".into()), store.put("<p>hi</p>".into()));
        assert_ne!(store.put("<p>hi</p>".into()), store.put("<p>ho</p>".into()));
    }

    #[test]
    fn the_oldest_artifact_is_evicted_first() {
        let store = Artifacts::default();
        let first = store.put("<p>0</p>".into());
        for n in 1..=CAPACITY {
            store.put(format!("<p>{n}</p>"));
        }
        assert!(store.get(&first).is_none());
        assert!(store
            .get(&store.put(format!("<p>{CAPACITY}</p>")))
            .is_some());
    }

    #[test]
    fn a_missing_artifact_is_not_served() {
        let store = Artifacts::default();
        assert_eq!(response(&store, "/nope").status(), 404);
    }

    #[test]
    fn a_served_artifact_cannot_reach_the_network() {
        let store = Artifacts::default();
        let id = store.put("<p>hi</p>".into());
        let served = response(&store, &format!("/{id}"));

        assert_eq!(served.status(), 200);
        let policy = served
            .headers()
            .get("Content-Security-Policy")
            .expect("artifacts are served with a policy")
            .to_str()
            .unwrap();
        assert!(policy.contains("connect-src 'none'"));
        assert!(policy.contains("default-src 'none'"));
    }
}
