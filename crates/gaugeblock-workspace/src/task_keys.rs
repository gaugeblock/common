//! Which ticket a running process belongs to (`GB-356`).
//!
//! The mechanism is an environment variable, chosen for the reason `GAUGEBLOCK_TRACE_ID` is one: a parent that
//! *knows* the fact hands it to the child it spawns, and the child needs no flag, no store access and no
//! knowledge of storypole. A flag cannot do this for MCP servers, which have no per-call argv, and a claim
//! marker file would be a third copy of what the store already holds - the mistake that cost `GB-347` and
//! `GB-349` in one session.
//!
//! The risk of the mechanism is why this module is more than one `env::var` call. An exported variable outlives
//! the lease that justified it: a shell still holding yesterday's `GB-355` would attribute today's work to a
//! closed ticket, and the corpus cannot tell a lie from a fact. Three rules, all refusing rather than guessing:
//!
//! 1. **Bound to a lease.** `GAUGEBLOCK_TASK_EXPIRES_MS` must be present and in the future. A key with no
//!    expiry is a hand-set variable with nothing to expire it, so it is not attributable.
//! 2. **Well formed.** `PREFIX-NUMBER`, nothing else, because a key is an identifier rather than prose.
//! 3. **Plausible for this repository.** The prefix must be the repository's own or the suite's harness prefix.
//!    A `GB-` ticket may touch any member - that is what the harness is for - but `SP-49` turning up in `rabbet`
//!    is a stale export from another repository, and it is refused.
//!
//! Refusal is never an error: telemetry that can fail a command is worse than telemetry that is missing one
//! field. It is reported to stderr instead, so a human whose attribution vanished can see why.

use std::path::Path;

use crate::{GaugeblockPaths, Membership, SuiteConfig, now_ms};

/// The claimed ticket this process is working on.
pub const TASK_KEY_ENV: &str = "GAUGEBLOCK_TASK_KEY";

/// When that claim's lease expires, in epoch milliseconds. Required, and required to be future.
pub const TASK_EXPIRES_ENV: &str = "GAUGEBLOCK_TASK_EXPIRES_MS";

/// Why a key was refused. Reported, because silent refusal looks like a bug in the emitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The key was set but carried no expiry, so nothing bounds it.
    NoExpiry,
    /// The lease behind the key has already run out.
    Expired,
    /// Not `PREFIX-NUMBER`.
    Malformed,
    /// A real key, but for a repository this one is not.
    WrongRepository,
}

impl Refusal {
    /// One line a human can act on.
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Self::NoExpiry => {
                "no GAUGEBLOCK_TASK_EXPIRES_MS: a key with no lease is a leftover, so it is not attributed"
            }
            Self::Expired => "the claim's lease has expired, so the key is not attributed",
            Self::Malformed => "the key is not PREFIX-NUMBER, so it is not attributed",
            Self::WrongRepository => {
                "the key's prefix belongs to neither this repository nor the suite, so it is not attributed"
            }
        }
    }
}

/// The outcome of resolving the ambient key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attribution {
    /// A key that passed every rule.
    Attributed(String),
    /// A key that was present and refused, with the reason.
    Refused(Refusal),
    /// Nothing was claimed: an unclaimed run, which is honest rather than a gap.
    Absent,
}

impl Attribution {
    /// The key to record, if any.
    #[must_use]
    pub fn key(&self) -> Option<&str> {
        match self {
            Self::Attributed(key) => Some(key),
            Self::Refused(_) | Self::Absent => None,
        }
    }
}

