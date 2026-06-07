use std::collections::HashMap;
use std::sync::Arc;

use agent_client_protocol::schema::{
    AgentCapabilities, InitializeRequest, InitializeResponse, NewSessionRequest,
    NewSessionResponse, SessionId,
};
use agent_client_protocol::{Agent, ConnectionTo, Dispatch, Stdio};

use parking_lot::Mutex;

use crate::loop_::Agent as KontrocodeAgent;

pub async fn run_acp_agent(_agent: KontrocodeAgent) -> agent_client_protocol::Result<()> {
    let sessions = Arc::new(Mutex::new(HashMap::<String, ()>::new()));

    Agent
        .builder()
        .name("kontrocode-agent")
        .on_receive_request(
            async move |req: InitializeRequest, responder, _cx| {
                tracing::info!("acp initialize: version={:?}", req.protocol_version);
                responder.respond(
                    InitializeResponse::new(req.protocol_version)
                        .agent_capabilities(AgentCapabilities::new()),
                )
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            {
                let s = Arc::clone(&sessions);
                async move |req: NewSessionRequest, responder, _cx| {
                    let sid = uuid::Uuid::new_v4().to_string();
                    s.lock().insert(sid.clone(), ());
                    tracing::info!("acp session/new: id={sid}");
                    responder.respond(NewSessionResponse::new(SessionId::from(sid)))
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_dispatch(
            async move |message: Dispatch, _cx: ConnectionTo<agent_client_protocol::Client>| {
                tracing::debug!("acp unhandled: method={}", message.method());
                let retry = message
                    .message()
                    .map(|m: &agent_client_protocol::UntypedMessage| {
                        m.params()
                            .get("sessionId")
                            .or_else(|| m.params().get("session_id"))
                            .is_some()
                    })
                    .unwrap_or(false);
                Ok(agent_client_protocol::Handled::No { message, retry })
            },
            agent_client_protocol::on_receive_dispatch!(),
        )
        .connect_to(Stdio::new())
        .await
}
