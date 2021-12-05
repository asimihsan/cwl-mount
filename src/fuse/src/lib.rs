/*
 * Copyright Kitten Cat LLC. All Rights Reserved.
 * SPDX-License-Identifier: Apache-2.0.
 */

 #[macro_use]
extern crate derivative;

use std::collections::BTreeMap;
use std::collections::HashMap;

use chrono::DateTime;
use chrono::Datelike;
use chrono::Duration;
use chrono::TimeZone;
use chrono::Utc;
use slotmap::new_key_type;
use slotmap::SlotMap;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Copy)]
pub struct TimeBounds {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum FileType {
    Directory,
    File(TimeBounds),
}

new_key_type! {
    pub struct FileKey;
}

#[derive(Derivative)]
#[derivative(Clone, Debug)]
pub struct File {
    pub inode: u64,
    pub name: String,
    pub file_type: FileType,

    #[derivative(Debug = "ignore")]
    pub parent: Option<FileKey>,

    /// Map name of child to FileKey. You cannot have duplicate names in a directory.
    #[derivative(Debug = "ignore")]
    pub children: BTreeMap<String, FileKey>,
}

impl File {
    pub fn new<T: Into<String>>(
        inode: u64,
        name: T,
        file_type: FileType,
        parent: Option<FileKey>,
    ) -> Self {
        Self {
            inode,
            name: name.into(),
            file_type,
            parent,
            children: BTreeMap::new(),
        }
    }

    pub fn is_root(&self) -> bool {
        self.parent.is_none()
    }
}

#[derive(Clone, Debug)]
pub struct FileWithFileKey<'a> {
    pub file: &'a File,
    pub file_key: FileKey,
}

impl<'a> Into<FileKey> for &FileWithFileKey<'a> {
    fn into(self) -> FileKey {
        self.file_key
    }
}

#[derive(Clone, Debug)]
pub struct FileTree {
    sm: SlotMap<FileKey, File>,
    root: Option<FileKey>,
    current_inode: u64,
    inode_to_file_key: HashMap<u64, FileKey>,
}

impl FileTree {
    pub fn new(expected_number_of_files: usize) -> Self {
        let mut file_tree = Self {
            sm: SlotMap::with_capacity_and_key(expected_number_of_files),
            root: None,

            // Must be 1, this is the root inode.
            current_inode: 1,

            inode_to_file_key: HashMap::with_capacity(expected_number_of_files),
        };
        let root = file_tree.create_directory("", None);
        file_tree.root = Some(root);
        file_tree
    }

    pub fn create_file<T: Into<String>>(
        &mut self,
        name: T,
        time_bounds: TimeBounds,
        parent: Option<FileKey>,
    ) -> FileKey {
        self._create_file(name, FileType::File(time_bounds), parent)
    }

    pub fn create_directory<T: Into<String>>(
        &mut self,
        name: T,
        parent: Option<FileKey>,
    ) -> FileKey {
        self._create_file(name, FileType::Directory, parent)
    }

    pub fn get_root(&self) -> Option<FileKey> {
        self.root
    }

    pub fn list_root(&self) -> Vec<FileWithFileKey> {
        self._list_directory(self.root.unwrap()).collect()
    }

    pub fn list_directory<F: Into<FileKey>>(&self, directory: F) -> Vec<FileWithFileKey> {
        self._list_directory(directory.into()).collect()
    }

    /// For the purposes of listing a directory get the parent of a file. If it is the root return itself.
    pub fn get_parent_for_ls(&self, file: FileKey) -> FileWithFileKey {
        let file = self.sm.get(file).unwrap();
        if let Some(parent_file_key) = file.parent {
            self._create_file_with_file_key(&parent_file_key)
        } else {
            self._create_file_with_file_key(&self.get_root().unwrap())
        }
    }