/// Resolves the ambient key for a process started in `start`.
///
/// Reads the environment and, when a key is present, the configuration needed to judge it. The judgement
/// itself is [`verdict`], which is pure and carries the tests: `set_var` is unsafe in edition 2024 and this
/// workspace denies `unsafe_code`, so a test cannot plant a variable while other tests read the environment.
#[must_use]
pub fn ambient(start: &Path) -> Attribution {
    let Ok(raw) = std::env::var(TASK_KEY_ENV) else {
        return Attribution::Absent;
    };
    let raw = raw.trim().to_string();
    let expires = std::env::var(TASK_EXPIRES_ENV)
        .ok()
        .and_then(|expires| expires.trim().parse::<u64>().ok());
    let attribution = verdict(&raw, expires, now_ms(), &allowed_prefixes(start));
    if let Attribution::Refused(refusal) = attribution {
        // The one line this crate writes to stderr, and the only place a refusal is visible: without it, a human
        // whose attribution silently vanished cannot tell a refused key from a broken emitter. stderr rather
        // than a logger because this crate carries no `tracing` dependency and should not grow one for a single
        // diagnostic; stderr rather than stdout because `--json` and MCP frames must stay clean.
        eprintln!(
            "gaugeblock: {TASK_KEY_ENV} is set but not attributed - {}",
            refusal.reason()
        );
    }
    attribution
}

/// The judgement, with every input explicit.
///
/// Split from [`ambient`] so the rules can be tested at every boundary: a future lease, a lease that expired a
/// second ago, no lease at all, junk, another repository's prefix, and a harness key inside a member (allowed,
/// because harness work happens in members all the time).
#[must_use]
pub fn verdict(key: &str, expires_ms: Option<u64>, now_ms: u64, allowed: &[String]) -> Attribution {
    if !is_well_formed(key) {
        return Attribution::Refused(Refusal::Malformed);
    }
    match expires_ms {
        None => return Attribution::Refused(Refusal::NoExpiry),
        Some(expires) if expires <= now_ms => return Attribution::Refused(Refusal::Expired),
        Some(_) => {}
    }
    // An empty prefix set means the configurations could not be read, not that nothing is allowed. Refusing
    // then would discard attribution over a missing file, so the rule degrades to "not checked" - and the
    // caller passing an empty list is the one place that degradation is visible.
    if !allowed.is_empty()
        && !allowed
            .iter()
            .any(|prefix| key.starts_with(prefix.as_str()))
    {
        return Attribution::Refused(Refusal::WrongRepository);
    }
    Attribution::Attributed(key.to_string())
}

/// `PREFIX-NUMBER`, where the prefix is 1-6 upper-case alphanumerics starting with a letter.
fn is_well_formed(key: &str) -> bool {
    let Some((prefix, number)) = key.split_once('-') else {
        return false;
    };
    let mut chars = prefix.chars();
    let starts_with_letter = chars.next().is_some_and(|c| c.is_ascii_uppercase());
    starts_with_letter
        && prefix.len() <= 6
        && chars.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && !number.is_empty()
        && number.chars().all(|c| c.is_ascii_digit())
}

/// The prefixes this repository may legitimately carry: its own, and the suite's harness prefix.
///
/// Empty when neither configuration can be read, which the caller treats as "do not check" rather than
/// "allowed nothing".
#[must_use]
pub fn allowed_prefixes(start: &Path) -> Vec<String> {
    let mut prefixes = Vec::new();
    let canonical = std::fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());
    if let Some(membership) = Membership::containing(&canonical) {
        // The harness prefix: a `GB-` ticket may be worked on in any member, which is what the harness is for.
        if let Ok(config) = SuiteConfig::load(&membership.suite_root)
            && let Some(prefix) = config.string("storypole", "ticket_prefix")
        {
            prefixes.push(prefix.to_ascii_uppercase());
        }
    }
    // The repository's own prefix. `discover` resolves the checkout this process is in, which is exactly the
    // question here - the opposite of the spool and sink resolution, where the suite is the right answer.
    if let Ok(paths) = GaugeblockPaths::discover(&canonical) {
        let repo_root = paths.root().parent().unwrap_or(paths.root());
        if let Ok(config) = SuiteConfig::load(repo_root)
            && let Some(prefix) = config.string("storypole", "ticket_prefix")
        {
            prefixes.push(prefix.to_ascii_uppercase());
        }
    }
    prefixes.sort();
    prefixes.dedup();
    prefixes
}

#[cfg(test)]
mod tests;
