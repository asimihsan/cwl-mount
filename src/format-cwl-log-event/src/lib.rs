use chrono::DateTime;
use chrono::Utc;
use pest::error::Error;
use pest::iterators::Pair;
use pest::Parser;

#[derive(Clone, Debug)]
pub struct FilteredLogEvent {
    pub log_group_name: String,
    pub event_id: String,
    pub ingestion_time: DateTime<Utc>,
    pub log_stream_name: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

include!(concat!(env!("OUT_DIR"), "/format_cwl_log_event_parser.rs"));

type Result<T> = std::result::Result<T, Error<Rule>>;

enum FormatValue {
    EscapedDelimeter,
    Variable(String),
    Literal(String),
}

pub struct LogFormatter {
    instructions: Vec<FormatValue>,
}

impl LogFormatter {
    pub fn new(format: impl AsRef<str>) -> Result<Self> {
        let mut parser = FormatCwlLogEventParser::parse(Rule::format, format.as_ref())?;
        let mut instructions = vec![];

        fn parse_value(pair: Pair<Rule>) {
            match pair.as_rule() {
                Rule::EOI => todo!(),
                Rule::format => todo!(),
                Rule::escaped_delimiter => {
                    let value = FormatValue::EscapedDelimeter;
                    instructions.push(value);
                },
                Rule::delimiter => todo!(),
                Rule::identifier => todo!(),
                Rule::variable => {
                    let s1 = pair.as_rule();
                    let s2 = pair.as_str();
                    let s3 = pair.into_inner().next().unwrap();
                    let _x = 1 + 2;
                    let value = FormatValue::EscapedDelimeter;
                    instructions.push(value);
                },
                Rule::literal => {
                    let value = FormatValue::Literal(String::from(pair.as_str()));
                    instructions.push(value)
                },
                Rule::char => todo!(),
                _ => unreachable!(),
            }
        }

        for pair in parser.into_iter() {
            parse_value(pair, &mut instructions);
        }

        Ok(Self { instructions })
    }

    pub fn format(&self, event: &FilteredLogEvent) -> String {
        return String::from("");
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use chrono::Utc;

    use crate::FilteredLogEvent;
    use crate::LogFormatter;

    fn get_test_event_1() -> FilteredLogEvent {
        FilteredLogEvent {
            log_group_name: String::from("/aws/logs/log-group"),
            event_id: String::from("event-id"),
            ingestion_time: Utc.ymd(2014, 7, 8).and_hms(9, 10, 11),
            log_stream_name: String::from("log-stream-name"),
            message: String::from("message"),
            timestamp: Utc.ymd(2014, 7, 8).and_hms(9, 10, 10),
        }
    }

    #[test]
    fn default_format_passes() {
        let formatter = LogFormatter::new("[$log_stream_name] $message").expect("default format should pass");
        let actual_output = formatter.format(&get_test_event_1());
        assert_eq!("[log-stream-name] message", actual_output);
    }

    #[test]
    fn just_escaped_delimiter_passes() {
        let formatter = LogFormatter::new("$$").expect("escaped delimiter should pass");
        let actual_output = formatter.format(&get_test_event_1());
        assert_eq!("$", actual_output);
    }

    #[test]
    fn just_delimiter_fails() {
        let formatter = LogFormatter::new("$");
        assert!(formatter.is_err());
    }
}
