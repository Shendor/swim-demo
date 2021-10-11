pub mod swim_node {
    use crate::member_node::swim_node::MemberNodeDetails;

    pub enum Message {
        Request(MemberNodeDetails, String),
        Response(MemberNodeDetails, String),
        Ping(MemberNodeDetails, Option<MemberNodeDetails>),
        PingResponse(u16, Option<MemberNodeDetails>, bool),
        ProbeRequest(MemberNodeDetails, u16),
        ProbeResponse(u16, bool),
        Shutdown(),
    }
}