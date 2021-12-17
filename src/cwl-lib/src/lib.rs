/*
 * Copyright Kitten Cat LLC. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

#[macro_use]
extern crate derivative;

use std::sync::Arc;

use aws_sdk_cloudwatchlogs::Client;
use aws_types::region::Region;
use bytes::Bytes;
use chrono::DateTime;
use chrono::Duration;
use chrono::TimeZone;
use chrono::Utc;
use futures::future::try_join_all;
use leaky_bucket::RateLimiter;
use lru::LruCache;
use regexes::LogGroupNameMatcher;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::{debug, instrument, trace};

#[derive(Error, Debug)]
pub enum CloudWatchLogsError {
    #[error("CloudWatch Logs SDK describe logs error")]
    DescribeLogGroupsError(
        #[from] aws_smithy_http::result::SdkError<aws_sdk_cloudwatchlogs::error::DescribeLogGroupsError>,
    ),

    #[error("CloudWatch Logs SDK filter log events error")]
    FilterLogEventsError(
        #[from] aws_smithy_http::result::SdkError<aws_sdk_cloudwatchlogs::error::FilterLogEventsError>,
    ),

    #[error("failed to convert CloudWatch filtered log event: {0}")]
    FailedToConvertCloudWatchFilteredLogEvent(String),

    #[error("Invalid GetLogsToDisplay message: {0}")]
    InvalidGetLogsToDisplayMessage(String),

    #[error("No CloudWatch Logs log groups match filter: {0}")]
    NoCloudWatchLogGroupsMatchFilter(String),

    #[error("unknown cloudwatch logs error")]
    Unknown,
}

#[derive(Clone, Debug)]
pub struct FilteredLogEvent {
    pub log_group_name: String,
    pub event_id: String,
    pub ingestion_time: DateTime<Utc>,
    pub log_stream_name: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

impl FilteredLogEvent {
    pub fn new(
        log_group_name: impl Into<std::string::String>,
        value: aws_sdk_cloudwatchlogs::model::FilteredLogEvent,
    ) -> Result<Self, CloudWatchLogsError> {
        let event_id = match value.event_id {
            Some(event_id) => Ok(event_id),
            None => Err(CloudWatchLogsError::FailedToConvertCloudWatchFilteredLogEvent(
                "event_id missing".to_string(),
            )),
        }?;
        let ingestion_time = match value.ingestion_time {
            Some(ingestion_time) => Ok(chrono::Utc.timestamp_millis(ingestion_time)),
            None => Err(CloudWatchLogsError::FailedToConvertCloudWatchFilteredLogEvent(
                "ingestion_time missing".to_string(),
            )),
        }?;
        let log_stream_name = match value.log_stream_name {
            Some(log_stream_name) => Ok(log_stream_name),
            None => Err(CloudWatchLogsError::FailedToConvertCloudWatchFilteredLogEvent(
                "log_stream_name missing".to_string(),
            )),
        }?;
        let message = match value.message {
            Some(message) => Ok(message),
            None => Err(CloudWatchLogsError::FailedToConvertCloudWatchFilteredLogEvent(
                "message missing".to_string(),
            )),
        }?;
        let timestamp = match value.timestamp {
            Some(timestamp) => Ok(chrono::Utc.timestamp_millis(timestamp)),
            None => Err(CloudWatchLogsError::FailedToConvertCloudWatchFilteredLogEvent(
                "timestamp missing".to_string(),
            )),
        }?;
        Ok(Self {
            log_group_name: log_group_name.into(),
            event_id,
            ingestion_time,
            log_stream_name,
            message,
            timestamp,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct TimeBounds {
    pub first_event_time: DateTime<Utc>,
    pub last_event_time: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct CacheKey {
    pub log_group_name_matcher: LogGroupNameMatcher,
    pub time_bounds: TimeBounds,
}

#[derive(Clone, Debug)]
struct CacheValue {
    pub data_to_display: Bytes,
}

#[derive(Derivative)]
#[derivative(Clone, Debug)]
pub struct CloudWatchLogsImpl {
    client: aws_sdk_cloudwatchlogs::Client,

    #[derivative(Debug = "ignore")]
    rate_limiter: Arc<RateLimiter>,
}

impl CloudWatchLogsImpl {
    #[instrument(level = "debug")]
    pub async fn new<T: std::fmt::Debug + Into<String>>(tps: usize, region: Option<T>) -> Self {
        let mut config = aws_config::from_env();
        if let Some(region) = region {
            config = config.region(Region::new(region.into()));
        }
        let config = config.load().await;
        let client = Client::new(&config);
        Self {
            client,
            rate_limiter: Arc::new(
                RateLimiter::builder()
                    .max(tps)
                    .initial(tps)
                    .refill(tps)
                    .interval(std::time::Duration::from_secs(1))
                    .build(),
            ),
        }
    }

    #[instrument(level = "debug")]
    pub async fn get_log_group_names(&self) -> Result<Vec<String>, CloudWatchLogsError> {
        const LOG_GROUP_LIMIT: i32 = 50;
        let mut result = Vec::new();
        let mut next_token: Option<String> = None;
        loop {
            self.rate_limiter.acquire_one().await;
            let req = self
                .client
                .describe_log_groups()
                .limit(LOG_GROUP_LIMIT)
                .set_next_token(next_token.clone());
            let resp = match req.send().await {
                Ok(inner) => Ok(inner),
                Err(err) => Err(CloudWatchLogsError::DescribeLogGroupsError(err)),
            }?;
            let log_groups = resp.log_groups();
            if log_groups.is_none() {
                break;
            }
            let log_groups = log_groups.unwrap();
            if log_groups.is_empty() {
                break;
            }
            log_groups
                .into_iter()
                .map(|log_group| log_group.log_group_name().unwrap().to_string())
                .for_each(|log_group| result.push(log_group));
            if resp.next_token.is_none() {
                break;
            }
            next_token = resp.next_token;
        }
        Ok(result)
    }

    #[instrument(level = "debug")]
    pub async fn get_log_events(
        &self,
        log_group_name: String,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: Option<i32>,
    ) -> Result<Vec<FilteredLogEvent>, CloudWatchLogsError> {
        const LOGS_BATCH_SIZE: i32 = 10_000;
        let mut events = Vec::with_capacity(LOGS_BATCH_SIZE as usize);
        let mut next_token: Option<String> = None;
        let limit = limit.unwrap_or(usize::MAX as i32) as usize;
        loop {
            debug!("tick, start_time: {:?}, end_time: {:?}", start_time, end_time);
            self.rate_limiter.acquire_one().await;
            let mut req = self
                .client
                .filter_log_events()
                .log_group_name(&log_group_name)
                .limit(LOGS_BATCH_SIZE as i32)
                .set_next_token(next_token);
            if let Some(start_time) = start_time {
                req = req.start_time(start_time.timestamp_millis());
            }
            if let Some(end_time) = end_time {
                req = req.end_time(end_time.timestamp_millis());
            }
            let resp = match req.send().await {
                Ok(inner) => Ok(inner),
                Err(err) => Err(CloudWatchLogsError::FilterLogEventsError(err)),
            }?;
            for event in resp.events.unwrap_or(vec![]) {
                let event = FilteredLogEvent::new(&log_group_name, event)?;
                if events.len() >= limit {
                    return Ok(events);
                }
                events.push(event);
            }
            if resp.next_token.is_none() {
                break;
            }
            next_token = resp.next_token;
        }
        Ok(events)
    }

    #[instrument(level = "debug")]
    pub async fn get_first_event_time_for_log_group(
        &self,
        log_group_name: String,
    ) -> Result<Option<DateTime<Utc>>, CloudWatchLogsError> {
        let search_window: chrono::Duration = Duration::days(365 * 5);
        let last_event_time = Utc::now();
        let mut first_event_time = last_event_time - search_window;
        let log_group_name = log_group_name.into();
        let log_events = self
            .get_log_events(
                log_group_name,
                Some(first_event_time),
                Some(last_event_time),
                Some(1),
            )
            .await?;
        if let Some(log_event) = log_events.first() {
            first_event_time = log_event.timestamp;
        } else {
            return Ok(None);
        }

        Ok(Some(first_event_time))
    }
}

fn is_cacheable(cache_key: &CacheKey) -> bool {
    Utc::now() - cache_key.time_bounds.last_event_time > Duration::minutes(5)
}

#[instrument(level = "debug")]
async fn get_logs_to_display(
    log_group_name_matcher: LogGroupNameMatcher,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    cwl: Arc<CloudWatchLogsImpl>,
    cache: Arc<tokio::sync::Mutex<LruCache<CacheKey, CacheValue>>>,
) -> Result<Bytes, CloudWatchLogsError> {
    let cache_key = CacheKey {
        log_group_name_matcher: log_group_name_matcher.clone(),
        time_bounds: TimeBounds {
            first_event_time: start_time,
            last_event_time: end_time,
        },
    };
    debug!("get_logs_to_display. cache_key: {:?}", cache_key);
    let cache = Arc::clone(&cache);
    {
        let mut cache = cache.lock().await;
        if let Some(value) = cache.get(&cache_key) {
            return Ok(value.data_to_display.clone());
        }
    }
    let log_group_names: Vec<String> = cwl
        .get_log_group_names()
        .await?
        .into_iter()
        .filter(|log_group_name| log_group_name_matcher.is_match(log_group_name))
        .collect();
    let mut tasks = vec![];
    for log_group_name in log_group_names.into_iter() {
        let cwl = Arc::clone(&cwl);
        let handle: JoinHandle<Vec<FilteredLogEvent>> = tokio::spawn(async move {
            debug!(
                "get_logs_to_display spawning to get logs for log_group_name {}",
                log_group_name
            );
            let logs = cwl
                .get_log_events(log_group_name, Some(start_time), Some(end_time), None)
                .await
                .unwrap();
            return logs;
        });
        tasks.push(handle);
    }
    let mut logs: Vec<FilteredLogEvent> = try_join_all(tasks)
        .await
        .unwrap()
        .into_iter()
        .flat_map(|e| e)
        .collect();
    logs.sort_by_key(|l| l.timestamp);

    trace!("logs: {:?}", logs);
    let data: Bytes = logs
        .into_iter()
        .map(|log| format!("[{}] {}", log.log_stream_name, log.message))
        .collect::<Vec<String>>()
        .join("\n")
        .into();
    if is_cacheable(&cache_key) {
        let mut cache = cache.lock().await;
        cache.put(
            cache_key,
            CacheValue {
                data_to_display: data.clone(),
            },
        );
    }
    Ok(data)
}

// See: https://ryhl.io/blog/actors-with-tokio/
#[derive(Debug)]
enum CloudWatchLogsMessage {
    GetLogGroupNames {
        respond_to: oneshot::Sender<Result<Vec<String>, CloudWatchLogsError>>,
    },
    GetLogEvents {
        log_group_name: String,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: Option<i32>,
        respond_to: oneshot::Sender<Result<Vec<FilteredLogEvent>, CloudWatchLogsError>>,
    },
    GetFirstEventTimeForLogGroup {
        log_group_name: String,
        respond_to: oneshot::Sender<Result<Option<DateTime<Utc>>, CloudWatchLogsError>>,
    },
    GetLogsToDisplay {
        log_group_name: Option<String>,
        log_group_filter: Option<String>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        respond_to: oneshot::Sender<Result<Bytes, CloudWatchLogsError>>,
    },
}

#[derive(Debug)]
struct CloudWatchLogsActor {
    cwl: Arc<CloudWatchLogsImpl>,
    logs_display_cache: Arc<tokio::sync::Mutex<LruCache<CacheKey, CacheValue>>>,
}

impl CloudWatchLogsActor {
    fn new(cwl: CloudWatchLogsImpl) -> Self {
        let cache_capacity = Duration::hours(1).num_minutes() as usize;
        CloudWatchLogsActor {
            cwl: Arc::new(cwl),
            logs_display_cache: Arc::new(tokio::sync::Mutex::new(LruCache::new(cache_capacity))),
        }
    }

    #[instrument(level = "debug")]
    async fn handle_message(&self, msg: CloudWatchLogsMessage) {
        match msg {
            CloudWatchLogsMessage::GetLogGroupNames { respond_to } => {
                let result = self.cwl.get_log_group_names().await;
                let _ = respond_to.send(result);
            }
            CloudWatchLogsMessage::GetLogEvents {
                log_group_name,
                start_time,
                end_time,
                limit,
                respond_to,
            } => {
                let result = self
                    .cwl
                    .get_log_events(log_group_name, start_time, end_time, limit)
                    .await;
                let _ = respond_to.send(result);
            }
            CloudWatchLogsMessage::GetFirstEventTimeForLogGroup {
                log_group_name,
                respond_to,
            } => {
                let result = self.cwl.get_first_event_time_for_log_group(log_group_name).await;
                let _ = respond_to.send(result);
            }
            CloudWatchLogsMessage::GetLogsToDisplay {
                log_group_name,
                log_group_filter,
                start_time,
                end_time,
                respond_to,
            } => {
                let pattern: String;
                if let Some(log_group_name) = log_group_name {
                    pattern = format!("^{}$", log_group_name.as_str());
                } else if let Some(log_group_filter) = log_group_filter {
                    pattern = log_group_filter;
                } else {
                    let _ = respond_to.send(Err(CloudWatchLogsError::InvalidGetLogsToDisplayMessage(
                        "Must specify either log_group_name or log_group_filter".to_string(),
                    )));
                    return;
                }
                let matcher = LogGroupNameMatcher::new(&pattern);
                let cwl = Arc::clone(&self.cwl);
                let cache = Arc::clone(&self.logs_display_cache);
                let result = get_logs_to_display(matcher, start_time, end_time, cwl, cache).await;
                let _ = respond_to.send(result);
            }
        }
    }
}

#[instrument(level = "debug")]
async fn run_cloud_watch_logs_actor(
    actor: Arc<CloudWatchLogsActor>,
    mut receiver: mpsc::Receiver<CloudWatchLogsMessage>,
) {
    while let Some(msg) = receiver.recv().await {
        debug!("actor sending msg {:?}...", msg);
        let actor = Arc::clone(&actor);
        tokio::spawn(async move { actor.handle_message(msg).await });
        debug!("actor finished sending msg");
    }
}

#[derive(Clone, Debug)]
pub struct CloudWatchLogsActorHandle {
    sender: mpsc::Sender<CloudWatchLogsMessage>,
}

impl CloudWatchLogsActorHandle {
    pub fn new(cwl: CloudWatchLogsImpl) -> Self {
        let (sender, receiver) = mpsc::channel(4);
        let actor = Arc::new(CloudWatchLogsActor::new(cwl));
        tokio::spawn(run_cloud_watch_logs_actor(actor, receiver));

        Self { sender }
    }

    #[instrument(level = "debug")]
    pub async fn get_log_group_names(&self) -> Result<Vec<String>, CloudWatchLogsError> {
        let (send, recv) = oneshot::channel();
        let msg = CloudWatchLogsMessage::GetLogGroupNames { respond_to: send };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    #[instrument(level = "debug")]
    pub async fn get_log_events(
        &self,
        log_group_name: String,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: Option<i32>,
    ) -> Result<Vec<FilteredLogEvent>, CloudWatchLogsError> {
        let (send, recv) = oneshot::channel();
        let msg = CloudWatchLogsMessage::GetLogEvents {
            respond_to: send,
            log_group_name,
            start_time,
            end_time,
            limit,
        };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    #[instrument(level = "debug")]
    pub async fn get_first_event_time_for_log_group(
        &self,
        log_group_name: String,
    ) -> Result<Option<DateTime<Utc>>, CloudWatchLogsError> {
        let (send, recv) = oneshot::channel();
        let msg = CloudWatchLogsMessage::GetFirstEventTimeForLogGroup {
            respond_to: send,
            log_group_name,
        };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }

    #[instrument(level = "debug")]
    pub async fn get_logs_to_display(
        &self,
        log_group_name: Option<String>,
        log_group_filter: Option<String>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<Bytes, CloudWatchLogsError> {
        let (send, recv) = oneshot::channel();
        let msg = CloudWatchLogsMessage::GetLogsToDisplay {
            respond_to: send,
            log_group_name,
            log_group_filter,
            start_time,
            end_time,
        };
        let _ = self.sender.send(msg).await;
        recv.await.expect("Actor task has been killed")
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use chrono::Utc;

    use crate::CloudWatchLogsImpl;

    #[test]
    fn test_list_log_groups() {
        let tps = 5;
        let region = Some("us-west-2");
        let cwl: CloudWatchLogsImpl = tokio_test::block_on(CloudWatchLogsImpl::new(tps, region));
        let res = tokio_test::block_on(cwl.get_log_group_names()).unwrap();
        res.iter().for_each(|l| println!("{}", l));
    }

    #[test]
    fn test_get_log_events() {
        let tps = 5;
        let region = Some("us-west-2");
        let cwl: CloudWatchLogsImpl = tokio_test::block_on(CloudWatchLogsImpl::new(tps, region));
        let log_group_name = "babynames-preprod-log-group-syslog".to_string();
        let start_time = Some(Utc.ymd(2021, 11, 26).and_hms(1, 0, 0));
        let end_time = Some(Utc.ymd(2021, 11, 26).and_hms(21, 0, 0));
        let res =
            tokio_test::block_on(cwl.get_log_events(log_group_name, start_time, end_time, None)).unwrap();
        res.iter().for_each(|l| println!("{:?}", l.message));
    }

    #[test]
    fn get_time_bounds_for_log_group() {
        let tps = 5;
        let region = Some("us-west-2");
        let cwl: CloudWatchLogsImpl = tokio_test::block_on(CloudWatchLogsImpl::new(tps, region));
        let log_group_name = "babynames-preprod-log-group-syslog".to_string();
        let res = tokio_test::block_on(cwl.get_first_event_time_for_log_group(log_group_name)).unwrap();
        println!("{:?}", res);
    }
}
