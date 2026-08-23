use serde::{Deserialize, Serialize};

pub const RING_LEN: usize = 10;
pub const TTFT_SPIKE_DELTA_MS: u64 = 500;
pub const SAMPLE_RETENTION_MS: u64 = 90 * 86_400_000;
const HOUR_MS: u64 = 3_600_000;
const DAY_MS: u64 = 86_400_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenCounts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

impl TokenCounts {
    pub fn missing() -> Self {
        Self::default()
    }

    pub fn merge(self, other: Self) -> Self {
        Self {
            input: add_field(self.input, other.input),
            output: add_field(self.output, other.output),
            cache_read: add_field(self.cache_read, other.cache_read),
            cache_write: add_field(self.cache_write, other.cache_write),
            reasoning: add_field(self.reasoning, other.reasoning),
            total: add_field(self.total, other.total),
        }
    }

    pub fn sum_rows(rows: impl IntoIterator<Item = Self>) -> Self {
        let mut iter = rows.into_iter();
        let Some(first) = iter.next() else {
            return Self::missing();
        };
        iter.fold(first, Self::merge)
    }

    pub fn cache_hit_rate(&self) -> Option<f64> {
        match (self.cache_read, self.input) {
            (Some(cache_read), Some(input)) if cache_read + input > 0 => {
                Some(cache_read as f64 / (cache_read + input) as f64)
            }
            _ => None,
        }
    }
}

