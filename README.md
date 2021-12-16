
<h1 align="center">
  cwl-mount
</h1>

<h4 align="center">Mount AWS CloudWatch logs as a file system.</h4>

<p align="center">
  <a href="https://cirrus-ci.com/github/asimihsan/cwl-mount">
    <img src="https://api.cirrus-ci.com/github/asimihsan/cwl-mount.svg"
         alt="Build Status">
  </a>
</p>

<p align="center">
  <a href="#key-features">Key Features</a> •
  <a href="#how-to-use">How To Use</a> •
  <a href="#installation">Installation</a> •
  <a href="#credits">Credits</a> •
  <a href="#license">License</a>
</p>

`cwl-mount` mounts an AWS CloudWatch Logs log group as a file system. This lets you use everyday utilities
like `cat`, `grep`, and shell globbing and query your logs.

![screenshot](https://cwl-mount-readme.s3.us-west-2.amazonaws.com/demo.gif)

## What problem does `cwl-mount` solve

AWS CloudWatch Logs Insights is powerful but only returns a maximum of 10,000 results with no option to
paginate results [1]. AWS CloudWatch Logs lets you filter logs but does not allow you to show logs before and
after matches. You can export CloudWatch logs to S3 but this can take up to 12 hours [3]. You can stream your
CloudWatch Logs to another data store but will pay for the streaming out and extra infrastructure [4].

Filling in the gap, `cwl-mount` has no upper limit on the number of logs you can search over, allows you to
run `grep -C` to search with context, can be used immediately, and does not require additional infrastructure.

[1]
https://docs.aws.amazon.com/AmazonCloudWatchLogs/latest/APIReference/API_StartQuery.html#API_StartQuery_RequestSyntax

[2] https://docs.aws.amazon.com/AmazonCloudWatch/latest/logs/cloudwatch_limits_cwl.html

[3] https://docs.aws.amazon.com/AmazonCloudWatch/latest/logs/S3Export.html

[4] https://docs.aws.amazon.com/AmazonCloudWatch/latest/logs/Subscriptions.html

## Key Features

* Access CloudWatch logs as if they are files. Use `cat` on certain time ranges, `grep --context`, etc.
* Query latest logs instantaneouly; don't wait for S3 exports or transferring to a stream.
* Cross platform
  - Natively supports Linux and Mac OS X.
  - Run on Windows via a Docker container.

## How To Use

You need IAM credentials for a IAM user or IAM role that can call
[`logs:FilterLogEvents`](https://docs.aws.amazon.com/AmazonCloudWatchLogs/latest/APIReference/API_FilterLogEvents.html)
and
[`logs:DescribeLogGroups`](https://docs.aws.amazon.com/AmazonCloudWatchLogs/latest/APIReference/API_DescribeLogGroups.html).

### Natively on Linux and Mac OS X

In one Terminal tab:

```
mkdir /tmp/foo
cwl-mount --region us-west-2 --log-group-name babynames-preprod-log-group-syslog /tmp/foo
```

In a second tab:

```
➜  ~ ls -l /tmp/foo

total 0
drwxrwxrwx  2 asimi  staff  0 Dec 31  1969 2021

➜  ~ ls -l /tmp/foo/2021/12/04/00-{00,01,02,03,04,05}
-rwxrwxrwx  1 asimi  staff  2147483647 Dec 31  1969 /tmp/foo/2021/12/04/00-00
-rwxrwxrwx  1 asimi  staff  2147483647 Dec 31  1969 /tmp/foo/2021/12/04/00-01
-rwxrwxrwx  1 asimi  staff  2147483647 Dec 31  1969 /tmp/foo/2021/12/04/00-02
-rwxrwxrwx  1 asimi  staff  2147483647 Dec 31  1969 /tmp/foo/2021/12/04/00-03
-rwxrwxrwx  1 asimi  staff  2147483647 Dec 31  1969 /tmp/foo/2021/12/04/00-04
-rwxrwxrwx  1 asimi  staff  2147483647 Dec 31  1969 /tmp/foo/2021/12/04/00-05

➜  ~ cat /tmp/foo/2021/12/04/00-{00,01,02,03,04,05}
[i-03e71e7954a899acb] Dec  4 00:00:07 ip-10-0-0-62 systemd[1]: Starting Rotate log files...
[i-03e71e7954a899acb] Dec  4 00:00:07 ip-10-0-0-62 systemd[1]: Starting Daily man-db regeneration...
[i-03e71e7954a899acb] Dec  4 00:00:07 ip-10-0-0-62 systemd[1]: logrotate.service: Succeeded.
[i-03e71e7954a899acb] Dec  4 00:00:07 ip-10-0-0-62 systemd[1]: Finished Rotate log files.
[i-03e71e7954a899acb] Dec  4 00:00:07 ip-10-0-0-62 systemd[1]: man-db.service: Succeeded.
[i-03e71e7954a899acb] Dec  4 00:00:07 ip-10-0-0-62 systemd[1]: Finished Daily man-db regeneration.[i-03e71e7954a899acb] Dec  4 00:03:01 ip-10-0-0-62 CRON[40987]: (root) CMD (/bin/sleep $[ ( $RANDOM % 3000 ) + 1 ]s; rm -f /var/log/awsagent-update.log; umask 037 && /opt/aws/awsagent/bin/update > /var/log/awsagent-update.log 2>&1)%
```

### Docker, for any OS

Since `cwl-mount` requires FUSE it will not work out of the box on Windows. You can instead use a [Docker
container](https://gallery.ecr.aws/b5u6b4p0/cwl-mount):

```
docker run \
    --privileged \
    --interactive \
    --tty \
    --env-file $HOME/.aws_kitten_cat_credentials_docker \
    public.ecr.aws/b5u6b4p0/cwl-mount:latest
```

The contents of `env-file` are some IAM role credentials that have access to
[`logs:FilterLogEvents`](https://docs.aws.amazon.com/AmazonCloudWatchLogs/latest/APIReference/API_FilterLogEvents.html)
and
[`logs:DescribeLogGroups`](https://docs.aws.amazon.com/AmazonCloudWatchLogs/latest/APIReference/API_DescribeLogGroups.html),
and look like:

```
AWS_ACCESS_KEY_ID=ABCDE
AWS_SECRET_ACCESS_KEY=fghij
AWS_REGION=us-west-2
```

### Usage help

Usage help:

```
cwl-mount 0.1.1

USAGE:
    cwl-mount [FLAGS] [OPTIONS] <mount-point> --log-group-name <log-group-name>

FLAGS:
        --allow-root    Allow root user to access filesystem
    -h, --help          Prints help information
    -V, --version       Prints version information
    -v, --verbose       Verbose output. Set three times for maximum verbosity.

OPTIONS:
        --log-group-name <log-group-name>    CloudWatch Logs log group name
        --region <region>                    AWS region, e.g. 'us-west-2'

ARGS:
    <mount-point>    Act as a client, and mount FUSE at given path
```

### Troubleshooting

If you get an error about the directory already being mounted, try `umount /tmp/foo` first.

I recommend always passing in the AWS region in `--region`, even if you have the `AWS_REGION` environment
variable set, otherwise STS temporary credentials may not work.

## Installation

Linux with RPM:

```
wget https://github.com/asimihsan/cwl-mount/releases/download/v0.1.1/cwl-mount-0.1.1-1-x86_64.rpm
yum localinstall cwl-mount-0.1.1-1-x86_64.rpm
cwl-mount --help
```

Linux with DEB:

```
wget https://github.com/asimihsan/cwl-mount/releases/download/v0.1.1/cwl-mount-0.1.1-1-x86_64.deb
apt -y install gdebi
gdebi cwl-mount-0.1.1-1-x86_64.deb
cwl-mount --help
```

Mac:

```
# If this is the first time you've installed macfuse, you will need to restart after this.
brew install macfuse

mkdir $HOME/bin
wget https://github.com/asimihsan/cwl-mount/releases/download/v0.1.1/cwl-mount-0.1.1-darwin-x64_64.tar.gz
tar xvf cwl-mount-0.1.1-darwin-x64_64.tar.gz --directory $HOME/bin
$HOME/bin/cwl-mount --help
```

## Credits

- This README file is based off of
  [`electron-markdownify`](https://github.com/amitmerchant1990/electron-markdownify#readme).

## Maintainer instructions

`./release.sh` combines all the steps below in the correct order.

### Building runnable Docker container and publishing it

```
docker build --squash . --file Dockerfile.runnable --tag cwl-mount:latest

source ~/.aws_kitten_cat_credentials
aws ecr-public get-login-password --region us-east-1 | docker login --username AWS --password-stdin public.ecr.aws/kittencat
docker tag cwl-mount:latest public.ecr.aws/kittencat/cwl-mount:latest
docker push public.ecr.aws/kittencat/cwl-mount:latest
```

To run it:

```
docker run \
    --privileged \
    --interactive \
    --tty \
    --env-file $HOME/.aws_kitten_cat_credentials_docker \
    cwl-mount:latest
```

The contents of `env-file` are some credentials like:

```
AWS_ACCESS_KEY_ID=ABCDE
AWS_SECRET_ACCESS_KEY=fghij
AWS_REGION=us-west-2
```

### Building RPM

```
docker build . --file Dockerfile.amazonlinux2 --tag cwl-mount-al2:latest

docker run \
    --privileged \
    --interactive \
    --tty \
    --volume "$(pwd):/workspace" \
    --workdir /workspace \
    --env-file $HOME/.aws_kitten_cat_credentials_docker \
    cwl-mount-al2:latest ./build_rpm.sh

rmdir /tmp/foo ; mkdir /tmp/foo
docker run \
    --cap-add SYS_ADMIN \
    --device /dev/fuse \
    --privileged \
    --interactive \
    --tty \
    --volume "$(pwd):/workspace" \
    --volume "/tmp/foo:/tmp/foo" \
    --workdir /workspace \
    --env-file $HOME/.aws_kitten_cat_credentials_docker \
    public.ecr.aws/amazonlinux/amazonlinux:2 ./test_rpm.sh
```

### Building DEB

```
docker build . --file Dockerfile.debian --tag cwl-mount-debian:latest

docker run \
    --privileged \
    --interactive \
    --tty \
    --volume "$(pwd):/workspace" \
    --workdir /workspace \
    --env-file $HOME/.aws_kitten_cat_credentials_docker \
    cwl-mount-debian:latest ./build_deb.sh
```

### Building for Mac

```
./build_mac.sh
```

### Recording a demo

See: https://terminalizer.com/docs

```
npm install -g terminalizer

[ ! -d $HOME/demo ] && mkdir $HOME/demo
cd $HOME/demo
terminalizer record demo

# inside the demo...

tmux
# ...

# when you're done
CTRL-D

terminalizer render demo
```

### References

How to set up Rust actors

-   Rust actors, channels, tasks: https://ryhl.io/blog/actors-with-tokio/
-   Rust channels and tasks: https://tokio.rs/tokio/tutorial/channels

## License

Apache License 2.0
