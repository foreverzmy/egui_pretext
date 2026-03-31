use pretext::analysis::{normalize_whitespace_normal, normalize_whitespace_pre_wrap};

#[test]
fn normal_mode_collapses_ascii_whitespace() {
    assert_eq!(
        normalize_whitespace_normal("  alpha\tbeta\n gamma  "),
        "alpha beta gamma"
    );
}

#[test]
fn pre_wrap_normalizes_crlf_and_cr() {
    assert_eq!(
        normalize_whitespace_pre_wrap("alpha\r\nbeta\rgamma"),
        "alpha\nbeta\ngamma"
    );
}
