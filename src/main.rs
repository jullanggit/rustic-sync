use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    process::Command,
};

use clap::Parser;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    /// Machine hostname -> machine config
    machines: HashMap<String, MachineConfig>,
}

#[derive(Deserialize)]
struct MachineConfig {
    /// rustic's `--use-profile` option
    rustic_profile: String,
    /// rustic's `--filter-jq` option
    filter_jq: Option<String>,
}

#[derive(clap::Parser)]
struct Cli {
    #[arg(long)]
    config: PathBuf,
    #[command(subcommand)]
    command: Commands,
}
#[derive(clap::Subcommand)]
enum Commands {
    /// Sync owned snapshots
    Sync,
    /// Run `rustic forget` on owned snapshots
    Forget,
}

fn main() {
    let cli = Cli::parse();

    let config: Config =
        toml::from_str(&fs::read_to_string(cli.config).expect("Failed to read config file"))
            .expect("Failed to deserialize config");

    let host = hostname::get()
        .expect("Failed to get hostname")
        .into_string()
        .expect("Hostname should be valid utf8");
    let host_config = config
        .machines
        .get(&host)
        .expect("Host machine not found in config");

    match cli.command {
        Commands::Sync => {
            config
                .machines
                .par_iter()
                .filter(|(machine, _)| machine != &&host)
                .for_each(|(other_machine, other_config)| {
                    // --- send owned snapshots ---

                    let mut command = Command::new("rustic");
                    command.args([
                        "copy",
                        "--init",
                        "--use-profile",
                        &host_config.rustic_profile,
                        "--target",
                        &other_config.rustic_profile,
                        // only copy this host's snapshot to other devicces
                        "--filter-host",
                        &host,
                    ]);
                    if let Some(ref jq) = other_config.filter_jq {
                        command.args(["--filter-jq", jq]);
                    }

                    run_rustic_cmd(command, "sync", "synced", &format!(" to {other_machine}"));

                    // --- delete non-owned snapshots ---

                    fn get_owned_snapshots(
                        owner: &str,
                        rustic_profile: &str,
                    ) -> Option<HashSet<String>> {
                        /// Structure returned by `rustic snapshots --json`
                        #[derive(Deserialize)]
                        struct Snapshots {
                            // only field we care about
                            snapshots: Vec<Snapshot>,
                        }
                        #[derive(Deserialize)]
                        struct Snapshot {
                            // only field we care about
                            id: String,
                        }

                        let mut command = Command::new("rustic");
                        command.args([
                            "--use-profile",
                            rustic_profile,
                            "snapshots",
                            "--filter-host",
                            owner,
                            "--json",
                        ]);
                        run_rustic_cmd(command, "list", "listed", &format!(" from {owner}")).map(
                            |stdout| {
                                let snapshots: Vec<Snapshots> = serde_json::from_slice(&stdout)
                                    .expect("Failed to deserialize snapshots");

                                snapshots
                                    .into_iter()
                                    .flat_map(|snapshots| snapshots.snapshots)
                                    .map(|snapshot| snapshot.id)
                                    .collect()
                            },
                        )
                    }

                    if let Some(others_owned_snapshots) =
                        get_owned_snapshots(other_machine, &other_config.rustic_profile)
                    {
                        let local_others_owned_snapshots =
                            get_owned_snapshots(other_machine, &host_config.rustic_profile)
                                .expect("Should be able to get local snapshots");

                        let only_local =
                            local_others_owned_snapshots.difference(&others_owned_snapshots);
                        let mut command = Command::new("rustic");
                        command
                            .args(["--use-profile", &host_config.rustic_profile, "forget"])
                            .args(only_local.collect::<Vec<&String>>());

                        run_rustic_cmd(command, "forget non-owned", "forgot non-owned", "");
                    }
                })
        }
        Commands::Forget => {
            let mut command = Command::new("rustic");
            command.args([
                "--use-profile",
                &host_config.rustic_profile,
                "forget",
                "--filter-host",
                &host,
            ]);

            run_rustic_cmd(command, "forget owned", "forgot owned", "");
        }
    }
}

/// Run rustic command and return stdout on success
fn run_rustic_cmd(
    mut command: Command,
    infinitive: &str,
    past_participle: &str,
    suffix: &str,
) -> Option<Vec<u8>> {
    println!("Running {command:?}");

    let output = command.output().expect("Failed to spawn rustic process");

    match output.status.success() {
        true => {
            println!("Successfully {past_participle} snapshots{suffix}");
            Some(output.stdout)
        }
        false => {
            println!(
                "Warning: failed to {infinitive} snapshots{suffix}:\ncommand status code: {}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            );
            None
        }
    }
}
