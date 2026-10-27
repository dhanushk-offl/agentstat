//! Integration tests for agentstat decimal.

use agentstat::Decimal;
#[test]
fn exact_amounts_arithmetic_and_serialization() {
    let a: Decimal = "9007199254740993.1234567890123456789".parse().unwrap();
    let b: Decimal = "0.0000000000000000001".parse().unwrap();
    assert_eq!(
        a.checked_add(&b).unwrap().to_string(),
        "9007199254740993.123456789012345679"
    );
    assert_eq!(
        "0.3"
            .parse::<Decimal>()
            .unwrap()
            .checked_sub(&"0.1".parse().unwrap())
            .unwrap()
            .to_string(),
        "0.2"
    );
    assert_eq!(
        "12345"
            .parse::<Decimal>()
            .unwrap()
            .divided_by_power_of_ten(2)
            .unwrap()
            .to_string(),
        "123.45"
    );
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        "\"9007199254740993.1234567890123456789\""
    );
    assert_eq!(
        serde_json::from_str::<Decimal>("12345678901234567890.123456789")
            .unwrap()
            .as_str(),
        "12345678901234567890.123456789"
    );
}
#[test]
fn decimal_normalization_and_rejection() {
    for (input, expected) in [
        ("-0.000", "0"),
        ("001.2300", "1.23"),
        ("123e-4", "0.0123"),
        ("0.01e2", "1"),
        ("-1e2", "-100"),
        ("10.00", "10"),
    ] {
        assert_eq!(input.parse::<Decimal>().unwrap().as_str(), expected)
    }
    for input in [
        "NaN",
        "inf",
        "1e99999999",
        "1.2.3",
        "1.",
        ".5",
        "+1",
        " 1",
        "--1",
        "1e",
        "",
        "1_000",
    ] {
        assert!(input.parse::<Decimal>().is_err(), "{input}")
    }
}
