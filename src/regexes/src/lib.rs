macro_rules! regex {
    ($re:literal $(,)?) => {{
        static RE: once_cell::sync::OnceCell<regex::Regex> = once_cell::sync::OnceCell::new();
        RE.get_or_init(|| regex::Regex::new($re).unwrap())
    }};
}

/// Check if the string is a valid AWS CloudWatch Logs log group name [1].
///
/// [1] https://docs.aws.amazon.com/AmazonCloudWatchLogs/latest/APIReference/API_CreateLogGroup.html
pub fn valid_cwl_log_group_name<T: AsRef<str>>(log_group_name: T) -> bool {
    const MIN_LENGTH: usize = 1;
    const MAX_LENGTH: usize = 512;
    let log_group_name: &str = log_group_name.as_ref();
    if log_group_name.len() < MIN_LENGTH || log_group_name.len() > MAX_LENGTH {
        return false;
    }
    let pattern = regex!(
        r#"(?x)
    ^               # start of string
    [               # any of these characters
        [:alpha:]   # a-z and A-Z
        \d          # digit
        _
        /
        .
        \#          # pound sign
        -
    ]+
    $               # end of string
    "#
    );
    return pattern.is_match(log_group_name);
}

pub fn clap_validate_cwl_log_group_name<T: Into<String>>(log_group_name: T) -> Result<(), String> {
    let log_group_name = log_group_name.into();
    match valid_cwl_log_group_name(&log_group_name) {
        true => Ok(()),
        false => Err(format!(
            "{} is not a valid CloudWatch Logs log group name",
            log_group_name
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_cwl_log_group_name_matches() {
        assert!(valid_cwl_log_group_name("Log-Group-03-/.#"));
    }

    #[test]
    fn test_clap_validate_cwl_log_group_name_matches() {
        assert!(clap_validate_cwl_log_group_name("Log-Group-03-/.#").is_ok());
    }

    #[test]
    fn test_valid_cwl_log_group_name_does_not_match_chars() {
        assert!(!valid_cwl_log_group_name("log-group+"));
    }

    #[test]
    fn test_clap_validate_cwl_log_group_name_does_not_match_chars() {
        assert!(clap_validate_cwl_log_group_name("log-group+").is_err());
    }

    #[test]
    fn test_valid_cwl_log_group_name_does_not_match_too_long() {
        let log_group_name: String = (0..1000).map(|_| "a").collect();
        assert!(!valid_cwl_log_group_name(log_group_name));
    }
}
