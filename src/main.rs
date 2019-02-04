extern crate clap;
extern crate config as c;
extern crate jfs;
#[macro_use]
extern crate log;
#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate serde_derive;
extern crate shellexpand;
extern crate simplelog;
extern crate textwrap;
extern crate unicode_width;

use clap::{App, Arg, ArgMatches, SubCommand};
use jfs::Store as Store;
use prettytable::{format, Row, Table};
use simplelog::*;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io;
use std::io::Write;
use std::path::Path;
use std::process;

static PATH_CONFIG_FILE: &'static str = "~/.config/qquotes/config.toml";
static DEFAULT_PATH_LOG_FILE: &'static str = "~/qquotes.log";
static DEFAULT_PATH_DATA_FILE: &'static str = "~/qquotes_data.json";

fn main() {
    let matches = App::new("qquotes")
        .about("Store quotes")
        .version("1.0.0")
        .author("Quentin Michel <quentinmichel69110@gmail.com>")
        .arg(Arg::with_name("verbose")
            .help("Shows details about the results of running qquotes")
            .short("v")
            .long("verbose")
            .takes_value(false)
            .multiple(true))
        .subcommand(SubCommand::with_name("add")
            .about("Add a quote"))
        .subcommand(SubCommand::with_name("list")
            .about("Prints all quotes")
            .arg(Arg::with_name("long-format")
                .help("Display all information such as IDs")
                .short("l")
                .long("long-format")))
        .subcommand(SubCommand::with_name("delete")
            .about("Delete a quote by ID")
            .arg(Arg::with_name("QUOTE_ID")
                .help("ID of the quote you want to delete")
                .required(true)
                .takes_value(true)
                .multiple(false)))
        .get_matches();
    if let Err(e) = run(matches) {
        error!("{}", e);
        process::exit(1);
    }
    trace!("exit_app");
}

//------------------------------------------------------------------------------------------------------
// Rooting
//------------------------------------------------------------------------------------------------------

fn run(matches: ArgMatches) -> Result<(), String> {
    // App setup
    // config file
    let (app_config, config_file_found) = get_config_parameter();
    // Init logger
    let term_min_log_level = match matches.occurrences_of("verbose") {
        0 => LevelFilter::Error,
        1 => LevelFilter::Warn,
        2 | _ => LevelFilter::Trace,
    };
    let mut log_term_config = Config::default();
    log_term_config.time_format = Some("[log]");
    let mut log_file_config = Config::default();
    log_file_config.time_format = Some("%Y-%m-%d %H-%M-%S");
    let log_file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(shellexpand::tilde(&app_config.log_path).into_owned());
    CombinedLogger::init(
        vec![
            TermLogger::new(term_min_log_level, log_term_config).unwrap(),
            WriteLogger::new(LevelFilter::Info, log_file_config, log_file.unwrap()),
        ]
    ).unwrap();
    trace!("app_setup");
    if config_file_found {
        trace!("config_file_loaded");
    } else {
        trace!("config_file_not_found");
    }
    trace!("config_parameter_log_path {}", &app_config.log_path);
    trace!("config_parameter_data_path {}", &app_config.data_path);
    // Repository
    let r: Repository;
    match Repository::new(&app_config.data_path) {
        Ok(repository) => {
            trace!("repository_initialized");
            r = repository
        }
        Err(e) => return Err(e)
    }
    trace!("app_setup_complete");
    // starting processing
    trace!("processing_started");
    match matches.subcommand() {
        ("add", Some(_)) => cmd_quote_add(r),
        ("list", Some(m)) => cmd_quote_list(r, m),
        ("delete", Some(m)) => cmd_quote_delete(r, m),
        _ => {
            println!("No default action. Please see qquotes --help for more information");
            Ok(())
        }
    }
}

//------------------------------------------------------------------------------------------------------
// Config
//------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct AppConfig<> {
    log_path: String,
    data_path: String,
}

fn get_config_parameter() -> (AppConfig, bool) {
    let default = AppConfig {
        log_path: DEFAULT_PATH_LOG_FILE.to_string(),
        data_path: DEFAULT_PATH_DATA_FILE.to_string(),
    };
    match Path::new(&shellexpand::tilde(PATH_CONFIG_FILE).into_owned()).exists() {
        true => {
            let mut app_config = default.clone();
            let mut settings = c::Config::default();
            settings.merge(c::File::with_name(&shellexpand::tilde(PATH_CONFIG_FILE).into_owned())).unwrap();
            match settings.get_str("path_log_file") {
                Ok(v) => app_config.log_path = v,
                Err(_) => (),
            }
            match settings.get_str("path_data_file") {
                Ok(v) => app_config.data_path = v,
                Err(_) => (),
            }
            return (app_config, true);
        }
        false => (),
    };
    (default, false)
}

