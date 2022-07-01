use serde::{Deserialize, Serialize};
use serde_json::Value;
use {futures::SinkExt, pharos::*};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PresentProofEvent {
    ProofReceived(String, Value),
}

pub struct PresentProofEvents {
    pharos: Pharos<PresentProofEvent>,
}

impl Default for PresentProofEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl PresentProofEvents {
    pub fn new() -> Self {
        Self {
            pharos: Pharos::default(),
        }
    }
    pub async fn send(&mut self, event: PresentProofEvent) {
        self.pharos.send(event).await.expect("notify observers");
    }
}

impl Observable<PresentProofEvent> for PresentProofEvents {
    type Error = PharErr;

    fn observe(
        &mut self,
        options: ObserveConfig<PresentProofEvent>,
    ) -> Observe<'_, PresentProofEvent, Self::Error> {
        self.pharos.observe(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_present_proof_events() {
        let mut present_proof_events = PresentProofEvents::new();
        let mut events = present_proof_events
            .observe(Channel::Bounded(3).into())
            .await
            .expect("observe");
        present_proof_events
            .send(PresentProofEvent::ProofReceived(
                String::default(),
                Value::Null,
            ))
            .await;
        let evt = dbg!(events.next().await.unwrap());
        drop(present_proof_events);
        assert_eq!(
            PresentProofEvent::ProofReceived(String::default(), Value::Null),
            evt
        );
        assert_eq!(None, events.next().await);
    }
}