    pub fn get_child_for_inode<T: Into<String>>(&self, parent: u64, filename: T) -> Option<FileWithFileKey> {
        let directory = self.get_file_by_inode(parent);
        if directory.is_none() {
            return None;
        }
        let directory = directory.unwrap();
        match directory.file.children.get(&filename.into()) {
            Some(child) => Some(self._create_file_with_file_key(&child)),
            None => None,
        }
    }

    pub fn get_file_by_inode(&self, inode: u64) -> Option<FileWithFileKey> {
        self.inode_to_file_key
            .get(&inode)
            .map(|file_key| self._create_file_with_file_key(file_key))
    }

    fn _create_file_with_file_key(&self, file_key: &FileKey) -> FileWithFileKey {
        FileWithFileKey {
            file: self.sm.get(*file_key).unwrap(),
            file_key: *file_key,
        }
    }

    fn _list_directory(&self, directory: FileKey) -> Box<dyn Iterator<Item = FileWithFileKey> + '_> {
        let directory = self.sm.get(directory).unwrap();
        Box::new(
            directory
                .children
                .values()
                .into_iter()
                .map(|file_key| self._create_file_with_file_key(file_key)),
        )
    }

    fn _create_file<T: Into<String>>(
        &mut self,
        name: T,
        file_type: FileType,
        parent: Option<FileKey>,
    ) -> FileKey {
        let name: String = name.into();
        if let Some(parent_file_key) = parent {
            let parent = self.sm.get(parent_file_key).unwrap();
            if let Some(child) = parent.children.get(&name) {
                return *child;
            }
        }
        let key = self.sm.insert(File::new(
            self.current_inode,
            name.clone(),
            file_type,
            parent,
        ));
        self.inode_to_file_key.insert(self.current_inode, key);
        self.current_inode += 1;
        if let Some(parent_file_key) = parent {
            let parent = self.sm.get_mut(parent_file_key).unwrap();
            parent.children.insert(name, key);
        }
        key
    }
}

pub fn create_file_tree_for_time_range(start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> FileTree {
    let just_under_one_minute = Duration::minutes(1) - Duration::nanoseconds(1);
    let expected_number_of_files = (end_time - start_time).num_minutes() as usize;
    let mut file_tree = FileTree::new(expected_number_of_files);
    let mut year = start_time.year();
    while year <= end_time.year() {
        let year_file = file_tree.create_directory(
            year.to_string(),
            file_tree.get_root(),
        );
        for month in 1..=12 {
            let month_file = file_tree.create_directory(
                format!("{:02}", month),
                Some(year_file),
            );
            for day in 1..=31 {
                match Utc.ymd_opt(year, month, day) {
                    chrono::LocalResult::Single(date) => {
                        let day_file = file_tree.create_directory(
                            format!("{:02}", day),
                            Some(month_file),
                        );
                        for hour in 0..=23 {
                            for minute in 0..=59 {
                                let filename = format!("{:02}-{:02}", hour, minute);
                                let time_bound_start = date.and_hms(hour, minute, 0);
                                let time_bound_end = time_bound_start + just_under_one_minute;
                                let time_bounds = TimeBounds {
                                    start_time: time_bound_start,
                                    end_time: time_bound_end,
                                };
                                file_tree.create_file(filename, time_bounds, Some(day_file));
                            }
                        }
                    }
                    _ => continue,
                }
            }
        }
        year += 1;
    }
    file_tree
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use chrono::Utc;

    use crate::create_file_tree_for_time_range;

    #[test]
    fn test_create_files_for_time_range() {
        let start_time = Utc.ymd(2014, 11, 28).and_hms(12, 0, 9);
        let end_time = Utc.ymd(2019, 11, 28).and_hms(13, 13, 13);
        let actual_result = create_file_tree_for_time_range(start_time, end_time);
        let root_list = actual_result.list_root();
        println!("{:?}", root_list);
        let first_dir = root_list.first().unwrap();
        let first_dir_list = actual_result.list_directory(first_dir);
        println!("{:?}", first_dir_list);
    }
}
