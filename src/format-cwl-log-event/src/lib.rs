use chrono::DateTime;
use chrono::SecondsFormat;
use chrono::Utc;
use pest::Parser;

include!(concat!(env!("OUT_DIR"), "/format_cwl_log_event_parser.rs"));

#[derive(thiserror::Error, Debug)]
pub enum FormatCwlLogEventError {
    #[error(transparent)]
    CompileError(#[from] pest::error::Error<Rule>),

    #[error("unknown format variable '{0}', choose one from 'log_group_name', 'event_id', 'ingestion_time', 'log_stream_name', 'message', 'timestamp'")]
    UnknownFormatVariable(String),

    #[error("unknown format error")]
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

    ingestion_time_rfc3339: String,
    timestamp_rfc3339: String,
}

impl FilteredLogEvent {
    pub fn new(
        log_group_name: impl Into<String>,
        event_id: impl Into<String>,
        ingestion_time: DateTime<Utc>,
        log_stream_name: impl Into<String>,
        message: impl Into<String>,
        timestamp: DateTime<Utc>,
    ) -> Self {
        Self {
            log_group_name: log_group_name.into(),
            event_id: event_id.into(),
            ingestion_time,
            ingestion_time_rfc3339: ingestion_time.to_rfc3339_opts(SecondsFormat::Millis, true),
            log_stream_name: log_stream_name.into(),
            message: message.into(),
            timestamp,
            timestamp_rfc3339: timestamp.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
enum FilteredLogEventVariable {
    LogGroupName,
    EventId,
    IngestionTime,
    LogStreamName,
    Message,
    Timestamp,
}

impl TryFrom<&str> for FilteredLogEventVariable {
    type Error = FormatCwlLogEventError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "log_group_name" => Ok(FilteredLogEventVariable::LogGroupName),
            "event_id" => Ok(FilteredLogEventVariable::EventId),
            "ingestion_time" => Ok(FilteredLogEventVariable::IngestionTime),
            "log_stream_name" => Ok(FilteredLogEventVariable::LogStreamName),
            "message" => Ok(FilteredLogEventVariable::Message),
            "timestamp" => Ok(FilteredLogEventVariable::Timestamp),
            _ => Err(FormatCwlLogEventError::UnknownFormatVariable(String::from(value))),
        }
    }
}

#[derive(PartialEq, Hash, Clone, Debug, Eq)]
enum FormatValue<T> {
    EscapedDelimeter,
    Variable(T),
    Literal(String),
}

#[derive(Clone, Debug, PartialEq, Hash, Eq)]
pub struct LogFormatter {
    instructions: Vec<FormatValue<FilteredLogEventVariable>>,
}

impl LogFormatter {
    pub fn new(format: impl AsRef<str>) -> Result<LogFormatter, FormatCwlLogEventError> {
        let parser = FormatCwlLogEventParser::parse(Rule::format, format.as_ref())?;
        let mut instructions = vec![];
        for pair in parser.into_iter() {
            match pair.as_rule() {
                Rule::escaped_delimiter => {
                    let value = FormatValue::EscapedDelimeter;
                    instructions.push(value);
                }
                Rule::variable => {
                    let identifier = pair.into_inner().next().unwrap().as_str();
                    let variable = identifier.try_into()?;
                    let value = FormatValue::Variable(variable);
                    instructions.push(value);
                }
                Rule::literal => {
                    let value = FormatValue::Literal(String::from(pair.as_str()));
                    instructions.push(value)
                }
                Rule::EOI => {}
                _ => unreachable!(),
            }
        }

        Ok(Self { instructions })
    }

    pub fn format(&self, event: FilteredLogEvent) -> String {
        let mut output = String::with_capacity(128);
        for instruction in self.instructions.iter() {
            output.push_str(match instruction {
                FormatValue::EscapedDelimeter => "$",
                FormatValue::Variable(identifier) => match identifier {
                    FilteredLogEventVariable::LogGroupName => &event.log_group_name,
                    FilteredLogEventVariable::EventId => &event.event_id,
                    FilteredLogEventVariable::IngestionTime => &event.ingestion_time_rfc3339,
                    FilteredLogEventVariable::LogStreamName => &event.log_stream_name,
                    FilteredLogEventVariable::Message => &event.message,
                    FilteredLogEventVariable::Timestamp => &event.timestamp_rfc3339,
                },
                FormatValue::Literal(value) => value,
            });
        }
        output
    }
}

pub fn clap_validate_output_format<T: Into<String>>(output_format: T) -> Result<(), String> {
    let output_format = output_format.into();
    match LogFormatter::new(output_format.clone()) {
        Ok(_) => Ok(()),
        Err(err) => Err(format!("\n{}", err)),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use chrono::Utc;

    use crate::FilteredLogEvent;
    use crate::LogFormatter;

    fn get_test_event_1() -> FilteredLogEvent {
        FilteredLogEvent::new(
            "/aws/logs/log-group",
            "event-id",
            Utc.ymd(2014, 7, 8).and_hms_nano(9, 10, 11, 123456789),
            "log-stream-name",
            "message",
            Utc.ymd(2014, 7, 8).and_hms_nano(9, 10, 10, 789101234),
        )
    }

    #[test]
    fn default_format_passes() {
        let formatter = LogFormatter::new("[${log_stream_name}] ${message}").expect("default format should pass");
        let actual_output = formatter.format(get_test_event_1());
        assert_eq!("[log-stream-name] message", actual_output);
    }

    #[test]
    fn default_format_without_braces_passes() {
        let formatter = LogFormatter::new("[$log_stream_name] $message").expect("default format without braces should pass");
        let actual_output = formatter.format(get_test_event_1());
        assert_eq!("[log-stream-name] message", actual_output);
    }

    #[test]
    fn timestamp_format_passes() {
        let formatter = LogFormatter::new("$timestamp - $message").expect("timestamp format should pass");
        let actual_output = formatter.format(get_test_event_1());
        assert_eq!("2014-07-08T09:10:10.789Z - message", actual_output);
    }

    #[test]
    fn just_escaped_delimiter_passes() {
        let formatter = LogFormatter::new("$$").expect("escaped delimiter should pass");
        let actual_output = formatter.format(get_test_event_1());
        assert_eq!("$", actual_output);
    }

    #[test]
    fn just_delimiter_fails() {
        let formatter = LogFormatter::new("$");
        assert!(formatter.is_err());
    }
}