//------------------------------------------------------------------------------------------------------
// Commands
//------------------------------------------------------------------------------------------------------

fn cmd_quote_add(r: Repository) -> Result<(), String> {
    let author: String;
    match ask("author") {
        Ok(v) => author = v,
        Err(e) => return Err(e.to_string()),
    };
    let quote: String;
    match ask("quote ") {
        Ok(v) => quote = v,
        Err(e) => return Err(e.to_string()),
    };
    match r.save_quote(Quote {
        author,
        quote,
    }) {
        Ok(_) => {
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn cmd_quote_list(r: Repository, args: &ArgMatches) -> Result<(), String> {
    match r.get_quotes() {
        Ok(quotes) => {
            if quotes.len() > 0 {
                format_and_display_quotes_list(quotes, args.is_present("long-format"));
            } else {
                println!("There is no quote saved.");
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn cmd_quote_delete(r: Repository, args: &ArgMatches) -> Result<(), String> {
    let id: String;
    match args.value_of("QUOTE_ID") {
        Some(v) => id = v.to_string(),
        None => return Err("Missing QUOTE_ID".to_string()),
    };
    match r.delete_quote(&id.to_string()) {
        Ok(_) => {
            Ok(())
        }
        Err(e) => Err(e),
    }
}

//------------------------------------------------------------------------------------------------------
// Format ask
//------------------------------------------------------------------------------------------------------

fn ask(label: &str) -> Result<String, io::Error> {
    print!("{} ⏵ ", label);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.replace("\n", ""))
}

//------------------------------------------------------------------------------------------------------
// Format list and display
//------------------------------------------------------------------------------------------------------

fn format_and_display_quotes_list(quotes: BTreeMap<String, Quote>, long_format: bool) {
    let titles: Row = if long_format {
        row!["QUOTE_ID", "Author", "Quote"]
    } else {
        row!["Author", "Quote"]
    };
    let mut rows: Vec<Row> = Vec::new();
    for (id, quote_obj) in quotes.iter() {
        let mut id_max_width = 0;
        let mut author_max_width = 0;
        for (id, quote) in quotes.iter() {
            if id.len() > id_max_width { id_max_width = id.len() }
            if quote.author.len() > author_max_width { author_max_width = quote.author.len() }
        }
        let term_width = textwrap::termwidth();
        let row = if long_format {
            row![id, quote_obj.author, textwrap::fill(&quote_obj.quote.as_str(), term_width - id_max_width - author_max_width - 8)]
        } else {
            row![quote_obj.author, textwrap::fill(&quote_obj.quote.as_str(), term_width - author_max_width - 5)]
        };
        rows.push(row);
    }
    display_quotes_table(titles, rows);
}

//------------------------------------------------------------------------------------------------------
// Repository
//------------------------------------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct Quote {
    author: String,
    quote: String,
}

struct Repository {
    store: jfs::Store,
}

impl Repository {
    fn new(path_data: &str) -> Result<Repository, String> {
        let mut cfg = jfs::Config::default();
        cfg.single = true;
        cfg.pretty = true;
        match Store::new_with_cfg(shellexpand::tilde(path_data).into_owned(), cfg) {
            Ok(store) => Ok(Repository {
                store
            }),
            Err(e) => Err(format!("{}", e)),
        }
    }

    fn save_quote(&self, quote: Quote) -> Result<Quote, String> {
        trace!("repository_save_quote {:?}", quote);
        let id = self.store.save(&quote).unwrap();
        let saved = self.store.get::<Quote>(&id).unwrap();
        info!("repository_saved_quote {:?}", saved);
        Ok(saved)
    }

    fn get_quotes(&self) -> Result<BTreeMap<String, Quote>, String> {
        trace!("repository_get_quotes");
        match self.store.all::<Quote>() {
            Ok(d) => Ok(d),
            Err(e) => Err(e.to_string()),
        }
    }

    fn delete_quote(&self, id: &String) -> Result<(), String> {
        trace!("repository_delete_quote id: {}", id);
        match self.store.delete(id) {
            Ok(d) => {
                info!("repository_delete_quote id: {:?}", id);
                Ok(d)
            }
            Err(e) => Err(e.to_string()),
        }
    }
}

//------------------------------------------------------------------------------------------------------
// Format list display
//------------------------------------------------------------------------------------------------------

fn display_quotes_table(mut titles: Row, rows: Vec<Row>) {
    let mut table = Table::new();
    for title in titles.iter_mut() {
        title.style(prettytable::Attr::Bold);
    }
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);
    table.set_titles(titles);
    for row in rows {
        table.add_row(row);
    }
    table.printstd();
}
