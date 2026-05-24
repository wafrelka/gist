use std::io::Write;

use anyhow::Context;
use chrono::{DateTime, Local, Utc};
use tracing::{error, info};

use crate::{
    Cli,
    cli::Command,
    repository::{Gist, Repository},
};

pub fn run(cli: Cli, now: DateTime<Utc>, mut output: impl Write) -> anyhow::Result<()> {
    let root = match cli.root {
        Some(root) => root,
        None => anyhow::bail!("gist root is not specified"),
    };
    let repository = Repository::open(&root)?;
    match cli.command {
        Command::Root => run_root(&repository, &mut output),
        Command::Create { names } => run_create(&repository, &names, now),
        Command::List => run_list(&repository, &mut output),
        Command::Archive { names } => run_archive(&repository, &names, now),
        Command::Unarchive { names } => run_unarchive(&repository, &names),
    }
}

fn run_root(repository: &Repository, output: &mut impl Write) -> anyhow::Result<()> {
    writeln!(output, "{}", repository.root().display()).context("cannot write to output")
}

fn run_create(
    repository: &Repository,
    names: &[String],
    created_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    run_each(names, |name| {
        repository.create(name, created_at)?;
        info!("created {name}");
        Ok(())
    })
}

fn run_list(repository: &Repository, output: &mut impl Write) -> anyhow::Result<()> {
    let mut gists = Vec::new();
    for gist in repository.list()? {
        match gist {
            Ok(gist) => gists.push(gist),
            Err(err) => error!("{err:?}"),
        }
    }

    gists.sort_by(|a, b| {
        a.archived
            .cmp(&b.archived)
            .then_with(|| b.created_at.cmp(&a.created_at))
            .then_with(|| a.name.cmp(&b.name))
    });

    for gist in gists {
        print_gist(output, &gist)?;
    }

    Ok(())
}

fn print_gist(output: &mut impl Write, gist: &Gist) -> anyhow::Result<()> {
    let created_at = gist.created_at.with_timezone(&Local).format("%Y-%m-%d %H:%M");
    if gist.archived {
        writeln!(output, "{created_at} | {} (archived)", gist.name)
    } else {
        writeln!(output, "{created_at} | {}", gist.name)
    }
    .context("cannot write to output")
}

fn run_archive(
    repository: &Repository,
    names: &[String],
    archived_at: DateTime<Utc>,
) -> anyhow::Result<()> {
    run_each(names, |name| {
        repository.archive(name, archived_at)?;
        info!("archived {name}");
        Ok(())
    })
}

fn run_unarchive(repository: &Repository, names: &[String]) -> anyhow::Result<()> {
    run_each(names, |name| {
        repository.unarchive(name)?;
        info!("unarchived {name}");
        Ok(())
    })
}

fn run_each(names: &[String], mut f: impl FnMut(&str) -> anyhow::Result<()>) -> anyhow::Result<()> {
    let mut failed = false;
    for name in names {
        if let Err(err) = f(name) {
            error!("{err:?}");
            failed = true;
        }
    }
    if failed {
        anyhow::bail!("some gists failed");
    }
    Ok(())
}
