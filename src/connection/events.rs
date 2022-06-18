use serde::{Deserialize, Serialize};
use {futures::SinkExt, pharos::*};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConnectionEvent {
    Created(String),
}

pub struct ConnectionEvents {
    pharos: Pharos<ConnectionEvent>,
}

impl ConnectionEvents {
    pub fn new() -> Self {
        Self {
            pharos: Pharos::default(),
        }
    }
    pub async fn send(&mut self, event: ConnectionEvent) {
        self.pharos.send(event).await.expect("notify observers");
    }
}

impl Observable<ConnectionEvent> for ConnectionEvents {
    type Error = PharErr;

    fn observe(
        &mut self,
        options: ObserveConfig<ConnectionEvent>,
    ) -> Observe<'_, ConnectionEvent, Self::Error> {
        self.pharos.observe(options)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_connection_events() {
        let mut connection_events = ConnectionEvents::new();
        let mut events = connection_events
            .observe(Channel::Bounded(3).into())
            .await
            .expect("observe");
        connection_events
            .send(ConnectionEvent::Created(String::default()))
            .await;
        let evt = dbg!(events.next().await.unwrap());
        drop(connection_events);
        assert_eq!(ConnectionEvent::Created(String::default()), evt);
        assert_eq!(None, events.next().await);
    }
}