fn add_field(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    Some(left? + right?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TelemetryLane {
    Main,
    Subagent,
    Switched,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetrySample {
    pub run_id: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub agent_id: String,
    pub model: String,
    pub lane: TelemetryLane,
    #[serde(default)]
    pub tokens: TokenCounts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_per_sec: Option<u64>,
    #[serde(default)]
    pub at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UsageRange {
    #[default]
    Today,
    #[serde(rename = "24-hours")]
    Last24Hours,
    #[serde(rename = "7-days")]
    Last7Days,
    #[serde(rename = "30-days")]
    Last30Days,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BucketKind {
    Hour,
    Day,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl Default for UsageFilter {
    fn default() -> Self {
        Self {
            project_id: None,
            agent_id: None,
            model: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageQuery {
    pub range: UsageRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_from_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_to_ms: Option<u64>,
    #[serde(default)]
    pub filter: UsageFilter,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlighted_run_id: Option<String>,
}

impl Default for UsageQuery {
    fn default() -> Self {
        Self {
            range: UsageRange::Today,
            custom_from_ms: None,
            custom_to_ms: None,
            filter: UsageFilter::default(),
            highlighted_run_id: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageRunRow {
    pub run_id: String,
    pub project_id: String,
    pub project_name: String,
    pub agent_id: String,
    pub agent_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue_id: Option<String>,
    pub started_at_ms: u64,
    pub models: Vec<String>,
    pub tokens: TokenCounts,
    pub highlighted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageBucket {
    pub start_ms: u64,
    pub tokens: TokenCounts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_per_sec: Option<u64>,
    pub slow: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsagePage {
    pub range: UsageRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_from_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_to_ms: Option<u64>,
    pub filter: UsageFilter,
    pub bucket_kind: BucketKind,
    pub from_ms: u64,
    pub to_ms: u64,
    pub runs: Vec<UsageRunRow>,
    pub buckets: Vec<UsageBucket>,
    pub totals: TokenCounts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_hit_rate: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub highlighted_run_id: Option<String>,
    pub projects: Vec<UsageOption>,
    pub agents: Vec<UsageOption>,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageOption {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryPoint {
    pub at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_per_sec: Option<u64>,
    pub spike: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTelemetryLane {
    pub model: String,
    pub lane: TelemetryLane,
    pub tokens: TokenCounts,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttft_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_per_sec: Option<u64>,
    pub recent: Vec<TelemetryPoint>,
    pub spike: bool,
}

#[derive(Debug, Clone)]
pub struct UsageRun {
    pub id: String,
    pub project_id: String,
    pub project_name: String,
    pub agent_id: String,
    pub agent_name: String,
    pub issue_id: Option<String>,
    pub started_at_ms: u64,
}

pub fn local_offset_secs() -> i64 {
    #[cfg(unix)]
    {
        unix_offset_secs()
    }
    #[cfg(windows)]
    {
        windows_offset_secs()
    }
    #[cfg(not(any(unix, windows)))]
    {
        0
    }
}

#[cfg(unix)]
fn unix_offset_secs() -> i64 {
    #[repr(C)]
    struct Tm {
        tm_sec: i32,
        tm_min: i32,
        tm_hour: i32,
        tm_mday: i32,
        tm_mon: i32,
        tm_year: i32,
        tm_wday: i32,
        tm_yday: i32,
        tm_isdst: i32,
        tm_gmtoff: i64,
        tm_zone: *const i8,
    }
    extern "C" {
        fn time(tloc: *mut i64) -> i64;
        fn localtime_r(timep: *const i64, result: *mut Tm) -> *mut Tm;
    }
    unsafe {
        let mut t = 0i64;
        time(&mut t);
        let mut tm = std::mem::zeroed();
        if localtime_r(&t, &mut tm).is_null() {
            0
        } else {
            tm.tm_gmtoff
        }
    }
}

#[cfg(windows)]
fn windows_offset_secs() -> i64 {
    #[repr(C)]
    struct TimeZoneInformation {
        bias: i32,
        standard_name: [u16; 32],
        standard_date: [u16; 8],
        standard_bias: i32,
        daylight_name: [u16; 32],
        daylight_date: [u16; 8],
        daylight_bias: i32,
    }
    extern "system" {
        fn GetTimeZoneInformation(info: *mut TimeZoneInformation) -> u32;
    }
    const TIME_ZONE_ID_DAYLIGHT: u32 = 2;
    unsafe {
        let mut info = std::mem::zeroed();
        let kind = GetTimeZoneInformation(&mut info);
        let mut bias = info.bias;
        if kind == TIME_ZONE_ID_DAYLIGHT {
            bias += info.daylight_bias;
        } else {
            bias += info.standard_bias;
        }
        -(bias as i64) * 60
    }
}

pub fn window(query: &UsageQuery, now_ms: u64, offset_secs: i64) -> (u64, u64) {
    match query.range {
        UsageRange::Today => (day_start_ms(now_ms, offset_secs), now_ms),
        UsageRange::Last24Hours => (now_ms.saturating_sub(24 * HOUR_MS), now_ms),
        UsageRange::Last7Days => (now_ms.saturating_sub(7 * DAY_MS), now_ms),
        UsageRange::Last30Days => (now_ms.saturating_sub(30 * DAY_MS), now_ms),
        UsageRange::Custom => {
            let from = query.custom_from_ms.unwrap_or(0);
            let to = query.custom_to_ms.unwrap_or(now_ms).max(from);
            (from, to)
        }
    }
}

pub fn bucket_kind(range: UsageRange, from_ms: u64, to_ms: u64) -> BucketKind {
    match range {
        UsageRange::Today | UsageRange::Last24Hours => BucketKind::Hour,
        UsageRange::Last7Days | UsageRange::Last30Days => BucketKind::Day,
        UsageRange::Custom => {
            if to_ms.saturating_sub(from_ms) <= 24 * HOUR_MS {
                BucketKind::Hour
            } else {
                BucketKind::Day
            }
        }
    }
}

pub fn day_start_ms(now_ms: u64, offset_secs: i64) -> u64 {
    let offset_ms = offset_secs.saturating_mul(1000);
    let local = now_ms as i64 + offset_ms;
    let day = local.div_euclid(DAY_MS as i64);
    (day * DAY_MS as i64 - offset_ms).max(0) as u64
}

fn bucket_start_ms(at_ms: u64, kind: BucketKind, offset_secs: i64) -> u64 {
    match kind {
        BucketKind::Hour => (at_ms / HOUR_MS) * HOUR_MS,
        BucketKind::Day => day_start_ms(at_ms, offset_secs),
    }
}

fn bucket_step(kind: BucketKind) -> u64 {
    match kind {
        BucketKind::Hour => HOUR_MS,
        BucketKind::Day => DAY_MS,
    }
}

pub fn median_u64(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();
    if n % 2 == 1 {
        Some(sorted[n / 2])
    } else {
        Some((sorted[n / 2 - 1] + sorted[n / 2]) / 2)
    }
}

pub fn ttft_spike(current: u64, baseline: u64) -> bool {
    current.saturating_mul(2) > baseline.saturating_mul(5)
        && current.saturating_sub(baseline) > TTFT_SPIKE_DELTA_MS
}

pub fn rate_spike(current: u64, baseline: u64) -> bool {
    baseline > 0 && current.saturating_mul(5) < baseline.saturating_mul(2)
}

pub fn sample_is_spike(current: &TelemetrySample, previous: &[TelemetrySample]) -> bool {
    let ttft_hit = match current.ttft_ms {
        Some(ttft) => median_u64(
            &previous
                .iter()
                .filter_map(|sample| sample.ttft_ms)
                .collect::<Vec<_>>(),
        )
        .is_some_and(|median| ttft_spike(ttft, median)),
        None => false,
    };
    let rate_hit = match current.tokens_per_sec {
        Some(rate) => median_u64(
            &previous
                .iter()
                .filter_map(|sample| sample.tokens_per_sec)
                .collect::<Vec<_>>(),
        )
        .is_some_and(|median| rate_spike(rate, median)),
        None => false,
    };
    ttft_hit || rate_hit
}

fn model_history<'a>(
    samples: &'a [TelemetrySample],
    model: &str,
    before_or_at: u64,
) -> Vec<&'a TelemetrySample> {
    samples
        .iter()
        .filter(|sample| sample.model == model && sample.at_ms <= before_or_at)
        .collect()
}

fn recent_ring<'a>(history: &[&'a TelemetrySample]) -> Vec<&'a TelemetrySample> {
    let start = history.len().saturating_sub(RING_LEN);
    history[start..].to_vec()
}

pub fn run_telemetry(samples: &[TelemetrySample], run_id: &str) -> Vec<RunTelemetryLane> {
    let mut lanes = Vec::new();
    let mut seen = Vec::new();
    for sample in samples.iter().filter(|sample| sample.run_id == run_id) {
        let key = (sample.model.clone(), sample.lane);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key.clone());
        lanes.push(lane_view(samples, run_id, &key.0, key.1));
    }
    lanes
}

fn lane_view(
    samples: &[TelemetrySample],
    run_id: &str,
    model: &str,
    lane: TelemetryLane,
) -> RunTelemetryLane {
    let lane_samples: Vec<&TelemetrySample> = samples
        .iter()
        .filter(|sample| sample.run_id == run_id && sample.model == model && sample.lane == lane)
        .collect();
    let tokens = TokenCounts::sum_rows(lane_samples.iter().map(|sample| sample.tokens));
    let latest = lane_samples.last().copied();
    let ring = recent_ring(&lane_samples);
    let recent = ring
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let previous: Vec<TelemetrySample> =
                ring[..index].iter().map(|item| (*item).clone()).collect();
            TelemetryPoint {
                at_ms: sample.at_ms,
                ttft_ms: sample.ttft_ms,
                tokens_per_sec: sample.tokens_per_sec,
                spike: sample_is_spike(sample, &previous),
            }
        })
        .collect::<Vec<_>>();
    let spike = recent.last().is_some_and(|point| point.spike);
    RunTelemetryLane {
        model: model.to_string(),
        lane,
        tokens,
        ttft_ms: latest.and_then(|sample| sample.ttft_ms),
        tokens_per_sec: latest.and_then(|sample| sample.tokens_per_sec),
        recent,
        spike,
    }
}

pub fn build_usage_page(
    query: &UsageQuery,
    now_ms: u64,
    offset_secs: i64,
    runs: &[UsageRun],
    samples: &[TelemetrySample],
) -> UsagePage {
    let (from_ms, to_ms) = window(query, now_ms, offset_secs);
    let kind = bucket_kind(query.range, from_ms, to_ms);
    let in_window = |at: u64| at >= from_ms && at <= to_ms;
    let matches_filter = |project_id: &str, agent_id: &str, model: Option<&str>| {
        query
            .filter
            .project_id
            .as_deref()
            .is_none_or(|id| id == project_id)
            && query
                .filter
                .agent_id
                .as_deref()
                .is_none_or(|id| id == agent_id)
            && match (query.filter.model.as_deref(), model) {
                (None, _) => true,
                (Some(want), Some(have)) => want == have,
                (Some(_), None) => false,
            }
    };

    let filtered_samples: Vec<&TelemetrySample> = samples
        .iter()
        .filter(|sample| {
            in_window(sample.at_ms)
                && matches_filter(&sample.project_id, &sample.agent_id, Some(&sample.model))
        })
        .collect();

    let mut rows: Vec<UsageRunRow> = runs
        .iter()
        .filter_map(|run| {
            let highlighted = query.highlighted_run_id.as_deref() == Some(run.id.as_str());
            let project_agent_ok = query
                .filter
                .project_id
                .as_deref()
                .is_none_or(|id| id == run.project_id)
                && query
                    .filter
                    .agent_id
                    .as_deref()
                    .is_none_or(|id| id == run.agent_id);
            if !project_agent_ok && !highlighted {
                return None;
            }
            let run_samples: Vec<&TelemetrySample> = filtered_samples
                .iter()
                .copied()
                .filter(|sample| sample.run_id == run.id)
                .collect();
            let started_in = in_window(run.started_at_ms) && query.filter.model.is_none();
            if run_samples.is_empty() && !started_in && !highlighted {
                return None;
            }
            let mut models: Vec<String> = run_samples
                .iter()
                .map(|sample| sample.model.clone())
                .collect();
            models.sort();
            models.dedup();
            Some(UsageRunRow {
                run_id: run.id.clone(),
                project_id: run.project_id.clone(),
                project_name: run.project_name.clone(),
                agent_id: run.agent_id.clone(),
                agent_name: run.agent_name.clone(),
                issue_id: run.issue_id.clone(),
                started_at_ms: run.started_at_ms,
                models,
                tokens: TokenCounts::sum_rows(run_samples.iter().map(|sample| sample.tokens)),
                highlighted,
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        b.started_at_ms
            .cmp(&a.started_at_ms)
            .then(b.run_id.cmp(&a.run_id))
    });

    let mut bucket_starts = Vec::new();
    let mut cursor = bucket_start_ms(from_ms, kind, offset_secs);
    let last = bucket_start_ms(to_ms, kind, offset_secs);
    while cursor <= last {
        bucket_starts.push(cursor);
        cursor = cursor.saturating_add(bucket_step(kind));
        if cursor == 0 {
            break;
        }
    }
    let buckets = bucket_starts
        .into_iter()
        .map(|start_ms| {
            let end_ms = start_ms.saturating_add(bucket_step(kind));
            let bucket_samples: Vec<&TelemetrySample> = filtered_samples
                .iter()
                .copied()
                .filter(|sample| sample.at_ms >= start_ms && sample.at_ms < end_ms)
                .collect();
            let mut run_ids: Vec<&str> = bucket_samples
                .iter()
                .map(|sample| sample.run_id.as_str())
                .collect();
            run_ids.sort_unstable();
            run_ids.dedup();
            let row_tokens = run_ids.iter().map(|run_id| {
                TokenCounts::sum_rows(
                    bucket_samples
                        .iter()
                        .filter(|sample| sample.run_id == *run_id)
                        .map(|sample| sample.tokens),
                )
            });
            let slow = bucket_samples.iter().any(|sample| {
                let history = model_history(samples, &sample.model, sample.at_ms);
                let ring = recent_ring(&history);
                if ring.is_empty() {
                    return false;
                }
                let previous: Vec<TelemetrySample> = ring[..ring.len().saturating_sub(1)]
                    .iter()
                    .map(|item| (*item).clone())
                    .collect();
                sample_is_spike(sample, &previous)
            });
            UsageBucket {
                start_ms,
                tokens: TokenCounts::sum_rows(row_tokens),
                ttft_ms: median_u64(
                    &bucket_samples
                        .iter()
                        .filter_map(|sample| sample.ttft_ms)
                        .collect::<Vec<_>>(),
                ),
                tokens_per_sec: median_u64(
                    &bucket_samples
                        .iter()
                        .filter_map(|sample| sample.tokens_per_sec)
                        .collect::<Vec<_>>(),
                ),
                slow,
            }
        })
        .collect();

    let totals = TokenCounts::sum_rows(rows.iter().map(|row| row.tokens));
    let mut projects: Vec<UsageOption> = runs
        .iter()
        .map(|run| UsageOption {
            id: run.project_id.clone(),
            name: run.project_name.clone(),
        })
        .collect();
    projects.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    projects.dedup();
    let mut agents: Vec<UsageOption> = runs
        .iter()
        .map(|run| UsageOption {
            id: run.agent_id.clone(),
            name: run.agent_name.clone(),
        })
        .collect();
    agents.sort_by(|a, b| a.name.cmp(&b.name).then(a.id.cmp(&b.id)));
    agents.dedup();
    let mut models: Vec<String> = samples.iter().map(|sample| sample.model.clone()).collect();
    models.sort();
    models.dedup();

    UsagePage {
        range: query.range,
        custom_from_ms: query.custom_from_ms,
        custom_to_ms: query.custom_to_ms,
        filter: query.filter.clone(),
        bucket_kind: kind,
        from_ms,
        to_ms,
        cache_hit_rate: totals.cache_hit_rate(),
        totals,
        runs: rows,
        buckets,
        highlighted_run_id: query.highlighted_run_id.clone(),
        projects,
        agents,
        models,
    }
}
