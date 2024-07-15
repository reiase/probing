use std::{ffi::OsString, marker::PhantomData, path::PathBuf, str::FromStr};

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use rustyline::{
    completion::{Completer, Pair},
    config::Configurer,
    error::ReadlineError,
    highlight::Highlighter,
    hint::Hinter,
    history::DefaultHistory,
    validate::Validator,
    CompletionType, Editor, Helper,
};

use super::{commands::ReplCommands, ctrl::CtrlChannel};

pub struct Repl<C: Parser + Send + Sync + 'static> {
    editor: Editor<LineReaderHelper<C>, DefaultHistory>,
}

impl<C: Parser + Send + Sync + 'static> Default for Repl<C> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C: Parser + Send + Sync + 'static> Repl<C> {
    pub fn new() -> Self {
        let mut editor = Editor::<LineReaderHelper<C>, DefaultHistory>::new().unwrap();
        editor.set_color_mode(rustyline::ColorMode::Enabled);
        editor.set_auto_add_history(true);
        editor.set_edit_mode(rustyline::EditMode::Emacs);
        editor.set_completion_type(CompletionType::List);
        editor.set_helper(Some(LineReaderHelper {
            phantom: PhantomData,
        }));
        Self { editor }
    }
}

impl<C: Parser + Send + Sync + 'static> Repl<C> {
    pub fn read_command(&mut self, prompt: &str) -> Result<C> {
        let line = self.editor.readline(prompt);
        match line {
            Ok(line) => match shlex::split(&line) {
                Some(args) => match C::try_parse_from(
                    std::iter::once("").chain(args.iter().map(String::as_str)),
                ) {
                    Ok(cmd) => {
                        self.editor.add_history_entry(line)?;
                        return Ok(cmd);
                    }
                    Err(e) => return Err(e.into()),
                },
                None => Err(anyhow::format_err!("")),
            },
            Err(ReadlineError::Interrupted) => todo!(),
            Err(_) => todo!(),
        }
    }
}

pub struct LineReaderHelper<C: Parser + Send + Sync + 'static> {
    phantom: PhantomData<C>,
}

impl<C: Parser + Send + Sync + 'static> Helper for LineReaderHelper<C> {}
impl<C: Parser + Send + Sync + 'static> Hinter for LineReaderHelper<C> {
    type Hint = String;
}
impl<C: Parser + Send + Sync + 'static> Highlighter for LineReaderHelper<C> {}
impl<C: Parser + Send + Sync + 'static> Validator for LineReaderHelper<C> {}
impl<C: Parser + Send + Sync + 'static> Completer for LineReaderHelper<C> {
    type Candidate = Pair;

    fn complete(
        &self, // FIXME should be `&mut self`
        line: &str,
        pos: usize,
        ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Self::Candidate>)> {
        let _ = (line, pos, ctx);

        let cmd = C::command();
        let mut cmd = clap_complete::dynamic::shells::CompleteCommand::augment_subcommands(cmd);
        let args = shlex::Shlex::new(line);
        let mut args = std::iter::once("".to_owned())
            .chain(args)
            .map(OsString::from)
            .collect::<Vec<_>>();
        if line.ends_with(' ') {
            args.push(OsString::new());
        }
        let arg_index = args.len() - 1;

        let pos = pos - args[arg_index].len();
        if let Ok(candidates) = clap_complete::dynamic::complete(
            &mut cmd,
            args,
            arg_index,
            PathBuf::from_str(".").ok().as_deref(),
        ) {
            let candidates = candidates
                .into_iter()
                .map(|c| {
                    let display = format!(
                        "{}: {}",
                        c.0.to_string_lossy(),
                        if let Some(s) = c.1 {
                            s.to_string()
                        } else {
                            "".to_string()
                        }
                    );
                    let replacement = c.0.to_string_lossy().to_string();
                    Self::Candidate {
                        display,
                        replacement,
                    }
                })
                .collect::<Vec<_>>();
            Ok((pos, candidates))
        } else {
            Ok(Default::default())
        }
    }
}

/// Repl debugging shell
#[derive(Args, Debug)]
pub struct ReplCommand {}

impl ReplCommand {
    pub fn run(&self, ctrl: CtrlChannel) -> Result<()> {
        let mut repl = Repl::<ReplCommands>::default();
        loop {
            let line = repl.read_command(">>")?;
            println!("{:?}", line);
        }
    }
}
