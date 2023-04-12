use mashin_runtime::bold;
use mashin_sdk::CliLogger;
use std::io::Write;

pub(crate) fn init() {
    let logger = env_logger::Builder::from_env(env_logger::Env::default())
        .filter_module("mashin", log::LevelFilter::Info)
        .format(|buf, record| {
            let mut target = record.target().to_string();
            let is_provider = target.starts_with("mashin::provider");

            if let Some(line_no) = record.line() {
                target.push(':');
                target.push_str(&line_no.to_string());
            }
            if record.level() <= log::Level::Info {
                // Print ERROR, WARN, INFO and lsp_debug logs as they are
                if is_provider {
                    writeln!(
                        buf,
                        "[{}]: {}",
                        bold(target.replace("mashin::provider::", "provider:")),
                        record.args()
                    )
                } else {
                    writeln!(buf, "{}", record.args())
                }
            } else {
                // Add prefix to DEBUG or TRACE logs
                writeln!(
                    buf,
                    "{} RS - {} - {}",
                    record.level(),
                    target,
                    record.args()
                )
            }
        })
        .build();

    let cli_logger = CliLogger::new(logger);
    let max_level = cli_logger.filter();

    let r = log::set_boxed_logger(Box::new(cli_logger));
    if r.is_ok() {
        log::set_max_level(max_level);
    }

    r.expect("Could not install logger.");
}
