// Пусть есть логи:
// System(requestid):
// - trace
// - error
// App(requestid):
// - trace
// - error
// - journal (человекочитаемая сводка)

// Есть прототип штуки, которая умеет:
// - парсить логи
// - фильтровать
//  -- по requestid
//  -- по ошибкам
//  -- по изменению счёта (купить/продать)

// Модель данных:
// - Пользователь (userid, имя)
// - Вещи
//  -- Предмет (assetid, название)
//  -- Набор (assetid, количество)
//      comment{-- Собственность (assetid, userid владельца, количество)}
//  -- Таблица предложения (assetid на assetid, userid продавца)
//  -- Таблица спроса (assetid на assetid, userid покупателя)
// - Операция App
//  -- Journal
//   --- Создать пользователя userid с уставным капиталом от 10usd и выше
//   --- Удалить пользователя
//   --- Зарегистрировать assetid с ликвидностью от 50usd
//   --- Удалить assetid (весь asset должен принадлежать пользователю)
//   --- Внести usd для userid (usd (aka доллар сша) - это тип asset)
//   --- Вывести usd для userid
//   --- Купить asset
//   --- Продать asset
//  -- Trace
//   --- Соединить с биржей
//   --- Получить данные с биржи
//   --- Локальная проверка корректности (упреждение ошибок в ответе)
//   --- Отправить запрос в биржу
//   --- Получить ответ от биржи
//  -- Error
//   --- нет asset
//   --- системная ошибка
// - Операция System
//  -- Trace
//   --- Отправить запрос
//   --- Получить ответ
//  -- Error
//   --- нет сети
//   --- отказано в доступе
#![forbid(unsafe_code)]

fn run() -> Result<(), Box<dyn std::error::Error>> {
    println!("Placeholder для экспериментов с cli");

    let parsing_demo =
        r#"[UserBackets{"user_id":"Bob","backets":[Backet{"asset_id":"milk","count":3,},],},]"#;
    let announcements =
        analysis::parse::just_parse::<analysis::parse::Announcements>(parsing_demo)?;
    println!("demo-parsed: {:?}", announcements);

    let filename = std::env::args_os()
        .nth(1)
        .ok_or("укажите файл логов: cargo run -- example.log")?;
    println!(
        "Trying opening file '{}' from directory '{}'",
        filename.to_string_lossy(),
        std::env::current_dir()?.to_string_lossy()
    );
    let mut file = std::fs::File::open(filename)?;

    let logs = analysis::read_log(&mut file, analysis::ReadMode::All, &[])?;
    println!("got logs:");
    logs.iter().for_each(|parsed| println!("  {:?}", parsed));
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Ошибка: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
