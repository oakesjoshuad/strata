use crate::{RecordKind, Status};

pub(super) fn valid_next_statuses(kind: RecordKind, from: Status) -> Vec<Status> {
    let transitions = match kind {
        RecordKind::Rfc => &[
            (Status::Draft, Status::Proposed),
            (Status::Proposed, Status::UnderReview),
            (Status::UnderReview, Status::Accepted),
            (Status::UnderReview, Status::Withdrawn),
        ][..],
        RecordKind::Pdr | RecordKind::Specification => &[
            (Status::Draft, Status::Review),
            (Status::Review, Status::Approved),
            (Status::Approved, Status::Superseded),
        ][..],
        RecordKind::Adr => &[
            (Status::Draft, Status::Proposed),
            (Status::Proposed, Status::Accepted),
            (Status::Accepted, Status::Deprecated),
            (Status::Accepted, Status::Superseded),
            (Status::Deprecated, Status::Superseded),
        ][..],
        RecordKind::Edr => &[
            (Status::Draft, Status::Proposed),
            (Status::Proposed, Status::Accepted),
            (Status::Accepted, Status::Superseded),
        ][..],
        RecordKind::Assessment | RecordKind::Research | RecordKind::Glossary => &[
            (Status::Draft, Status::Accepted),
            (Status::Accepted, Status::Superseded),
        ][..],
        RecordKind::Risk => &[
            (Status::Open, Status::Monitoring),
            (Status::Open, Status::Mitigated),
            (Status::Open, Status::Accepted),
            (Status::Open, Status::Materialized),
            (Status::Monitoring, Status::Mitigated),
            (Status::Monitoring, Status::Accepted),
            (Status::Monitoring, Status::Materialized),
            (Status::Accepted, Status::Materialized),
            (Status::Mitigated, Status::Closed),
            (Status::Accepted, Status::Closed),
            (Status::Materialized, Status::Closed),
            (Status::Mitigated, Status::Open),
            (Status::Closed, Status::Open),
        ][..],
    };
    transitions
        .iter()
        .filter_map(|(current, next)| (*current == from).then_some(*next))
        .collect()
}
