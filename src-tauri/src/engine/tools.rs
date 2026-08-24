use serde::{Deserialize, Serialize};

/// Starlux's own names for what a run may reach. Each provider calls these
/// something different — `WebSearch`, `google_web_search`, nothing at all — so
/// the adapters translate and this is the only vocabulary the UI and the
/// database share.
pub const WEB_SEARCH: &str = "webSearch";
pub const WEB_FETCH: &str = "webFetch";

pub const ALL: [&str; 2] = [WEB_SEARCH, WEB_FETCH];

/// Searching and fetching are granted separately because the providers grant
/// them separately, and because one of the three has no search tool at all.
/// Chat-only is `default()`, which is what an absent setting reads back as.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Tools {
    pub web_search: bool,
    pub web_fetch: bool,
}

impl Tools {
    pub fn get(&self, id: &str) -> bool {
        match id {
            WEB_SEARCH => self.web_search,
            WEB_FETCH => self.web_fetch,
            _ => false,
        }
    }

    pub fn set(&mut self, id: &str, on: bool) {
        match id {
            WEB_SEARCH => self.web_search = on,
            WEB_FETCH => self.web_fetch = on,
            _ => {}
        }
    }

    /// Whether this run reaches the network at all, which is the one thing both
    /// windows show without naming the individual tools.
    pub fn any(&self) -> bool {
        self.web_search || self.web_fetch
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_is_granted_by_default() {
        let tools = Tools::default();
        assert!(!tools.any());
        assert!(ALL.iter().all(|id| !tools.get(id)));
    }

    /// The id crosses IPC as a string, so one nobody defined must not silently
    /// become a grant.
    #[test]
    fn an_unknown_id_grants_nothing() {
        let mut tools = Tools::default();
        tools.set("filesystem", true);
        assert_eq!(tools, Tools::default());
        assert!(!tools.get("filesystem"));
    }

    #[test]
    fn each_tool_is_granted_on_its_own() {
        let mut tools = Tools::default();
        tools.set(WEB_FETCH, true);
        assert_eq!(
            tools,
            Tools {
                web_search: false,
                web_fetch: true
            }
        );
        assert!(tools.any());
    }
}
