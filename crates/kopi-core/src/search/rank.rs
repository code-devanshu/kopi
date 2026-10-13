//! Ordering index matches: how closely the clip is the text searched for, blended with how
//! recently it was used.
//!
//! FTS5's `bm25()` counts every match in the index before it scores one, to weigh rare words
//! above common ones. For a fragment in most of the history that costs 15–30 ms at 100,000
//! clips, whatever the `LIMIT`. So the matches are scored here instead, with bm25's own
//! length normalisation and FTS5's constants, counting each match once. A short clip that is
//! mostly the search scores higher than a long text that mentions it.

/// bm25's term-frequency saturation, as FTS5 sets it.
const K1: f64 = 1.2;

/// bm25's length normalisation, as FTS5 sets it.
const B: f64 = 0.75;

/// Stands in for the average clip length that bm25 would count, in bytes. The 100,000 clips
/// in the storage benchmark average 1.3 KB. Short clips all score about the same against it,
/// so recency orders them, while long pastes rank lower.
const AVERAGE_LENGTH: f64 = 1024.0;

/// A clip used this long before the newest match counts half as recent.
const RECENCY_HALF_LIFE_MS: f64 = 3.0 * 24.0 * 60.0 * 60.0 * 1000.0;

/// An index match before it's ranked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Candidate {
    pub id: i64,
    pub last_used_at: i64,
    /// The bytes of indexed text: the clip's text and its app's name.
    pub length: i64,
}

/// The candidates' ids, best first.
///
/// Each clip scores relevance plus recency, both from 0 to 1:
/// - relevance is bm25 for one match in a clip of this length, as a share of the best among
///   the candidates;
/// - recency halves every [`RECENCY_HALF_LIFE_MS`] before the newest candidate, so time spent
///   away from the computer doesn't age everything at once.
///
/// So the same match ranks a recent clip first, and a close match from a day ago beats a long
/// text that mentions it just now. Ties go to the more recent clip.
pub(super) fn rank(candidates: Vec<Candidate>) -> Vec<i64> {
    let Some(newest) = candidates.iter().map(|c| c.last_used_at).max() else {
        return Vec::new();
    };
    let bm25 = |c: &Candidate| {
        let length = c.length.max(1) as f64;
        (K1 + 1.0) / (1.0 + K1 * (1.0 - B + B * length / AVERAGE_LENGTH))
    };
    let best = candidates.iter().map(bm25).fold(0.0, f64::max);

    let mut scored: Vec<(f64, Candidate)> = candidates
        .into_iter()
        .map(|c| {
            let age = newest.saturating_sub(c.last_used_at) as f64;
            let score = bm25(&c) / best + 0.5f64.powf(age / RECENCY_HALF_LIFE_MS);
            (score, c)
        })
        .collect();
    scored.sort_by(|(a, ca), (b, cb)| {
        b.total_cmp(a)
            .then(cb.last_used_at.cmp(&ca.last_used_at))
            .then(cb.id.cmp(&ca.id))
    });
    scored.into_iter().map(|(_, c)| c.id).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: i64 = 60 * 60 * 1000;
    const DAY: i64 = 24 * HOUR;
    const NOW: i64 = 1_760_000_000_000;

    fn c(id: i64, length: i64, ago: i64) -> Candidate {
        Candidate {
            id,
            last_used_at: NOW - ago,
            length,
        }
    }

    #[test]
    fn orders() {
        let cases: &[(&str, &[Candidate], &[i64])] = &[
            ("nothing", &[], &[]),
            ("one", &[c(1, 20, 0)], &[1]),
            (
                "the same length: newest first",
                &[c(1, 20, 2 * DAY), c(2, 20, 0), c(3, 20, HOUR)],
                &[2, 3, 1],
            ),
            (
                "the same time: shortest first",
                &[c(1, 900, 0), c(2, 10, 0), c(3, 90, 0)],
                &[2, 3, 1],
            ),
            (
                "a close match from a day ago beats a long mention now",
                &[c(1, 2000, 0), c(2, 10, DAY)],
                &[2, 1],
            ),
            (
                "a long mention now beats a close match from a week ago",
                &[c(1, 2000, 0), c(2, 10, 7 * DAY)],
                &[1, 2],
            ),
            (
                "between clips of about the same length, an hour counts more",
                &[c(1, 30, HOUR), c(2, 40, 0)],
                &[2, 1],
            ),
            (
                "a few hundred bytes now beats a close match from a day ago",
                &[c(1, 10, DAY), c(2, 200, 0)],
                &[2, 1],
            ),
            (
                "full ties go to the higher id",
                &[c(1, 20, 0), c(2, 20, 0)],
                &[2, 1],
            ),
            (
                "ages count from the newest match, not from now",
                &[c(1, 10, 31 * DAY), c(2, 2000, 30 * DAY)],
                &[1, 2],
            ),
            (
                "no text counts as one byte",
                &[c(1, 0, HOUR), c(2, 0, 0), c(3, -5, 2 * HOUR)],
                &[2, 1, 3],
            ),
        ];
        for (name, candidates, want) in cases {
            assert_eq!(rank(candidates.to_vec()), *want, "{name}");
        }
    }
}
