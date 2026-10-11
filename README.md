Sync a rustic repository across multiple machines.

## Local-first Backups

Each machine backs up to their local repository, which is then synced with other devices when they are reachable.

## Syncing

### Transfer

Snapshots are transferred between machines using `rustic copy`.

### Ownership

Every machine "owns" it's own snapshots, and is the authoritative source for them. This means concretely:

1. `rustic-sync forget` only runs on snapshots the machine owns
2. Only owned snapshots are synced to other devices.
3. Devices `forget` snapshots that the owner no longer has

This together ensures consistency across the cluster.

### Sharing

By default, all machines receive all snapshots. To avoid syncing certain snapshots to certain machines, every machine can specify a `filter-jq` string, which will be passed to rustic's `--filter-jq` option.

### Configuration

Every machine's configuration configures rustic profiles and optionally filters for all other machines. This is necessary as the decision whether to send a snapshot is made on the source machine, not the target machine.

Different machine's rustic profiles for other machines don't have to match. For example the local machine's profile can use a plain local-filesystem repository for itself, while other machines need to communicate with it over ssh, sftp, etc.

### Pruning

Due to the aforementioned design decisions, every machine can run `rustic prune` independently, without coordination.

### Initialisation

If the target repository does not yet exist, it will be created. This is done by passing `--init` to `rustic copy`.

## Runtime requirements

rustic-sync requires `rustic` to be in $PATH and has to have the required permissions to run `rustic copy`. The passed rustic configs also need to have a non-interactive method for retrieving the password configured, as the commands are run in the background without user interaction.
