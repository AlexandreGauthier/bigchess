use clap::{App, AppSettings, Arg, ArgMatches};

pub fn parse() -> ArgMatches {
    let about = "\nRust backend to a chess GUI frontend. Comunicates in JSON through stdin/out.\nContribute at https://github.com/AlexandreGauthier/bigchess";

    App::new("bigchess-core")
        .about(about)
        .version("0.1.0 (Barely usable)")
        .arg(
            Arg::with_name("start")
                .long("start")
                .about("Start listening to STDIN"),
        )
        .setting(AppSettings::ArgRequiredElseHelp)
        .get_matches()
}
