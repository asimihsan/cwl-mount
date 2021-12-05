# `cwl-mount`

`cwl-mount` mounts an AWS CloudWatch Logs log group as a file system. This lets you use everyday utilities
like `cat`, `grep`, and shell globbing and query your logs.

## Demo

## Getting started

`cwl-mount` natively supports Linux and Mac OS X, and supports Windows by running in a Docker container.

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

## Maintainer instructions

### Building runnable Docker container

First build DEB, then:

```
docker build . --file Dockerfile.runnable --tag cwl-mount:latest
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

TODO

### References

How to set up Rust actors

-   Rust actors, channels, tasks: https://ryhl.io/blog/actors-with-tokio/
-   Rust channels and tasks: https://tokio.rs/tokio/tutorial/channels

Packaging

-   https://agmprojects.com/blog/packaging-a-game-for-windows-mac-and-linux-with-rust.html