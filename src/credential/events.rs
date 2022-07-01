use serde::{Deserialize, Serialize};
use serde_json::Value;
use {futures::SinkExt, pharos::*};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum IssueCredentialEvent {
    IssueCredentialReceived { from: String, value: Value },
}

pub struct IssueCredentialEvents {
    pharos: Pharos<IssueCredentialEvent>,
}

impl Default for IssueCredentialEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl IssueCredentialEvents {
    pub fn new() -> Self {
        Self {
            pharos: Pharos::default(),
        }
    }
    pub async fn send(&mut self, event: IssueCredentialEvent) {
        self.pharos.send(event).await.expect("notify observers");
    }
}

impl Observable<IssueCredentialEvent> for IssueCredentialEvents {
    type Error = PharErr;

    fn observe(
        &mut self,
        options: ObserveConfig<IssueCredentialEvent>,
    ) -> Observe<'_, IssueCredentialEvent, Self::Error> {
        self.pharos.observe(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_issue_credential_events() {
        let mut issue_credential_events = IssueCredentialEvents::new();
        let mut events = issue_credential_events
            .observe(Channel::Bounded(3).into())
            .await
            .expect("observe");
        issue_credential_events
            .send(IssueCredentialEvent::IssueCredentialReceived {
                from: String::default(),
                value: Value::Null,
            })
            .await;
        let evt = dbg!(events.next().await.unwrap());
        drop(issue_credential_events);
        assert_eq!(
            IssueCredentialEvent::IssueCredentialReceived {
                from: String::default(),
                value: Value::Null
            },
            evt
        );
        assert_eq!(None, events.next().await);
    }
}
