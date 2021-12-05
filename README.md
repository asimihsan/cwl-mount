
<h1 align="center">
  `cwl-mount`
</h1>

<h4 align="center">Mount AWS CloudWatch logs as a file system.</h4>

<!--
<p align="center">
  <a href="https://badge.fury.io/js/electron-markdownify">
    <img src="https://badge.fury.io/js/electron-markdownify.svg"
         alt="Gitter">
  </a>
  <a href="https://gitter.im/amitmerchant1990/electron-markdownify"><img src="https://badges.gitter.im/amitmerchant1990/electron-markdownify.svg"></a>
  <a href="https://saythanks.io/to/bullredeyes@gmail.com">
      <img src="https://img.shields.io/badge/SayThanks.io-%E2%98%BC-1EAEDB.svg">
  </a>
  <a href="https://www.paypal.me/AmitMerchant">
    <img src="https://img.shields.io/badge/$-donate-ff69b4.svg?maxAge=2592000&amp;style=flat">
  </a>
</p>
-->

<p align="center">
  <a href="#key-features">Key Features</a> •
  <a href="#how-to-use">How To Use</a> •
  <a href="#download">Installation</a> •
  <a href="#credits">Credits</a> •
  <a href="#license">License</a>
</p>

<p>
`cwl-mount` mounts an AWS CloudWatch Logs log group as a file system. This lets you use everyday utilities
like `cat`, `grep`, and shell globbing and query your logs.
</p>

![screenshot](https://raw.githubusercontent.com/amitmerchant1990/electron-markdownify/master/app/img/markdownify.gif)

## Key Features

* Cross platform
  - Natively supports Linux and Mac OS X.
  - Run on Windows via a Docker container.

## How To Use

To clone and run this application, you'll need [Git](https://git-scm.com) and
[Node.js](https://nodejs.org/en/download/) (which comes with [npm](http://npmjs.com)) installed on your
computer. From your command line:

```bash
# Clone this repository
$ git clone https://github.com/amitmerchant1990/electron-markdownify

# Go into the repository
$ cd electron-markdownify

# Install dependencies
$ npm install

# Run the app
$ npm start
```

Note: If you're using Linux Bash for Windows, [see this
guide](https://www.howtogeek.com/261575/how-to-run-graphical-linux-desktop-applications-from-windows-10s-bash-shell/)
or use `node` from the command prompt.

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

## Credits

- This README file is based off of
  [`electron-markdownify`](https://github.com/amitmerchant1990/electron-markdownify#readme).

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

## License

Apache License 2.0
