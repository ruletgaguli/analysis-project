use analysis::parse::{
    AppLogJournalKind, AppLogKind, AppLogTraceKind, AssetDsc, AuthData, Backet, LogKind, LogLine,
    ParseError, UserCash, just_parse,
};
use analysis::{ReadLogError, ReadMode, read_log};
use std::io::{self, Cursor, Read};
use std::num::{NonZeroI32, NonZeroU32};

const SOURCE: &str = concat!(
    "\nSystem::Error NetworkError \"нет сети\" requestid=1\n",
    "System::Trace SendRequest \"запрос\" requestid=2\n",
    "App::Journal DepositCash UserCash{\"user_id\":\"Bob\",\"count\":3,} requestid=2\n",
    "App::Error LackOf \"нет товара\" requestid=3\n",
    "App::Journal DeleteUser {\"user_id\":\"Bob\",} requestid=4\n",
    "App::Journal UnregisterAsset {\"user_id\":\"Bob\",\"asset_id\":\"milk\",} requestid=5\n",
);

#[test]
fn filters_preserve_order_and_original_exchange_selection() {
    let all = read_log(SOURCE.as_bytes(), ReadMode::All, &[]).unwrap();
    assert_eq!(all.len(), 6);
    let errors = read_log(SOURCE.as_bytes(), ReadMode::Errors, &[]).unwrap();
    assert_eq!(errors, vec![all[0].clone(), all[3].clone()]);
    let exchanges = read_log(SOURCE.as_bytes(), ReadMode::Exchanges, &[]).unwrap();
    assert_eq!(exchanges, vec![all[2].clone()]);
    let ids = [NonZeroU32::new(2).unwrap()];
    assert_eq!(
        read_log(SOURCE.as_bytes(), ReadMode::All, &ids).unwrap(),
        all[1..3]
    );
    assert_eq!(
        read_log(SOURCE.as_bytes(), ReadMode::Exchanges, &ids).unwrap(),
        exchanges
    );
    assert!(
        read_log(SOURCE.as_bytes(), ReadMode::Errors, &ids)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn supports_owned_and_borrowed_readers_without_static_lifetime() {
    let source = SOURCE.to_string();
    let mut cursor = Cursor::new(source.as_bytes());
    assert_eq!(read_log(&mut cursor, ReadMode::All, &[]).unwrap().len(), 6);
    assert_eq!(cursor.position(), source.len() as u64);
    assert_eq!(
        read_log(Cursor::new(source), ReadMode::All, &[])
            .unwrap()
            .len(),
        6
    );
    assert!(
        read_log(&b" \n\t\n"[..], ReadMode::All, &[])
            .unwrap()
            .is_empty()
    );
}

#[test]
fn errors_are_reported_instead_of_silent_truncation() {
    let input = "System::Error NetworkError \"ok\" requestid=1\n\nне лог\n";
    assert!(matches!(
        read_log(input.as_bytes(), ReadMode::All, &[]),
        Err(ReadLogError::Parse { line: 3, .. })
    ));
    let garbage = "System::Error NetworkError \"ok\" requestid=1 garbage";
    assert!(matches!(
        read_log(garbage.as_bytes(), ReadMode::All, &[]),
        Err(ReadLogError::Parse { line: 1, .. })
    ));
    assert!(matches!(
        read_log(&b"\xff\n"[..], ReadMode::All, &[]),
        Err(ReadLogError::Io { line: 1, .. })
    ));
}

#[derive(Debug)]
struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("ошибка тестового источника"))
    }
}

#[test]
fn io_errors_are_returned() {
    assert!(matches!(
        read_log(FailingReader, ReadMode::All, &[]),
        Err(ReadLogError::Io { line: 1, .. })
    ));
}

#[test]
fn remainder_borrows_original_input() {
    let input = String::from("Backet{\"count\":7,\"asset_id\":\"молоко\",}остаток");
    let (remaining, backet) = just_parse::<Backet>(&input).unwrap();
    let offset = input.find("остаток").unwrap();
    assert_eq!(remaining, &input[offset..]);
    assert_eq!(remaining.as_ptr(), input[offset..].as_ptr());
    assert_eq!(backet.asset_id, "молоко");
    assert_eq!(backet.count.get(), 7);
}

#[test]
fn generic_parser_supports_different_types() {
    let (_, asset) =
        just_parse::<AssetDsc>("AssetDsc{\"id\":\"usd\",\"dsc\":\"доллар\",}").unwrap();
    assert_eq!(asset.dsc, "доллар");
    let (_, cash) = just_parse::<UserCash>("UserCash{\"count\":1,\"user_id\":\"Bob\",}").unwrap();
    assert_eq!(cash.count.get(), 1);
    assert_eq!(
        just_parse::<NonZeroI32>("-2147483648!"),
        Ok(("!", NonZeroI32::new(i32::MIN).unwrap()))
    );
}

#[test]
fn nonzero_numbers_preserve_invariants() {
    for value in 1..=1024 {
        for input in [format!("{value}!"), format!("0x{value:x}!")] {
            let (remaining, number) = just_parse::<NonZeroU32>(&input).unwrap();
            assert_eq!(remaining, "!");
            assert_eq!(number.get(), value);
        }
    }
    for input in ["0", "0x0", "4294967296", "0x100000000", "-1", "", "0x"] {
        assert_eq!(just_parse::<NonZeroU32>(input), Err(ParseError));
    }
    assert_eq!(
        just_parse::<NonZeroU32>("4294967295").unwrap().1.get(),
        u32::MAX
    );
    for input in ["0", "-0", "2147483648", "-2147483649"] {
        assert_eq!(just_parse::<NonZeroI32>(input), Err(ParseError));
    }
    assert!(just_parse::<Backet>("Backet{\"asset_id\":\"usd\",\"count\":0,}").is_err());
    assert!(just_parse::<LogLine>("System::Error NetworkError \"ok\" requestid=0").is_err());
}

#[test]
fn authentication_data_has_fixed_size_and_compact_representation() {
    let input = format!("{}tail", "a5".repeat(1024));
    let (remaining, auth) = just_parse::<AuthData>(&input).unwrap();
    assert_eq!(remaining, "tail");
    assert_eq!(auth.as_bytes(), &[0xa5; 1024]);
    assert_eq!(
        std::mem::size_of::<AuthData>(),
        std::mem::size_of::<usize>()
    );
    assert!(std::mem::size_of::<AppLogTraceKind>() < 128);
    assert!(std::mem::size_of::<LogKind>() < 256);
    assert!(just_parse::<AuthData>(&"a5".repeat(1023)).is_err());
    assert!(just_parse::<AuthData>(&format!("{}я", "a5".repeat(1023))).is_err());
}

#[test]
fn parser_rejects_incomplete_and_malformed_input() {
    for input in [
        "Backet{\"asset_id\":\"usd\",\"count\":1}",
        "Backet{\"asset_id\":\"usd\",}",
        "Backet{\"asset_id\":\"usd\",\"count\":1,",
        "Backet{\"asset_id\":\"usd,\"count\":1,}",
    ] {
        assert!(just_parse::<Backet>(input).is_err());
    }
}

#[test]
fn unrelated_prototype_behavior_is_preserved() {
    // Это существующее поведение прототипа, а не исправление предметной логики.
    let (_, log) = just_parse::<LogKind>(
        "App::Journal WithdrawCash UserCash{\"user_id\":\"Bob\",\"count\":1,}",
    )
    .unwrap();
    assert!(matches!(
        log,
        LogKind::App(AppLogKind::Journal(AppLogJournalKind::DepositCash(_)))
    ));
}
