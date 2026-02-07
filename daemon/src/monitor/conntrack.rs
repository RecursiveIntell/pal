#[derive(Debug, Clone)]
pub struct ConntrackEvent {
    pub src: String,
    pub dst: String,
    pub protocol: String,
}
