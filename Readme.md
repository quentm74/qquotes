# qquotes

> Manage quotes you want to save with qquotes CLI for Ubuntu. Made with Rust

- Save, list and remove quotes

## Installation

### Binary
Binary can be downloaded from [here](https://github.com/quentm74/qquotes/releases).

### Development 

> Rust and cargo must be installed.

```sh
git clone git@github.com:quentm74/qquotes.git
cargo run -- --help
```

## Config

> By default, logs can be found in ~/qquotes.log and data in ~/qquotes_data.json.

Default configuration can be overrided via the file ~/.config/qquotes/config.tolm.

```sh
touch ~/.config/qquotes/config.tolm
```

### Configuration example

```sh
mkdir ~/qquotes/
```

**~/.config/qquotes/config.tolm**
```toml
path_log_file = "~/qquotes/qquotes.log"
path_data_file = "~/qquotes/data.json"
```

## Logs
By default, logs can be found in ~/qquotes.log. Verbose is also available in CLI.