# Custom path for `thin-edge` config files

The `tedge` CLI `--config-dir` option can be used to store the thin-edge
configuration files in a custom directory.

## Create all the directories
Create all the directories that are needed by the thin-edge for its operations in a custom path as below.

```shell
tedge --config-dir path/to/config/dir --init
```

All the directories will be created in the `path/to/config/dir` directory.

Create the directories and files that are required by the `tedge_mapper`
and `tedge_agent` in a custom directory as below.

```shell
tedge_mapper --config-dir path/to/config/dir --init c8y
tedge_agent --config-dir path/to/config/dir --init
```

## Manage the configuration parameters

The configuration parameters can be set/unset/list in a config file that is present
in a custom directory.

For example, the config parameter can be set as below

```shell
tedge --config-dir path/to/config/dir config set c8y.url your.cumulocity.io
```

## Manage the certificate

This involves create/remove/upload the certificate.

For example, the certificate can be created as below.


```shell
tedge --config-dir path/to/config/dir cert create --device-id thinedge
```

Now the certificate will be created in the `path/to/config/dir` directory.


## Connect to cloud

To connect to cloud with the `--config-dir` option, two steps are required.

### Step 1: Update the `mosquitto.conf`

Since the bridge configuration files for Cumulocity IoT or Azure IoT Hub will be created in a directory given through `--config-dir`,
the path to the bridge configuration files must be found by `mosquitto`.
So, this line has to be added to your `mosquitto.conf` file manually.

`include_dir path/to/config/dir/tedge/mosquitto-conf`

### Step 2: `tedge connect <cloud> using the `--config-dir` option

Use the below command to connect to `Cumulocity IoT or Azure IoT Hub` cloud using `--config-dir`

```shell
tedge --config-dir path/to/config/dir connect c8y/az
```
Here the `path/to/config/dir` is the directory where the configuration files are present.

