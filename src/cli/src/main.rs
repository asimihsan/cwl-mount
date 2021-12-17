/*
 * Copyright Kitten Cat LLC. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

use chrono::prelude::*;
use chrono::Duration;

// See:
//
// - https://github.com/cberner/fuser/blob/c05bea58/examples/simple.rs

use clap::ArgGroup;
use clap::SubCommand;
use clap::{crate_version, App, Arg};
use cwl_lib::CloudWatchLogsActorHandle;
use cwl_lib::CloudWatchLogsImpl;
use fuse::create_file_tree_for_time_range;
use fuser::consts::FOPEN_DIRECT_IO;
use fuser::ReplyOpen;
use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, Request,
};
use libc::ENOENT;
use std::cmp::min;
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::io::Cursor;
use std::io::Read;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use tokio::runtime::Handle;
use tracing::Level;
use tracing::{debug, error, info};
use tracing_subscriber::FmtSubscriber;

const TTL: std::time::Duration = std::time::Duration::from_secs(1); // 1 second
const FMODE_EXEC: i32 = 0x20;
const EMPTY_BUFFER: [u8; 0] = [];

pub async fn prepare_file_tree(_cwl: &CloudWatchLogsImpl) -> fuse::FileTree {
    let end_time = Utc::now();
    let default_start_time = end_time - Duration::days(365);
    let start_time = default_start_time;

    // TODO use CloudWatch actor to get this start time
    // let start_time = cwl
    //     .get_first_event_time_for_log_group(log_group_name.into())
    //     .await
    //     .unwrap_or(Some(default_start_time))
    //     .unwrap_or(default_start_time);

    create_file_tree_for_time_range(start_time, end_time)
}

struct HelloFS {
    handle: Arc<Handle>,
    cwl_actor_handle: Arc<CloudWatchLogsActorHandle>,

    // Must use direct I/O for open files because we do not know how large files are before we do a network call,
    // and we don't want to have to know the file size before opening a file. This bypasses the OS page cache.
    // See: [1].
    // [1] https://stackoverflow.com/questions/46267972/fuse-avoid-calculating-size-in-getattr
    direct_io: bool,

    log_group_name: Option<String>,
    log_group_filter: Option<String>,
    file_tree: Arc<fuse::FileTree>,
}

impl HelloFS {
    pub fn new(
        handle: Handle,
        cwl: CloudWatchLogsImpl,
        log_group_name: Option<&str>,
        log_group_filter: Option<&str>,
        file_tree: Arc<fuse::FileTree>,
    ) -> Self {
        let direct_io = true;
        let cwl_actor_handle = Arc::new(CloudWatchLogsActorHandle::new(cwl));

        Self {
            handle: Arc::new(handle),
            cwl_actor_handle,
            direct_io,
            log_group_name: log_group_name.map(|s| s.to_string()),
            log_group_filter: log_group_filter.map(|s| s.to_string()),
            file_tree,
        }
    }
}

impl Filesystem for HelloFS {
    fn lookup(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let filename = name.to_string_lossy().to_string();
        debug!("lookup call. parent: {}, name: {}", parent, filename);
        let child = self.file_tree.get_child_for_inode(parent, filename);
        if child.is_none() {
            reply.error(ENOENT);
            return;
        }
        let child = child.unwrap();
        reply.entry(
            &TTL,
            &FileAttr {
                ino: child.file.inode,
                size: match child.file.file_type {
                    fuse::FileType::Directory => 0,
                    fuse::FileType::File(_) => i32::MAX as u64,
                },
                blocks: match child.file.file_type {
                    fuse::FileType::Directory => 0,
                    fuse::FileType::File(_) => 1,
                },
                atime: UNIX_EPOCH, // 1970-01-01 00:00:00
                mtime: UNIX_EPOCH,
                ctime: UNIX_EPOCH,
                crtime: UNIX_EPOCH,
                kind: match child.file.file_type {
                    fuse::FileType::Directory => FileType::Directory,
                    fuse::FileType::File(_) => FileType::RegularFile,
                },
                perm: match child.file.file_type {
                    fuse::FileType::Directory => 0o777,
                    fuse::FileType::File(_) => 0o777,
                },
                nlink: match child.file.file_type {
                    fuse::FileType::Directory => 2,
                    fuse::FileType::File(_) => 1,
                },
                uid: req.uid(),
                gid: req.gid(),
                rdev: 0,
                flags: 0,
                blksize: 512,
            },
            0,
        );
    }

    fn getattr(&mut self, req: &Request, ino: u64, reply: ReplyAttr) {
        debug!("getattr call. ino: {}", ino);
        let file = self.file_tree.get_file_by_inode(ino);
        if file.is_none() {
            reply.error(ENOENT);
            return;
        }
        let file = file.unwrap();
        match &file.file.file_type {
            fuse::FileType::Directory => {}
            fuse::FileType::File(_info) => {
                debug!("file: {:?}", file.file);
            }
        }
        reply.attr(
            &TTL,
            &FileAttr {
                ino: file.file.inode,
                size: match file.file.file_type {
                    fuse::FileType::Directory => 0,
                    fuse::FileType::File(_) => i32::MAX as u64,
                },
                blocks: match file.file.file_type {
                    fuse::FileType::Directory => 0,
                    fuse::FileType::File(_) => 1,
                },
                atime: UNIX_EPOCH, // 1970-01-01 00:00:00
                mtime: UNIX_EPOCH,
                ctime: UNIX_EPOCH,
                crtime: UNIX_EPOCH,
                kind: match file.file.file_type {
                    fuse::FileType::Directory => FileType::Directory,
                    fuse::FileType::File(_) => FileType::RegularFile,
                },
                perm: match file.file.file_type {
                    fuse::FileType::Directory => 0o777,
                    fuse::FileType::File(_) => 0o777,
                },
                nlink: match file.file.file_type {
                    fuse::FileType::Directory => 2,
                    fuse::FileType::File(_) => 1,
                },
                uid: req.uid(),
                gid: req.gid(),
                rdev: 0,
                flags: 0,
                blksize: 512,
            },
        )

        // match ino {
        //     1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
        //     2 => reply.attr(&TTL, &HELLO_TXT_ATTR),
        //     _ => reply.error(ENOENT),
        // }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        debug!("ino: {}, offset: {}, size: {}", ino, offset, size);
        let file_tree = Arc::clone(&self.file_tree);
        let file = file_tree.get_file_by_inode(ino);
        if file.is_none() {
            reply.error(ENOENT);
            return;
        }
        let file = file.unwrap().clone();
        match file.file.file_type {
            fuse::FileType::Directory => {
                reply.error(ENOENT);
                return;
            }
            fuse::FileType::File(time_bounds) => {
                let log_group_name = self.log_group_name.clone();
                let log_group_filter = self.log_group_filter.clone();
                let cwl_actor_handle = Arc::clone(&self.cwl_actor_handle);
                let (tx, rx) = crossbeam::channel::bounded(1);
                let handle = Arc::clone(&self.handle);
                handle.spawn(async move {
                    let res = cwl_actor_handle
                        .get_logs_to_display(
                            log_group_name,
                            log_group_filter,
                            time_bounds.start_time,
                            time_bounds.end_time,
                        )
                        .await;
                    let _ = tx.send(res);
                });
                let res = rx.recv().unwrap().unwrap();
                let file_size = res.len();
                debug!("logs to display: {:?}", res);
                let read_size = min(size, file_size.saturating_sub(offset as usize) as u32);
                if read_size == 0 {
                    reply.data(&EMPTY_BUFFER);
                    return;
                }
                let mut buffer = vec![0; read_size as usize];
                let res_as_slice = res.as_ref();
                let mut reader = Cursor::new(&res_as_slice[offset as usize..]);
                reader.read_exact(&mut buffer).unwrap();
                reply.data(&buffer);
            }
        }
    }

    fn open(&mut self, _req: &Request, inode: u64, flags: i32, reply: ReplyOpen) {
        debug!("open() called for {:?}", inode);
        let (_access_mask, _read, _write) = match flags & libc::O_ACCMODE {
            libc::O_RDONLY => {
                // Behavior is undefined, but most filesystems return EACCES
                if flags & libc::O_TRUNC != 0 {
                    reply.error(libc::EACCES);
                    return;
                }
                if flags & FMODE_EXEC != 0 {
                    // Open is from internal exec syscall
                    (libc::X_OK, true, false)
                } else {
                    (libc::R_OK, true, false)
                }
            }
            libc::O_WRONLY => (libc::W_OK, false, true),
            libc::O_RDWR => (libc::R_OK | libc::W_OK, true, true),
            // Exactly one access mode flag must be specified
            _ => {
                reply.error(libc::EINVAL);
                return;
            }
        };

        let file_tree = Arc::clone(&self.file_tree);
        match file_tree.get_file_by_inode(inode) {
            Some(file) => match file.file.file_type {
                fuse::FileType::Directory => {}
                fuse::FileType::File(_) => {
                    let open_flags = if self.direct_io { FOPEN_DIRECT_IO } else { 0 };
                    let fh = 10;
                    reply.opened(fh, open_flags);
                    return;
                }
            },
            None => {}
        }
        reply.error(libc::EACCES);
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        debug!("readdir, ino: {}, offset: {}", ino, offset);
        let directory = self.file_tree.get_file_by_inode(ino);
        if directory.is_none() {
            reply.error(ENOENT);
            return;
        }
        let directory = directory.unwrap();
        let children = self.file_tree.list_directory(directory.file_key);
        let mut entries: VecDeque<(u64, FileType, String)> = children
            .into_iter()
            .map(|file| {
                (
                    file.file.inode,
                    match file.file.file_type {
                        fuse::FileType::Directory => FileType::Directory,
                        fuse::FileType::File(_) => FileType::RegularFile,
                    },
                    file.file.name.clone(),
                )
            })
            .collect();
        let parent_inode = self.file_tree.get_parent_for_ls(directory.file_key).file.inode;
        entries.push_front((parent_inode, FileType::Directory, "..".to_string()));
        entries.push_front((parent_inode, FileType::Directory, ".".to_string()));

        // if ino != 1 {
        //     reply.error(ENOENT);
        //     return;
        // }
        // let entries = vec![
        //     (1, FileType::Directory, "."),
        //     (1, FileType::Directory, ".."),
        //     (2, FileType::RegularFile, "hello.txt"),
        // ];

        for (i, entry) in entries.into_iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            debug!("readdir add entry: {:?}", entry);
            if reply.add(entry.0, (i + 1) as i64, entry.1, entry.2) {
                break;
            }
        }
        reply.ok();
    }
}

/// Valid transactions per second (TPS) value fits in usize and is not zero.
pub fn is_valid_tps(v: String) -> Result<(), String> {
    match v.parse::<usize>() {
        Ok(value) => match value {
            0 => Err("Zero is not a valid transactions per second value".to_string()),
            _ => Ok(()),
        },
        Err(_) => Err(format!(
            "{} isn't a valid transactions per second value because not a positive integer",
            &*v
        )),
    }
}

#[tokio::main]
async fn main() {
    let matches = App::new("cwl-mount")
        .version(crate_version!())
        .subcommands(vec![
            SubCommand::with_name("list-log-groups").about("List AWS CloudWatch Logs log groups then quit."),
            SubCommand::with_name("mount")
                .about("Mount AWS CloudWatch Logs to a directory.")
                .arg(
                    Arg::with_name("mount-point")
                        .index(1)
                        .required(true)
                        .takes_value(true)
                        .help("Mount the AWS CloudWatch logs at the given directory"),
                )
                .arg(
                    Arg::with_name("log-group-name")
                        .long("log-group-name")
                        .takes_value(true)
                        .validator(regexes::clap_validate_cwl_log_group_name)
                        .help("CloudWatch Logs log group name"),
                )
                .arg(
                    Arg::with_name("log-group-filter")
                        .long("log-group-filter")
                        .takes_value(true)
                        .validator(regexes::validate_regex)
                        .help("CloudWatch Logs log group filter, a regular expression"),
                )
                .arg(
                    Arg::with_name("allow-root")
                        .long("allow-root")
                        .help("Allow root user to access filesystem"),
                )
                .group(
                    ArgGroup::with_name("log-group-specifiers")
                        .args(&["log-group-name", "log-group-filter"])
                        .required(true)
                        .multiple(false),
                ),
        ])
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .multiple(true)
                .help("Verbose output. Set three times for maximum verbosity."),
        )
        .arg(
            Arg::with_name("region")
                .long("region")
                .required(true)
                .takes_value(true)
                .help("AWS region, e.g. 'us-west-2'"),
        )
        .arg(
            Arg::with_name("tps")
                .long("tps")
                .takes_value(true)
                .validator(is_valid_tps)
                .default_value("5")
                .help("Transactions per second (TPS) at which to call AWS CloudWatch Logs."),
        )
        .get_matches();

    let region = matches.value_of("region");
    let tps = matches.value_of("tps").unwrap().parse::<usize>().unwrap();
    let tracing_level = match matches.occurrences_of("verbose") {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };
    let subscriber = FmtSubscriber::builder().with_max_level(tracing_level).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
    let cwl = CloudWatchLogsImpl::new(tps, region).await;

    match matches.subcommand() {
        ("list-log-groups", _matches) => {
            info!("listing log groups...");
            match cwl.get_log_group_names().await {
                Ok(log_group_names) => print!("{}", log_group_names.join("\n")),
                Err(err) => {
                    error!("Failed to list log groups: {:?}", err);
                }
            }
        }
        (_, matches) => {
            info!("mounting...");
            let matches = matches.unwrap();
            let log_group_name = matches.value_of("log-group-name");
            let log_group_filter = matches.value_of("log-group-filter");
            let mountpoint = matches.value_of("mount-point").unwrap();
            let mut options = vec![MountOption::RO, MountOption::FSName("hello".to_string())];
            if matches.is_present("allow-root") {
                options.push(MountOption::AllowRoot);
            }

            let file_tree = Arc::new(prepare_file_tree(&cwl).await);
            let hello_fs = HelloFS::new(
                Handle::current(),
                cwl,
                log_group_name,
                log_group_filter,
                file_tree,
            );

            // See: https://github.com/cberner/fuser/issues/179
            let (send, recv) = std::sync::mpsc::channel();
            ctrlc::set_handler(move || {
                info!("CTRL-C pressed");
                send.send(()).unwrap();
            })
            .unwrap();
            info!("starting...");
            let _guard = fuser::spawn_mount(hello_fs, mountpoint, &vec![]).unwrap();
            let () = recv.recv().unwrap();
        }
    }

    info!("finishing.");
}
