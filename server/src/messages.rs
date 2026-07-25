use crate::events::CobaltServerEvent;

#[derive(Debug, Clone)]
pub enum CobaltCommand {
    Event(CobaltServerEvent),
}

#[derive(Debug, Clone)]
pub enum CobaltResponse {
    Acknowledged,
}

