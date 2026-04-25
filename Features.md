# ripr

A tool to return specific lines from a file. It is similar to `head` and `tail`, but allows you to specify any range of lines. By default, it does not allow access to any files. A configuration file is required to whitelist allowed places to read from.

## Features

- Return specific lines from a file, by default with inclusive range. (`ripr 5-10 file.txt`, or `ripr 5,10 file.txt`)
- Use an exclusive range on either end by using the appropriate parentheses. (`ripr '(5-10' file.txt`, or `ripr '5,10)' file.txt` for starting at line 6, or ending at line 9, respectively)
- Also support sed syntax for the same functionality. (`ripr -n '5,10p' file.txt` so it can be used as a drop-in replacement for `sed -n '5,10p' file.txt` in AI tooling)
- Whitelist specific files and directories in a configuration file.
- Prevent access to files that are not whitelisted.
- Simple command-line interface.
- Written in Rust for performance and safety.
- command-line subcommand for adding a folder or file to the whitelist (`ripr whitelist add /path/to/file_or_folder`, `ripr whitelist remove /path/to/file_or_folder`)
